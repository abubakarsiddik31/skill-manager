use crate::detect::{self, DetectedProject};
use crate::projects::{self, ProjectInfo};
use crate::skills::{self, tools::ToolEntry, Skill};
use serde::Serialize;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

const MANIFEST_FILE: &str = "SKILL.md";
const DISABLED_DIR: &str = ".disabled";

/// Every skills directory the app manages: each adapter's user-level
/// folder, plus every tracked project's per-adapter subfolder. The
/// `.disabled` toggle target lives inside these roots already.
fn skills_roots(tracked: &[ProjectInfo]) -> Vec<PathBuf> {
    let adapters = skills::all_adapters();
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
fn validate_manifest_at(path: &Path, roots: &[PathBuf]) -> bool {
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
    let tracked = projects::list(app).unwrap_or_default();
    validate_manifest_at(path, &skills_roots(&tracked))
}

/// Tool-level registry entries (see `skills::tools`): one per coding
/// agent, listing every skills folder it reads.
#[tauri::command]
pub fn list_tool_entries() -> Vec<ToolEntry> {
    skills::tools::tool_entries()
}

#[tauri::command]
pub fn list_skills() -> Vec<Skill> {
    skills::all_adapters()
        .into_iter()
        .flat_map(|adapter| adapter.discover())
        .collect()
}

#[tauri::command]
pub fn set_skill_enabled(app: AppHandle, id: String, enabled: bool) -> Result<Skill, String> {
    let path = Path::new(&id);
    if !manifest_is_manageable(&app, path) {
        return Err("not a managed skill path".into());
    }
    let new_manifest = skills::toggle_enabled(path, enabled).map_err(|e| e.to_string())?;

    find_skill_by_manifest(&app, &new_manifest).ok_or_else(|| "skill not found after toggle".into())
}

#[tauri::command]
pub fn delete_skill(app: AppHandle, id: String) -> Result<(), String> {
    let path = Path::new(&id);
    if !manifest_is_manageable(&app, path) {
        return Err("not a managed skill path".into());
    }
    skills::delete_skill_dir(path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn read_skill_content(app: AppHandle, id: String) -> Result<String, String> {
    if !manifest_is_manageable(&app, Path::new(&id)) {
        return Err("not a managed skill path".into());
    }
    fs::read_to_string(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_skill_content(app: AppHandle, id: String, content: String) -> Result<(), String> {
    let path = Path::new(&id);
    if !manifest_is_manageable(&app, path) {
        return Err("not a managed skill path".into());
    }
    fs::write(path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_projects(app: AppHandle) -> Result<Vec<ProjectInfo>, String> {
    projects::list(&app)
}

/// Suggests project folders the user seems to work in, most recently
/// active first. Already-tracked paths are passed in `exclude` so the
/// picker never re-offers them.
#[tauri::command]
pub fn detect_projects(exclude: Vec<String>) -> Vec<DetectedProject> {
    detect::detect(&exclude)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSkillCount {
    pub path: String,
    pub count: usize,
}

/// Skill counts for every tracked project, so the sidebar can badge
/// rows without the frontend issuing one scan per project.
#[tauri::command]
pub fn list_project_skill_counts(app: AppHandle) -> Vec<ProjectSkillCount> {
    projects::list(&app)
        .unwrap_or_default()
        .into_iter()
        .map(|p| ProjectSkillCount {
            count: skills::discover_project_skills(Path::new(&p.path)).len(),
            path: p.path,
        })
        .collect()
}

#[tauri::command]
pub fn add_project(app: AppHandle, path: String) -> Result<ProjectInfo, String> {
    projects::add(&app, path)
}

#[tauri::command]
pub fn remove_project(app: AppHandle, path: String) -> Result<(), String> {
    projects::remove(&app, &path)
}

#[tauri::command]
pub fn set_project_pinned(app: AppHandle, path: String, pinned: bool) -> Result<(), String> {
    projects::set_pinned(&app, &path, pinned)
}

#[tauri::command]
pub fn touch_project(app: AppHandle, path: String) -> Result<(), String> {
    projects::touch(&app, &path)
}

#[tauri::command]
pub fn list_project_skills(app: AppHandle, path: String) -> Vec<Skill> {
    // only tracked projects get a breakdown — anything else is ignored
    let tracked = projects::list(&app)
        .unwrap_or_default()
        .into_iter()
        .any(|p| p.path == path);
    if !tracked {
        return Vec::new();
    }
    skills::discover_project_skills(Path::new(&path))
}

fn find_skill_by_manifest(app: &AppHandle, manifest: &Path) -> Option<Skill> {
    let target = manifest.to_string_lossy().to_string();

    let in_user_scope = skills::all_adapters()
        .into_iter()
        .flat_map(|adapter| adapter.discover())
        .find(|s| s.id == target);
    if in_user_scope.is_some() {
        return in_user_scope;
    }

    projects::list(app)
        .unwrap_or_default()
        .into_iter()
        .find_map(|p| {
            skills::discover_project_skills(Path::new(&p.path))
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
