use crate::detect::{self, DetectedProject};
use crate::projects::{self, ProjectInfo};
use crate::skills::{self, tools::ToolEntry, Skill};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

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
/// commands only operate inside the managed skills roots above.
fn is_managed_skill_path(app: &AppHandle, path: &Path) -> bool {
    let tracked = projects::list(app).unwrap_or_default();
    skills_roots(&tracked)
        .iter()
        .any(|root| path.starts_with(root))
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
    if !is_managed_skill_path(&app, path) {
        return Err("not a managed skill path".into());
    }
    let new_manifest = skills::toggle_enabled(path, enabled).map_err(|e| e.to_string())?;

    find_skill_by_manifest(&app, &new_manifest).ok_or_else(|| "skill not found after toggle".into())
}

#[tauri::command]
pub fn delete_skill(app: AppHandle, id: String) -> Result<(), String> {
    let path = Path::new(&id);
    if !is_managed_skill_path(&app, path) {
        return Err("not a managed skill path".into());
    }
    skills::delete_skill_dir(path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn read_skill_content(app: AppHandle, id: String) -> Result<String, String> {
    if !is_managed_skill_path(&app, Path::new(&id)) {
        return Err("not a managed skill path".into());
    }
    fs::read_to_string(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_skill_content(app: AppHandle, id: String, content: String) -> Result<(), String> {
    let path = Path::new(&id);
    if !is_managed_skill_path(&app, path) {
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
}
