use crate::detect::DetectedProject;
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
    /// Recent opens (unix seconds), pruned to the last 30 days so the
    /// sidebar can surface the most-used projects. Older files that
    /// predate usage tracking start empty.
    #[serde(default)]
    pub opens: Vec<u64>,
    /// Last known number of skills. Counts are refreshed only after the user
    /// explicitly opens a project, never by scanning every tracked folder at
    /// startup.
    #[serde(default)]
    pub skill_count: Option<usize>,
}

const STORE_FILE: &str = "projects.json";
const DISCOVERY_STORE_FILE: &str = "detected-projects.json";
const USAGE_WINDOW_SECS: u64 = 30 * 24 * 60 * 60;

fn store_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
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

fn discovery_store_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join(DISCOVERY_STORE_FILE))
}

/// Returns `None` until the user has explicitly asked to find projects.
/// Keeping this separate from tracked projects makes the consent boundary
/// durable without treating suggestions as projects the app may inspect.
pub fn list_detected(
    app: &AppHandle,
    exclude: &[String],
) -> Result<Option<Vec<DetectedProject>>, String> {
    let path = discovery_store_path(app)?;
    let Ok(raw) = fs::read_to_string(path) else {
        return Ok(None);
    };
    let projects: Vec<DetectedProject> = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    Ok(Some(
        projects
            .into_iter()
            .filter(|project| !exclude.contains(&project.path))
            .collect(),
    ))
}

pub fn save_detected(app: &AppHandle, detected: &[DetectedProject]) -> Result<(), String> {
    let path = discovery_store_path(app)?;
    let raw = serde_json::to_string_pretty(detected).map_err(|e| e.to_string())?;
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
        opens: Vec::new(),
        skill_count: None,
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

/// Records an open so the sidebar can order projects latest-first and
/// surface the most-used ones.
pub fn touch(app: &AppHandle, path: &str) -> Result<(), String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let cutoff = now.saturating_sub(USAGE_WINDOW_SECS);
    let mut projects = load(app)?;
    for project in projects.iter_mut() {
        if project.path == path {
            project.last_opened = now;
            project.opens.retain(|t| *t >= cutoff);
            project.opens.push(now);
            break;
        }
    }
    save(app, &projects)
}

/// Stores a count already obtained during an explicit project action. This
/// keeps the sidebar useful without reading every project folder at startup.
pub fn set_skill_count(app: &AppHandle, path: &str, count: usize) -> Result<(), String> {
    let mut projects = load(app)?;
    for project in projects.iter_mut() {
        if project.path == path {
            project.skill_count = Some(count);
            break;
        }
    }
    save(app, &projects)
}

pub fn clear_skill_count(app: &AppHandle, path: &str) -> Result<(), String> {
    let mut projects = load(app)?;
    for project in projects.iter_mut() {
        if project.path == path {
            project.skill_count = None;
            break;
        }
    }
    save(app, &projects)
}

#[cfg(test)]
mod tests {
    use super::ProjectInfo;

    #[test]
    fn older_project_records_start_with_an_unknown_skill_count() {
        let project: ProjectInfo = serde_json::from_str(
            r#"{
                "path": "/tmp/demo",
                "name": "demo",
                "pinned": false,
                "lastOpened": 0,
                "opens": []
            }"#,
        )
        .unwrap();

        assert_eq!(project.skill_count, None);
    }
}
