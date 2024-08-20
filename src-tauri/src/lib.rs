mod llm;
mod processor;
use llm::{anthropic_pipeline, update_file_name};
use processor::final_process;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![anthropic_pipeline, update_file_name, final_process])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}