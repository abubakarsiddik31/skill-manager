use crate::detect::{self, DetectedProject};
use crate::projects::{self, ProjectInfo};
use crate::skills::{self, Skill};
use serde::Serialize;
use std::path::Path;
use tauri::AppHandle;

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
