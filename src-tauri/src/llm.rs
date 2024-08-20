use base64::prelude::*;
use dotenv::dotenv;
use quick_xml::{de::from_str, Reader, Writer};
use regex::Regex;
use serde_json::json;
use std::fs;
use std::io::Cursor;
use std::{
    fs::{create_dir_all, File},
    io::{Read, Write},
    path::Path,
};
use tauri_plugin_http::reqwest;
use tokio::time::{sleep, Duration};

pub mod models;
use models::*;

mod prompts;
use prompts::*;

#[tauri::command]
pub async fn anthropic_pipeline(paths: Vec<String>) -> Result<DocumentInfo, String> {
    dotenv().ok();
    let client = reqwest::Client::new();
    let api_key = std::env::var("ANTHROPIC_API_KEY").map_err(|e| e.to_string())?;
    let page_numbers: Vec<String> = paths
        .iter()
        .map(|path| extract_page_number(path).to_string())
        .collect();

    let file_name = format!("document_page_{}", page_numbers.join("_"));
    let first_path = Path::new(&paths[0]);
    let parent_dir = first_path
        .parent()
        .ok_or("Unable to get parent directory")?;
    let xml_path = parent_dir.join(format!("{}.xml", &file_name));
    let json_path = parent_dir.join(format!("{}.json", &file_name));

    if json_path.exists() && xml_path.exists() {
        return read_json_file(&json_path);
    }

    let xml_content = if xml_path.exists() {
        read_existing_file(&xml_path)?
    } else {
        let vec_strings = process_images(&client, &api_key, &paths).await?;
        let combined_xml = vec_strings.join("\n");
        let formatted_xml = format_xml(&combined_xml)?;
        save_xml_file(&formatted_xml, &xml_path)?;
        formatted_xml
    };

    let prompt = FILE_NAME_GENERATION_PROMPT.replace("{XML}", &xml_content);
    let response = process_xml(&client, &api_key, &prompt).await?;
    let json_path_str = json_path.to_str().unwrap().to_string();
    let wrapped_xml = format!(
        "<document><json_file_path>{}</json_file_path><pages_paths>{}</pages_paths>{}</document>",
        json_path_str, paths.join(","), response
    );

    let json_content = xml_to_json(&wrapped_xml)?;
    let mut document_info: DocumentInfo =
        serde_json::from_str(&json_content).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    document_info.json_file_path = json_path.to_str().unwrap().to_string();

    let serialized_json = serde_json::to_string(&document_info)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;

    save_json_file(&serialized_json, &json_path)?;

    Ok(document_info)
}

fn extract_page_number(input: &str) -> &str {
    let re = Regex::new(r"page-(\d+)").expect("Regex should never fail");
    re.captures(input)
        .and_then(|caps| caps.get(1).map(|m| m.as_str()))
        .unwrap_or("unidentified")
}

fn read_existing_file(xml_path: &Path) -> Result<String, String> {
    let mut file =
        File::open(xml_path).map_err(|e| format!("Failed to open existing file: {}", e))?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("Failed to read existing file: {}", e))?;
    Ok(content)
}

async fn process_images(
    client: &reqwest::Client,
    api_key: &str,
    paths: &[String],
) -> Result<Vec<String>, String> {
    let mut vec_strings = Vec::new();
    for path in paths {
        let result = process_image(client, api_key, path).await?;
        vec_strings.push(result);
    }
    Ok(vec_strings)
}

fn xml_to_json(xml: &str) -> Result<String, String> {
    let document: DocumentInfo =
        from_str(xml).map_err(|e| format!("Failed to parse XML: {}", e))?;
    serde_json::to_string_pretty(&document)
        .map_err(|e| format!("Failed to serialize to JSON: {}", e))
}

fn save_xml_file(xml_content: &str, xml_path: &Path) -> Result<(), String> {
    if let Some(parent) = xml_path.parent() {
        create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    let mut file = File::create(xml_path).map_err(|e| format!("Failed to create file: {}", e))?;
    file.write_all(xml_content.as_bytes())
        .map_err(|e| format!("Failed to write to file: {}", e))?;

    println!("XML file saved at: {:?}", xml_path);
    Ok(())
}

fn format_xml(input: &str) -> Result<String, String> {
    let mut reader = Reader::from_str(input);
    reader.config_mut().trim_text_start = true;
    reader.config_mut().trim_text_end = true;
    reader.config_mut().expand_empty_elements = true;

    let mut writer = Writer::new(Cursor::new(Vec::new()));

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(event) => writer
                .write_event(event)
                .map_err(|e| format!("Error writing XML event: {}", e))?,
            Err(e) => return Err(format!("Error parsing XML: {}", e)),
        }
        buf.clear();
    }

    String::from_utf8(writer.into_inner().into_inner())
        .map_err(|e| format!("Invalid UTF-8 sequence: {}", e))
}

async fn process_image(
    client: &reqwest::Client,
    api_key: &str,
    path: &str,
) -> Result<String, String> {
    let page_number = extract_page_number(path);
    let base64_image = encode_image_to_base64(path)?;
    let prefilled_message = format!("<page number=\"{page_number}\">");

    let max_retries = 5;
    let mut retry_count = 0;

    loop {
        let response = send_anthropic_request(client, api_key, &base64_image, page_number).await?;

        println!("Response: {:?}", response);
        print_rate_limit_headers(&response);

        if response.status() == 529 && retry_count < max_retries {
            println!("Received 529 status code. Retrying after 200ms...");
            sleep(Duration::from_millis(200)).await;
            retry_count += 1;
            continue;
        }

        return handle_anthropic_response(response, &prefilled_message).await;
    }
}

fn encode_image_to_base64(path: &str) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
    Ok(BASE64_STANDARD.encode(&buffer))
}

async fn send_anthropic_request(
    client: &reqwest::Client,
    api_key: &str,
    base64_image: &str,
    page_number: &str,
) -> Result<reqwest::Response, String> {
    client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&json!({
            "model": "claude-3-5-sonnet-20240620",
            "max_tokens": 4096,
            "system": SYSTEM_MESSAGE,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "text",
                            "text": DOCUMENT_PARSE_INITIAL_MESSAGE.replace("{page_number}", page_number),
                        },
                        {
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "media_type": "image/webp",
                                "data": base64_image,
                            },
                        },
                        {
                            "type": "text",
                            "text": DOCUMENT_PARSE_FINAL_MESSAGE.replace("{page_number}", page_number),
                        }
                    ]
                },
                {
                    "role": "assistant",
                    "content": [
                        {
                            "type": "text",
                            "text": format!("<page number=\"{page_number}\">"),
                        }
                    ]
                }
            ]
        }))
        .send()
        .await
        .map_err(|e| format!("Error sending request: {:?}", e))
}

async fn handle_anthropic_response(
    response: reqwest::Response,
    prefilled_message: &str,
) -> Result<String, String> {
    if response.status().is_success() {
        let output: AnthropicResponse = response.json().await
            .map_err(|_| "Response to Anthropic API request is success but there is no JSON output. This is unexpected.".to_string())?;

        println!(
            "Request success. Token usage: input:{}, output:{}",
            output.usage.input_tokens, output.usage.output_tokens
        );

        let text = output.content.last()
            .ok_or("Response to Anthropic API request is success but lacks content. This is unexpected.".to_string())?
            .text.clone();

        Ok(format!("{prefilled_message}{text}"))
    } else {
        let output: AnthropicError = response.json().await
            .map_err(|_| "Response to Anthropic API request is not success but there is no JSON output. This is unexpected.".to_string())?;

        Err(format!(
            "Anthropic request error: type:{}, message:{}",
            output.error.error_type, output.error.message
        ))
    }
}

fn print_rate_limit_headers(response: &reqwest::Response) {
    for header in &[
        "anthropic-ratelimit-requests-limit",
        "anthropic-ratelimit-requests-remaining",
        "anthropic-ratelimit-requests-reset",
        "anthropic-ratelimit-tokens-limit",
        "anthropic-ratelimit-tokens-remaining",
        "anthropic-ratelimit-tokens-reset",
        "retry-after",
    ] {
        if let Some(value) = response.headers().get(*header) {
            println!("{}: {:?}", header, value);
        }
    }
}

async fn process_xml(
    client: &reqwest::Client,
    api_key: &str,
    prompt: &str,
) -> Result<String, String> {
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&json!({
            "model": "claude-3-5-sonnet-20240620",
            "max_tokens": 4096,
            "system": SYSTEM_MESSAGE,
            "messages": [
                {
                    "role": "user",
                    "content": prompt,
                }
            ]
        }))
        .send()
        .await
        .map_err(|e| format!("Error sending request: {:?}", e))?;

    handle_anthropic_response(response, "").await
}

fn read_json_file(json_path: &Path) -> Result<DocumentInfo, String> {
    let json_content =
        fs::read_to_string(json_path).map_err(|e| format!("Failed to read JSON file: {}", e))?;

    serde_json::from_str(&json_content).map_err(|e| format!("Failed to parse JSON: {}", e))
}

fn save_json_file(json_content: &str, json_path: &Path) -> Result<(), String> {
    if let Some(parent) = json_path.parent() {
        create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    fs::write(json_path, json_content).map_err(|e| format!("Failed to write JSON file: {}", e))?;

    println!("JSON file saved at: {:?}", json_path);
    Ok(())
}

#[tauri::command]
pub fn update_file_name(path: String, name: String) -> Result<DocumentInfo, String> {
    let mut document_info: DocumentInfo = read_json_file(&Path::new(&path))?;
    if document_info.file_name != name {
        if document_info.file_name_history.is_empty() {
            document_info.file_name_history.push(document_info.file_name.clone());
        }
        if !document_info.file_name_history.contains(&name) {
            document_info.file_name_history.push(name.clone());
        }
        document_info.file_name = name;

        let serialized_json = serde_json::to_string(&document_info)
            .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
        save_json_file(&serialized_json, &Path::new(&path))?;
    }
    Ok(document_info)
}