use base64::prelude::*;
use dotenv::dotenv;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{fs::File, io::Read};
use tauri_plugin_http::reqwest;
use tokio::time::{sleep, Duration};

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicResponse {
    content: Vec<Content>,
    id: String,
    model: String,
    role: String,
    stop_reason: String,
    stop_sequence: Option<String>,
    #[serde(rename = "type")]
    response_type: String,
    usage: Usage,
}

#[derive(Debug, Serialize, Deserialize)]
struct Content {
    text: String,
    #[serde(rename = "type")]
    content_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Usage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicError {
    #[serde(rename = "type")]
    response_type: String,
    error: OutputError,
}

#[derive(Debug, Serialize, Deserialize)]
struct OutputError {
    #[serde(rename = "type")]
    error_type: String,
    message: String,
}

const SYSTEM_MESSAGE: &str = "You are an expert in document analysis and information extraction, specializing in Brazilian business documents. Output only the XML.";
const INITIAL_MESSAGE_PREFIX: &str = "You will be given an image of a document page. Your task is to extract the data from this image and structure it as XML. Follow these steps carefully:\n\n1. First, examine the provided document image:\n<page_image number=";
const FINAL_MESSAGE_PREFIX: &str = "</page_image>\n\n2. Analyze the content of the image. Look for key elements such as:\n- Title or heading\n- Paragraphs of text\n- Lists (bulleted or numbered)\n- Tables\n- Images or diagrams\n- Signatures\n- Dates\n- Any other relevant information\n\n3. Identify the relationships between these elements. For example, determine which text belongs to which headings, or which cells belong to which rows in a table.\n\n4. Begin structuring the extracted data as XML. Use appropriate tags that describe the content. For example:\n- <title> for the main title\n- <paragraph> for blocks of text\n- <list> for lists, with <item> for each list item\n- <table> for tables, with <row> and <cell> for table contents\n- <image> for images or diagrams, with a brief description\n- <signature> for signatures\n- <date> for dates\n- Create other tags as necessary to accurately represent the document structure\n\n5. Ensure that your XML structure maintains the hierarchy and relationships of the original document page. Nested elements should be properly indented.\n\n6. If there's any text or content you cannot read or understand clearly, use <unclear> tags to indicate this.\n\n7. Once you've extracted and structured all the data, present your output in the following format:\n\n<page number=";
const FINAL_MESSAGE_SUFFIX: &str = ">\n[Your XML-structured data goes here]\n</page>\n\nRemember to be as accurate and detailed as possible in your extraction.";

fn extract_page_number(input: &str) -> &str {
    let re = Regex::new(r"page-(\d+)").expect("Regex should never fail");
    re.captures(input)
        .and_then(|caps| caps.get(1).map(|m| m.as_str()))
        .unwrap_or("unidentified")
}

#[tauri::command]
pub async fn call_anthropic(paths: Vec<String>) -> Result<String, String> {
    dotenv().ok();
    let client = reqwest::Client::new();
    let api_key = std::env::var("ANTHROPIC_API_KEY").map_err(|e| e.to_string())?;
    let mut vec_strings: Vec<String> = vec![];

    for path in paths {
        let result = process_single_image(&client, &api_key, &path).await?;
        vec_strings.push(result);
    }

    let string = vec_strings.join("\n\n\n\n");
    Ok(string)
}

async fn process_single_image(
    client: &reqwest::Client,
    api_key: &str,
    path: &str,
) -> Result<String, String> {
    let page_number = extract_page_number(path);
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
    let base64_image = BASE64_STANDARD.encode(&buffer);
    let prefilled_message = format!("<page number={page_number}>");

    let max_retries = 5;
    let mut retry_count = 0;

    loop {
        let response = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&json!({
                "model": "claude-3-haiku-20240307",
                "max_tokens": 4096,
                "system": SYSTEM_MESSAGE,
                "messages": [
                    {
                        "role": "user",
                        "content": [
                            {
                                "type": "text",
                                "text": format!("{INITIAL_MESSAGE_PREFIX}{page_number}>").as_str(),
                            },
                            {
                                "type": "image",
                                "source": {
                                    "type": "base64",
                                    "media_type": "image/webp",
                                    "data": base64_image.clone(),
                                },
                            },
                            {
                                "type": "text",
                                "text": format!("{FINAL_MESSAGE_PREFIX}{page_number}{FINAL_MESSAGE_SUFFIX}").as_str(),
                            }
                        ]
                    },
                    {
                        "role": "assistant",
                        "content": [
                            {
                                "type": "text",
                                "text": prefilled_message.as_str(),
                            }
                        ]
                    }
                ]
            }))
            .send()
            .await
            .map_err(|e| format!("Error sending request: {:?}", e))?;

        println!("Response: {:?}", response);

        print_rate_limit_headers(&response);

        if response.status() == 529 && retry_count < max_retries {
            println!("Received 529 status code. Retrying after 1 second...");
            sleep(Duration::from_millis(200)).await;
            retry_count += 1;
            continue;
        }

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

            return Ok(format!("{prefilled_message}{text}"));
        } else {
            let output: AnthropicError = response.json().await
                .map_err(|_| "Response to Anthropic API request is not success but there is no JSON output. This is unexpected.".to_string())?;

            return Err(format!(
                "Anthropic request error: type:{}, message:{}",
                output.error.error_type, output.error.message
            ));
        }
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
