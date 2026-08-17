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
    skills::delete_skill_dir(path).map_err(|e| e.to_string())?;
    // A project count is only a cache; clear it after a mutation rather than
    // scanning the project again in the background.
    if let Some(project) = projects::list(&app)
        .unwrap_or_default()
        .into_iter()
        .find(|p| path.starts_with(&p.path))
    {
        let _ = projects::clear_skill_count(&app, &project.path);
    }
    Ok(())
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

// ---------------------------------------------------------------------
// Skill creation
// ---------------------------------------------------------------------

/// Windows device names: a skill folder named one of these (in any case)
/// is unusable there, and skills folders travel between machines.
const WINDOWS_RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Folder-name policy for new skills: non-empty, no surrounding
/// whitespace, at most 64 chars, ASCII letters/digits plus `_`, `-` and
/// `.`, no leading dot, and no reserved name. Deliberately tight so one
/// folder is safe on every platform the app ships on.
fn validate_skill_folder_name(name: &str) -> Result<(), String> {
    if name.trim() != name || name.is_empty() {
        return Err("skill name is empty or has surrounding whitespace".into());
    }
    if name.chars().count() > 64 {
        return Err("skill name must be at most 64 characters".into());
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
    {
        return Err("skill name may only contain ASCII letters, digits, '_', '-' and '.'".into());
    }
    if name.starts_with('.') {
        return Err("skill name must not start with a dot".into());
    }
    if name == DISABLED_DIR {
        return Err(format!("'{DISABLED_DIR}' is reserved for disabled skills"));
    }
    if WINDOWS_RESERVED_NAMES.contains(&name.to_ascii_uppercase().as_str()) {
        return Err(format!("'{name}' is a reserved device name on Windows"));
    }
    Ok(())
}

/// A one-line description rendered as a YAML double-quoted scalar, so
/// colons, quotes and other YAML-significant characters survive the
/// frontmatter round trip without inventing a parser. Multi-line values
/// are emitted as YAML `\n` escapes inside the double-quoted scalar, so
/// a multi-line description round-trips exactly.
fn yaml_double_quoted(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Create `<root>/<name>/SKILL.md` with minimal frontmatter (name and
/// description only — no invented instructions) and return the manifest
/// path. `root` must be exactly one of the managed `roots`, compared
/// before anything exists on disk, so the webview cannot direct a write
/// at a folder this app does not manage.
fn create_skill_manifest(
    roots: &[PathBuf],
    root: &Path,
    name: &str,
    description: &str,
) -> Result<PathBuf, String> {
    if !roots.iter().any(|r| r == root) {
        return Err("target folder is not managed by this app".into());
    }
    validate_skill_folder_name(name)?;
    let description = description.trim();
    if description.is_empty() {
        return Err("description is empty".into());
    }
    let skill_dir = root.join(name);
    if skill_dir.exists() {
        return Err(format!(
            "a skill named '{name}' already exists in this folder"
        ));
    }
    // A managed root may legitimately not exist yet (fresh install, or a
    // project subfolder nobody has put skills in).
    fs::create_dir_all(root).map_err(|e| format!("cannot create target folder: {e}"))?;
    fs::create_dir(&skill_dir).map_err(|e| format!("cannot create skill folder: {e}"))?;
    let manifest = skill_dir.join(MANIFEST_FILE);
    let content = format!(
        "---\nname: {name}\ndescription: {}\n---\n",
        yaml_double_quoted(description)
    );
    fs::write(&manifest, content).map_err(|e| {
        // Leave no half-created folder behind on a failed write.
        let _ = fs::remove_dir(&skill_dir);
        format!("cannot write manifest: {e}")
    })?;
    Ok(manifest)
}

/// Creates `<skills-dir>/<name>/SKILL.md` for one adapter. The target
/// folder is chosen as tool + scope: user scope writes the adapter's own
/// user-level folder, project scope writes `<project>/<adapter
/// subpath>` for an already-tracked project. The manifest carries only
/// the frontmatter the user supplied — the editor opens afterwards for
/// the instructions.
#[tauri::command]
pub fn create_skill(
    app: AppHandle,
    tool: skills::AgentTool,
    scope: skills::SkillScope,
    project_path: Option<String>,
    name: String,
    description: String,
) -> Result<Skill, String> {
    let adapter = skills::adapter_for(tool);
    let tracked = projects::list(&app).unwrap_or_default();
    let root = match scope {
        skills::SkillScope::User => adapter.skills_dir(),
        skills::SkillScope::Project => {
            let Some(path) = project_path else {
                return Err("project scope requires a project".into());
            };
            if !tracked.iter().any(|p| p.path == path) {
                return Err("project is not tracked by this app".into());
            }
            PathBuf::from(path).join(adapter.project_subpath())
        }
    };
    let manifest = create_skill_manifest(&skills_roots(&tracked), &root, &name, &description)?;
    // A project count is only a cache; clear it after a mutation rather
    // than scanning the project again in the background.
    if let Some(project) = tracked.into_iter().find(|p| manifest.starts_with(&p.path)) {
        let _ = projects::clear_skill_count(&app, &project.path);
    }
    find_skill_by_manifest(&app, &manifest).ok_or_else(|| "skill not found after creation".into())
}

#[tauri::command]
pub fn list_projects(app: AppHandle) -> Result<Vec<ProjectInfo>, String> {
    projects::list(&app)
}

/// Saved project suggestions. Opening the picker must not walk the user's
/// folders again, especially protected locations such as Documents.
#[tauri::command]
pub fn list_detected_projects(
    app: AppHandle,
    exclude: Vec<String>,
) -> Result<Option<Vec<DetectedProject>>, String> {
    projects::list_detected(&app, &exclude)
}

/// Explicitly refreshes the saved suggestions. This is the only discovery
/// command that reads development folders.
#[tauri::command]
pub fn refresh_detected_projects(
    app: AppHandle,
    exclude: Vec<String>,
) -> Result<Vec<DetectedProject>, String> {
    let detected = detect::detect(&exclude);
    projects::save_detected(&app, &detected)?;
    Ok(detected)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSkillCount {
    pub path: String,
    pub count: usize,
}

/// Cached counts for tracked projects. This command never reads a project
/// directory, so opening the app cannot trigger a batch of macOS prompts.
#[tauri::command]
pub fn list_project_skill_counts(app: AppHandle) -> Vec<ProjectSkillCount> {
    projects::list(&app)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|p| {
            p.skill_count.map(|count| ProjectSkillCount {
                path: p.path,
                count,
            })
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
    let skills = skills::discover_project_skills(Path::new(&path));
    // This scan follows an explicit project open, so cache it for future
    // launches instead of re-reading every tracked project on startup.
    let _ = projects::set_skill_count(&app, &path, skills.len());
    skills
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
        .find(|p| manifest.starts_with(&p.path))
        .and_then(|p| {
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

    #[test]
    fn folder_name_validation_accepts_standard_names() {
        for name in ["my-skill", "pdf_export", "skill.v2", "a", "Skill-Name_1.0"] {
            assert!(
                validate_skill_folder_name(name).is_ok(),
                "{name} should be valid"
            );
        }
    }

    #[test]
    fn folder_name_validation_rejects_unsafe_names() {
        // empty / whitespace-wrapped
        for name in ["", "  ", " padded", "padded ", "\ttab"] {
            assert!(
                validate_skill_folder_name(name).is_err(),
                "{name:?} should be invalid"
            );
        }
        // path traversal and separators
        for name in [".", "..", "a/b", "a\\b", "sub/dir/skill", "a:b"] {
            assert!(
                validate_skill_folder_name(name).is_err(),
                "{name:?} should be invalid"
            );
        }
        // reserved / hidden
        for name in [DISABLED_DIR, ".hidden", "con", "NUL", "LPT1", "com3"] {
            assert!(
                validate_skill_folder_name(name).is_err(),
                "{name:?} should be invalid"
            );
        }
        // non-ASCII, control characters, too long
        let too_long = "a".repeat(65);
        for name in ["héllo", "skill\nname", "a\tb", too_long.as_str()] {
            assert!(
                validate_skill_folder_name(name).is_err(),
                "{name:?} should be invalid"
            );
        }
    }

    fn fresh_roots(tag: &str) -> (PathBuf, PathBuf) {
        let base =
            std::env::temp_dir().join(format!("skill-manager-create-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        (base.join("managed"), base.join("unmanaged"))
    }

    #[test]
    fn create_rejects_roots_the_app_does_not_manage() {
        let (managed, unmanaged) = fresh_roots("root-guard");
        let roots = vec![managed.clone()];

        let err =
            create_skill_manifest(&roots, &unmanaged, "my-skill", "does something").unwrap_err();
        assert!(err.contains("not managed"), "unexpected error: {err}");
        assert!(!unmanaged.exists());

        // lookalike prefixes must not slide through either
        let lookalike = managed.with_file_name(format!(
            "{}-suffix",
            managed.file_name().unwrap().to_string_lossy()
        ));
        let err =
            create_skill_manifest(&roots, &lookalike, "my-skill", "does something").unwrap_err();
        assert!(err.contains("not managed"), "unexpected error: {err}");

        let _ = fs::remove_dir_all(managed.parent().unwrap());
    }

    #[test]
    fn create_writes_minimal_manifest_and_opens_missing_root() {
        let (managed, _unmanaged) = fresh_roots("happy-path");
        let roots = vec![managed.clone()];

        // the root does not exist yet — a fresh install must still be able
        // to create its first skill
        let manifest =
            create_skill_manifest(&roots, &managed, "my-skill", "Trims PDFs: fast & safe")
                .expect("creation should succeed");
        assert!(manifest.is_file());
        let content = fs::read_to_string(&manifest).unwrap();
        assert_eq!(
            content,
            "---\nname: my-skill\ndescription: \"Trims PDFs: fast & safe\"\n---\n"
        );
        // exactly one SKILL.md inside one new folder — nothing else
        let entries: Vec<_> = fs::read_dir(&managed).unwrap().collect();
        assert_eq!(entries.len(), 1);

        let _ = fs::remove_dir_all(managed.parent().unwrap());
    }

    #[test]
    fn create_reports_collisions_without_touching_existing_skill() {
        let (managed, _unmanaged) = fresh_roots("collision");
        let roots = vec![managed.clone()];
        fs::create_dir_all(managed.join("existing")).unwrap();
        fs::write(managed.join("existing").join(MANIFEST_FILE), "original").unwrap();

        let err = create_skill_manifest(&roots, &managed, "existing", "a duplicate").unwrap_err();
        assert!(err.contains("already exists"), "unexpected error: {err}");
        // the existing skill is untouched
        assert_eq!(
            fs::read_to_string(managed.join("existing").join(MANIFEST_FILE)).unwrap(),
            "original"
        );

        let _ = fs::remove_dir_all(managed.parent().unwrap());
    }

    #[test]
    fn create_validates_name_and_description_before_writing() {
        let (managed, _unmanaged) = fresh_roots("validation");
        let roots = vec![managed.clone()];

        assert!(create_skill_manifest(&roots, &managed, "../escape", "d").is_err());
        assert!(create_skill_manifest(&roots, &managed, ".disabled", "d").is_err());
        assert!(create_skill_manifest(&roots, &managed, "ok-name", "").is_err());
        assert!(create_skill_manifest(&roots, &managed, "ok-name", "   ").is_err());
        // nothing was created by any rejected attempt
        assert!(!managed.exists());

        let _ = fs::remove_dir_all(managed.parent().unwrap());
    }

    #[test]
    fn multi_line_descriptions_round_trip_through_the_frontmatter() {
        let (managed, _unmanaged) = fresh_roots("multiline");
        let roots = vec![managed.clone()];

        let description = "what it does\nand when to use it";
        let manifest =
            create_skill_manifest(&roots, &managed, "my-skill", description).expect("creation");
        let content = fs::read_to_string(&manifest).unwrap();
        // YAML double-quoted scalar carries the newline as an escape, so the
        // line stays single-line YAML while parsing yields the original text.
        assert_eq!(
            content,
            "---\nname: my-skill\ndescription: \"what it does\\nand when to use it\"\n---\n"
        );

        let _ = fs::remove_dir_all(managed.parent().unwrap());
    }
}
