use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
    pub path: String,
    pub name: String,
    /// Older projects.json files predate pinning — treat as unpinned.
    #[serde(default)]
    pub pinned: bool,
    /// Unix seconds of the last open in this app, for latest-first order.
    #[serde(default)]
    pub last_opened: u64,
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
    let info = ProjectInfo {
        path,
        name,
        pinned: false,
        last_opened: 0,
    };
    projects.push(info.clone());
    save(app, &projects)?;
    Ok(info)
}

pub fn remove(app: &AppHandle, path: &str) -> Result<(), String> {
    let mut projects = load(app)?;
    projects.retain(|p| p.path != path);
    save(app, &projects)
}

pub fn set_pinned(app: &AppHandle, path: &str, pinned: bool) -> Result<(), String> {
    let mut projects = load(app)?;
    for project in projects.iter_mut() {
        if project.path == path {
            project.pinned = pinned;
            break;
        }
    }
    save(app, &projects)
}

/// Records an open so the sidebar can order projects latest-first.
pub fn touch(app: &AppHandle, path: &str) -> Result<(), String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut projects = load(app)?;
    for project in projects.iter_mut() {
        if project.path == path {
            project.last_opened = now;
            break;
        }
    }
    save(app, &projects)
}
