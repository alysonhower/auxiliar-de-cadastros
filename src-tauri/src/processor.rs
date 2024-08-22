use std::path::Path;

use crate::llm::models::DocumentInfo;
use regex::Regex;
use tauri::async_runtime;
use tauri_plugin_shell::{process::CommandEvent, ShellExt};

#[tauri::command]
pub async fn final_pipeline(
    handle: tauri::AppHandle,
    document_info: DocumentInfo,
) -> Result<(), String> {
    let parent_dir = Path::new(&document_info.json_file_path).parent().expect("Failed to get parent directory");
    let re = Regex::new(r"(.+)-data$").unwrap();
    let original_file = re.replace(&parent_dir.to_string_lossy(), "$1").to_string();
    let original_file = Path::new(&original_file).with_extension("pdf");
    let done_dir = parent_dir.join("done");

    if !done_dir.exists() {
        std::fs::create_dir_all(&done_dir).map_err(|_| "Failed to create done directory")?;
    }

    let save_path = done_dir.join(document_info.file_name).with_extension("pdf");

    if save_path.exists() {
        std::fs::remove_file(&save_path).map_err(|_| "Failed to delete file")?;
    }

    let mut pages = vec![];

    for page in document_info.pages_paths {
        let page_number = extract_page_number(&page);
        pages.push(page_number.to_string());
    }

    let success = call_utility(
        handle.clone(),
        "qpdf".to_owned(),
        vec![
            "--empty".to_string(),
            "--pages".to_string(),
            original_file.to_string_lossy().to_string(),
            pages.join(","),
            "--".to_string(),
            save_path.to_string_lossy().to_string(),
        ],
    )
    .await;

    if success {
        let success = call_utility(
            handle.clone(),
            "ocrmypdf.exe".to_owned(),
            vec![
                "--force-ocr".to_string(),
                "--pdf-renderer".to_string(),
                "hocr".to_string(),
                "--color-conversion-strategy".to_string(),
                "UseDeviceIndependentColor".to_string(),
                "-l".to_string(),
                "por".to_string(),
                "--clean".to_string(),
                "--output-type".to_string(),
                "pdfa-2".to_string(),
                save_path.to_string_lossy().to_string(),
                save_path.to_string_lossy().to_string(),
            ],
        ).await;
        if !success {
            return Err("Failed to call utility".to_string());
        }
    }

    Ok(())
}

async fn call_utility(handle: tauri::AppHandle, utility: String, args: Vec<String>) -> bool {
    let spawn_utility = async_runtime::spawn(async move {
        let (mut rx, child) = handle
            .shell()
            .command(utility)
            .args(args)
            .spawn()
            .expect("Failed to spawn process");

        let mut is_success = false;

        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(data) => {
                    println!("{}", String::from_utf8_lossy(&data));
                }
                CommandEvent::Stderr(data) => {
                    println!("{}", String::from_utf8_lossy(&data));
                }
                CommandEvent::Terminated(status) => {
                    if let Some(code) = status.code {
                        println!("Process terminated with status: {}", code);
                        if code == 0 {
                            is_success = true;
                        }
                    }
                    if let Some(signal) = status.signal {
                        println!("Process terminated with signal: {}", signal);
                    }
                }
                _ => {}
            }
        }
        child.kill().expect("Failed to kill process");
        is_success
    });

    spawn_utility.await.unwrap()
}

fn extract_page_number(input: &str) -> &str {
    let re = Regex::new(r"page-(\d+)").expect("Regex should never fail");
    re.captures(input)
        .and_then(|caps| caps.get(1).map(|m| m.as_str()))
        .unwrap_or("unidentified")
}

#[tauri::command]
pub fn open_in_explorer(path: &str) -> Result<(), String> {
    let mut command = std::process::Command::new("explorer");
    command.args(&["/select,", path]);
    command.spawn().map_err(|_| "Failed to open in explorer")?;
    Ok(())
}