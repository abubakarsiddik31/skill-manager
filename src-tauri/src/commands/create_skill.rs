use super::{find_skill_by_manifest, skills_roots, DISABLED_DIR, MANIFEST_FILE};
use crate::projects;
use crate::skills::{self, Skill};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

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
pub(crate) fn validate_skill_folder_name(name: &str) -> Result<(), String> {
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

#[cfg(test)]
mod tests {
    use super::*;

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
