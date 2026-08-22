use crate::projects::ProjectInfo;
use crate::skills::Skill;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

mod create_skill;
mod projects;
mod skills;

// The `#[tauri::command]` macros generate hidden `__cmd__X` and
// `__tauri_command_name_X` companion macros beside each fn;
// `generate_handler![commands::X]` in `lib.rs` looks them up under the
// `commands::` path, so they are re-exported together with the fns.
pub(crate) use create_skill::validate_skill_folder_name;
pub use create_skill::{__cmd__create_skill, __tauri_command_name_create_skill, create_skill};
pub use projects::{
    __cmd__add_project, __cmd__list_detected_projects, __cmd__list_project_skill_counts,
    __cmd__list_project_skills, __cmd__list_projects, __cmd__refresh_detected_projects,
    __cmd__remove_project, __cmd__set_project_pinned, __cmd__touch_project,
    __tauri_command_name_add_project, __tauri_command_name_list_detected_projects,
    __tauri_command_name_list_project_skill_counts, __tauri_command_name_list_project_skills,
    __tauri_command_name_list_projects, __tauri_command_name_refresh_detected_projects,
    __tauri_command_name_remove_project, __tauri_command_name_set_project_pinned,
    __tauri_command_name_touch_project, add_project, list_detected_projects,
    list_project_skill_counts, list_project_skills, list_projects, refresh_detected_projects,
    remove_project, set_project_pinned, touch_project,
};
pub use skills::{
    __cmd__delete_skill, __cmd__list_skills, __cmd__list_tool_entries, __cmd__read_skill_content,
    __cmd__set_skill_enabled, __cmd__write_skill_content, __tauri_command_name_delete_skill,
    __tauri_command_name_list_skills, __tauri_command_name_list_tool_entries,
    __tauri_command_name_read_skill_content, __tauri_command_name_set_skill_enabled,
    __tauri_command_name_write_skill_content, delete_skill, list_skills, list_tool_entries,
    read_skill_content, set_skill_enabled, write_skill_content,
};

const MANIFEST_FILE: &str = "SKILL.md";
const DISABLED_DIR: &str = ".disabled";

/// Every skills directory the app manages: each adapter's user-level
/// folder, plus every tracked project's per-adapter subfolder. The
/// `.disabled` toggle target lives inside these roots already.
fn skills_roots(tracked: &[ProjectInfo]) -> Vec<PathBuf> {
    let adapters = crate::skills::all_adapters();
    let mut roots: Vec<PathBuf> = adapters.iter().map(|a| a.skills_dir()).collect();
    for project in tracked {
        let root = PathBuf::from(&project.path);
        for adapter in &adapters {
            roots.push(root.join(adapter.project_subpath()));
        }
    }
    roots
}

/// The webview is untrusted input like any other frontend: file-taking
/// commands only operate on manifests the scanner itself could have
/// produced. Two bars, both required:
///
/// - *shape*: the id must look like scanner output —
///   `<root>/<skill>/SKILL.md` or `<root>/.disabled/<skill>/SKILL.md` —
///   never the root itself. A crafted `<root>/SKILL.md` would otherwise
///   pass a prefix check and delete or relocate an entire skills
///   directory in one call.
/// - *resolution*: the id must exist and, with symlinks followed, land
///   inside a managed root. A link planted inside a skills folder must
///   not turn "save skill" into a write outside the folders we manage.
///   Sharing a skill across tools via a link stays allowed because every
///   tool's folder is a root, so a Cursor link into `~/.claude/skills`
///   still resolves home.
pub(crate) fn validate_manifest_at(path: &Path, roots: &[PathBuf]) -> bool {
    if path.file_name() != Some(OsStr::new(MANIFEST_FILE)) {
        return false;
    }
    let Some(skill_dir) = path.parent() else {
        return false;
    };
    // `<root>/SKILL.md` and `<root>/.disabled/SKILL.md` name no skill
    if skill_dir.file_name().is_none_or(|n| n == DISABLED_DIR) {
        return false;
    }
    if !roots.iter().any(|root| skill_dir.starts_with(root)) {
        return false;
    }
    let Ok(resolved) = fs::canonicalize(path) else {
        return false;
    };
    let resolved_roots: Vec<PathBuf> = roots
        .iter()
        .filter_map(|r| fs::canonicalize(r).ok())
        .collect();
    resolved_roots.iter().any(|root| resolved.starts_with(root))
}

fn manifest_is_manageable(app: &AppHandle, path: &Path) -> bool {
    let tracked = crate::projects::list(app).unwrap_or_default();
    validate_manifest_at(path, &skills_roots(&tracked))
}

fn find_skill_by_manifest(app: &AppHandle, manifest: &Path) -> Option<Skill> {
    let target = manifest.to_string_lossy().to_string();

    let in_user_scope = crate::skills::all_adapters()
        .into_iter()
        .flat_map(|adapter| adapter.discover())
        .find(|s| s.id == target);
    if in_user_scope.is_some() {
        return in_user_scope;
    }

    crate::projects::list(app)
        .unwrap_or_default()
        .into_iter()
        .find(|p| manifest.starts_with(&p.path))
        .and_then(|p| {
            crate::skills::discover_project_skills(Path::new(&p.path))
                .into_iter()
                .find(|s| s.id == target)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo_project(path: &str) -> ProjectInfo {
        ProjectInfo {
            path: path.into(),
            name: "demo".into(),
            pinned: false,
            last_opened: 0,
            opens: Vec::new(),
            skill_count: None,
        }
    }

    #[test]
    fn skills_roots_cover_user_dirs_and_project_subpaths() {
        let roots = skills_roots(&[demo_project("/tmp/demo")]);

        // every tracked project subfolder is a root
        assert!(roots.contains(&PathBuf::from("/tmp/demo/.claude/skills")));
        assert!(roots.contains(&PathBuf::from("/tmp/demo/.agents/skills")));
        // user-level folders are roots too (paths depend on $HOME)
        assert!(roots
            .iter()
            .any(|r| r.ends_with(".claude/skills") && !r.starts_with("/tmp")));
    }

    #[test]
    fn lookalike_paths_are_not_roots() {
        let roots = skills_roots(&[demo_project("/tmp/demo")]);
        // a sibling directory sharing a prefix must not match
        assert!(!roots.iter().any(|r| r.ends_with(".claude/skills-not")));
        // component-wise starts_with semantics are enforced by Path, so
        // /tmp/demo/.claude/skills2 does not start with the skills root
        let root = PathBuf::from("/tmp/demo/.claude/skills");
        assert!(!Path::new("/tmp/demo/.claude/skills2").starts_with(&root));
        assert!(Path::new("/tmp/demo/.claude/skills/.disabled/x").starts_with(&root));
    }

    fn temp_root(tag: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("skill-manager-cmd-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("demo")).unwrap();
        fs::write(root.join("demo").join(MANIFEST_FILE), "x").unwrap();
        root
    }

    #[test]
    fn accepts_manifests_in_scanner_shapes_only() {
        let root = temp_root("shape");
        let roots = vec![root.clone()];

        assert!(validate_manifest_at(&root.join("demo/SKILL.md"), &roots));
        fs::create_dir_all(root.join(DISABLED_DIR).join("demo")).unwrap();
        fs::write(
            root.join(DISABLED_DIR).join("demo").join(MANIFEST_FILE),
            "x",
        )
        .unwrap();
        assert!(validate_manifest_at(
            &root.join(DISABLED_DIR).join("demo").join(MANIFEST_FILE),
            &roots
        ));

        // the id must name a skill folder, never the root or .disabled itself
        assert!(!validate_manifest_at(&root.join(MANIFEST_FILE), &roots));
        fs::write(root.join(DISABLED_DIR).join(MANIFEST_FILE), "x").unwrap();
        assert!(!validate_manifest_at(
            &root.join(DISABLED_DIR).join(MANIFEST_FILE),
            &roots
        ));
        // and it must be a SKILL.md, not some other file under the root
        fs::write(root.join("demo").join("notes.txt"), "x").unwrap();
        assert!(!validate_manifest_at(
            &root.join("demo").join("notes.txt"),
            &roots
        ));
        // missing manifests are rejected too
        assert!(!validate_manifest_at(&root.join("ghost/SKILL.md"), &roots));

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn symlinked_skills_must_resolve_into_a_root() {
        let root = temp_root("symlink-ok");
        let other = temp_root("symlink-other");
        let roots = vec![root.clone(), other.clone()];

        // a link from one managed root into another is the legit
        // cross-tool sharing setup — allowed
        std::os::unix::fs::symlink(other.join("demo"), root.join("shared")).unwrap();
        assert!(validate_manifest_at(
            &root.join("shared").join(MANIFEST_FILE),
            &roots
        ));

        // a link pointing outside every root would let a file operation
        // escape the folders we manage — rejected
        let outside =
            std::env::temp_dir().join(format!("skill-manager-outside-{}", std::process::id()));
        let _ = fs::remove_dir_all(&outside);
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join(MANIFEST_FILE), "x").unwrap();
        std::os::unix::fs::symlink(&outside, root.join("escape")).unwrap();
        assert!(!validate_manifest_at(
            &root.join("escape").join(MANIFEST_FILE),
            &roots
        ));

        fs::remove_dir_all(&root).unwrap();
        fs::remove_dir_all(&other).unwrap();
        fs::remove_dir_all(&outside).unwrap();
    }
}
