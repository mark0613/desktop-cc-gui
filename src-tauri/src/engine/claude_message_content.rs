use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};

use super::SendMessageParams;

/// Format the user's AskUserQuestion answers into a human-readable message
/// that can be sent as a follow-up via `--resume`.
pub(super) fn format_ask_user_answer(result: &Value) -> String {
    let mut parts = Vec::new();

    if let Some(answers_obj) = result.get("answers").and_then(|a| a.as_object()) {
        for (_key, entry) in answers_obj {
            if let Some(arr) = entry.get("answers").and_then(|a| a.as_array()) {
                let texts: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
                if !texts.is_empty() {
                    parts.push(texts.join(", "));
                }
            }
        }
    }

    if parts.is_empty() {
        "The user dismissed the question without selecting an option.".to_string()
    } else {
        format!(
            "The user answered the AskUserQuestion: {}. Please continue based on this selection.",
            parts.join("; ")
        )
    }
}

/// Build message content with images for stream-json input
pub(super) fn build_message_content(params: &SendMessageParams) -> Result<Value, String> {
    let mut content = Vec::new();

    if let Some(ref images) = params.images {
        for image_path in images {
            let trimmed = image_path.trim();
            if trimmed.is_empty() {
                continue;
            }

            if trimmed.starts_with("data:") {
                let parts: Vec<&str> = trimmed.splitn(2, ',').collect();
                if parts.len() == 2 {
                    let media_type = parts[0]
                        .strip_prefix("data:")
                        .and_then(|s| s.strip_suffix(";base64"))
                        .unwrap_or("image/png");
                    content.push(json!({
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": media_type,
                            "data": parts[1]
                        }
                    }));
                }
            } else if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
                content.push(json!({
                    "type": "image",
                    "source": {
                        "type": "url",
                        "url": trimmed
                    }
                }));
            } else {
                let path = std::path::Path::new(trimmed);
                if let Ok(data) = std::fs::read(path) {
                    let base64_data = STANDARD.encode(&data);
                    let media_type = match path.extension().and_then(|e| e.to_str()) {
                        Some("png") => "image/png",
                        Some("jpg") | Some("jpeg") => "image/jpeg",
                        Some("gif") => "image/gif",
                        Some("webp") => "image/webp",
                        _ => "image/png",
                    };
                    content.push(json!({
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": media_type,
                            "data": base64_data
                        }
                    }));
                }
            }
        }
    }

    if !params.text.trim().is_empty() {
        content.push(json!({
            "type": "text",
            "text": params.text.trim()
        }));
    }

    Ok(json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": content
        }
    }))
}
