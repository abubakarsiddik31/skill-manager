use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
    pub path: String,
    pub name: String,
}

const STORE_FILE: &str = "projects.json";

fn store_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join(STORE_FILE))
}

fn load(app: &AppHandle) -> Result<Vec<ProjectInfo>, String> {
    let path = store_path(app)?;
    let Ok(raw) = fs::read_to_string(&path) else {
        return Ok(Vec::new());
    };
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

fn save(app: &AppHandle, projects: &[ProjectInfo]) -> Result<(), String> {
    let path = store_path(app)?;
    let raw = serde_json::to_string_pretty(projects).map_err(|e| e.to_string())?;
    fs::write(path, raw).map_err(|e| e.to_string())
}

pub fn list(app: &AppHandle) -> Result<Vec<ProjectInfo>, String> {
    load(app)
}

pub fn add(app: &AppHandle, path: String) -> Result<ProjectInfo, String> {
    let root = Path::new(&path);
    if !root.is_dir() {
        return Err(format!("{} is not a directory", path));
    }
    let mut projects = load(app)?;
    if let Some(existing) = projects.iter().find(|p| p.path == path) {
        return Ok(existing.clone());
    }
    let name = root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&path)
        .to_string();
    let info = ProjectInfo { path, name };
    projects.push(info.clone());
    save(app, &projects)?;
    Ok(info)
}

pub fn remove(app: &AppHandle, path: &str) -> Result<(), String> {
    let mut projects = load(app)?;
    projects.retain(|p| p.path != path);
    save(app, &projects)
}
