mod commands;
mod detect;
mod projects;
mod skills;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::list_tool_entries,
            commands::list_skills,
            commands::set_skill_enabled,
            commands::delete_skill,
            commands::read_skill_content,
            commands::write_skill_content,
            commands::list_detected_projects,
            commands::refresh_detected_projects,
            commands::list_project_skill_counts,
            commands::list_projects,
            commands::add_project,
            commands::remove_project,
            commands::set_project_pinned,
            commands::touch_project,
            commands::list_project_skills,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
