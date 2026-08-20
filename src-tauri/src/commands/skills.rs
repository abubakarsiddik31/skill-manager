use super::{find_skill_by_manifest, manifest_is_manageable};
use crate::projects;
use crate::skills::{self, tools::ToolEntry, Skill};
use std::fs;
use std::path::Path;
use tauri::AppHandle;

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
