use crate::skills::{self, Skill, ToolInfo};
use std::fs;
use std::path::Path;

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
pub fn set_skill_enabled(id: String, enabled: bool) -> Result<Skill, String> {
    let path = Path::new(&id);
    let new_manifest = skills::toggle_enabled(path, enabled).map_err(|e| e.to_string())?;

    find_skill_by_manifest(&new_manifest).ok_or_else(|| "skill not found after toggle".into())
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

fn find_skill_by_manifest(manifest: &Path) -> Option<Skill> {
    let target = manifest.to_string_lossy().to_string();
    skills::all_adapters()
        .into_iter()
        .flat_map(|adapter| adapter.discover())
        .find(|s| s.id == target)
}
