use crate::llm::models::DocumentInfo;
// use serde_json::Value;
// #[tauri::command]
// pub fn process_pages(document_info: DocumentInfo) -> Result<(), ()> {
//     println!("process_pages: {:?}", document_info);
//     Ok(())
// }

#[tauri::command]
pub fn final_process(document_info: DocumentInfo) -> Result<(), ()> {
    println!("final_process: {:?}", document_info);
    Ok(())
}