use crate::projects::{self, ProjectInfo};
use crate::skills::{self, Skill, ToolInfo};
use std::fs;
use std::path::Path;
use tauri::AppHandle;

#[tauri::command]
pub fn list_tools() -> Vec<ToolInfo> {
    skills::all_adapters()
        .into_iter()
        .map(|adapter| {
            let dir = adapter.skills_dir();
            ToolInfo {
                tool: adapter.tool(),
                label: adapter.tool().label().to_string(),
                skills_dir: dir.to_string_lossy().to_string(),
                dir_exists: dir.is_dir(),
            }
        })
        .collect()
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
    let new_manifest = skills::toggle_enabled(path, enabled).map_err(|e| e.to_string())?;

    find_skill_by_manifest(&app, &new_manifest).ok_or_else(|| "skill not found after toggle".into())
}

#[tauri::command]
pub fn delete_skill(id: String) -> Result<(), String> {
    let path = Path::new(&id);
    skills::delete_skill_dir(path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn read_skill_content(id: String) -> Result<String, String> {
    fs::read_to_string(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_skill_content(id: String, content: String) -> Result<(), String> {
    fs::write(&id, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_projects(app: AppHandle) -> Result<Vec<ProjectInfo>, String> {
    projects::list(&app)
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
pub fn list_project_skills(path: String) -> Vec<Skill> {
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

    projects::list(app).unwrap_or_default().into_iter().find_map(|p| {
        skills::discover_project_skills(Path::new(&p.path))
            .into_iter()
            .find(|s| s.id == target)
    })
}
