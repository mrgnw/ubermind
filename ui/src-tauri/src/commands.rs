use crate::services;
use crate::tmux;

#[tauri::command]
pub fn get_services() -> Vec<services::ServiceInfo> {
    services::list_services()
}

#[tauri::command]
pub fn get_service_detail(name: String) -> Result<services::ServiceDetail, String> {
    services::get_service_detail(&name)
}

#[tauri::command]
pub fn start_service(name: String) -> Result<String, String> {
    services::start_service(&name)
}

#[tauri::command]
pub fn stop_service(name: String) -> Result<String, String> {
    services::stop_service(&name)
}

#[tauri::command]
pub fn reload_service(name: String) -> Result<String, String> {
    services::reload_service(&name)
}

#[tauri::command]
pub fn echo_service(name: String) -> Result<String, String> {
    tmux::capture_all_panes(&name)
}

#[tauri::command]
pub fn get_panes(name: String) -> Vec<tmux::TmuxPane> {
    tmux::list_panes(&name)
}

#[tauri::command]
pub fn capture_pane(name: String, window: u32, pane: u32) -> Result<String, String> {
    tmux::capture_pane(&name, window, pane)
}
