//! Read Gemini CLI session history from ~/.gemini/{tmp,history}/**/chats/session-*.json

use chrono::DateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Summary of a Gemini session for sidebar display.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiSessionSummary {
    pub session_id: String,
    pub first_message: String,
    pub updated_at: i64,
    pub created_at: i64,
    pub message_count: usize,
}

/// Single normalized message row used by frontend history parser.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiSessionMessage {
    pub id: String,
    pub role: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    /// "message", "reasoning", or "tool"
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_input: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_output: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GeminiSessionUsage {
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cache_creation_input_tokens: Option<i64>,
    pub cache_read_input_tokens: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiSessionLoadResult {
    pub messages: Vec<GeminiSessionMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<GeminiSessionUsage>,
}

fn parse_timestamp_millis(value: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let truncated: String = value.chars().take(max_chars).collect();
    format!("{}…", truncated)
}

fn normalize_windows_path_for_comparison(path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }
    let mut normalized = path.replace('\\', "/");
    if normalized.starts_with("//?/UNC/") {
        normalized = format!("//{}", &normalized["//?/UNC/".len()..]);
    } else if normalized.starts_with("//?/") {
        normalized = normalized["//?/".len()..].to_string();
    }
    while normalized.ends_with('/') && normalized.len() > 1 {
        normalized.pop();
    }
    normalized
}

fn build_path_variants(path: &str) -> Vec<String> {
    let normalized = normalize_windows_path_for_comparison(path.trim());
    if normalized.is_empty() {
        return Vec::new();
    }
    let mut variants = vec![normalized.clone()];
    if normalized.starts_with("/private/") {
        variants.push(normalized["/private".len()..].to_string());
    } else if normalized.starts_with('/') {
        variants.push(format!("/private{}", normalized));
    }
    if normalized.len() >= 2 && normalized.as_bytes()[1] == b':' {
        let mut chars = normalized.chars();
        if let Some(first) = chars.next() {
            variants.push(format!("{}{}", first.to_ascii_lowercase(), chars.as_str()));
        }
        variants.push(normalized.to_ascii_lowercase());
    }
    if normalized.starts_with("//") {
        variants.push(normalized.to_ascii_lowercase());
    }
    variants.sort();
    variants.dedup();
    variants
}

fn matches_workspace_path(project_root: &str, workspace_path: &Path) -> bool {
    let workspace_raw = workspace_path.to_string_lossy().to_string();
    let workspace_variants = build_path_variants(&workspace_raw);
    if workspace_variants.is_empty() {
        return false;
    }
    let project_variants = build_path_variants(project_root);
    for candidate in project_variants {
        for workspace in &workspace_variants {
            if candidate == *workspace {
                return true;
            }
            if candidate.starts_with(workspace)
                && candidate.chars().nth(workspace.len()) == Some('/')
            {
                return true;
            }
        }
    }
    false
}

fn resolve_gemini_base_dir(custom_home: Option<&str>) -> PathBuf {
    if let Some(home) = custom_home.map(str::trim).filter(|value| !value.is_empty()) {
        return PathBuf::from(home);
    }
    if let Some(home) = std::env::var_os("GEMINI_CLI_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
    {
        return home;
    }
    dirs::home_dir().unwrap_or_default().join(".gemini")
}

fn is_chat_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    if path.extension().and_then(|value| value.to_str()) != Some("json") {
        return false;
    }
    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    if !file_name.starts_with("session-") {
        return false;
    }
    path.parent()
        .and_then(|value| value.file_name())
        .and_then(|value| value.to_str())
        == Some("chats")
}

fn collect_chat_files_sync(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if !root.exists() {
        return files;
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(path) = stack.pop() {
        let read_dir = match std::fs::read_dir(&path) {
            Ok(reader) => reader,
            Err(_) => continue,
        };
        for entry in read_dir.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                stack.push(entry_path);
                continue;
            }
            if is_chat_file(&entry_path) {
                files.push(entry_path);
            }
        }
    }
    files.sort();
    files.dedup();
    files
}

async fn collect_chat_files(base_dir: &Path) -> Vec<PathBuf> {
    let roots = vec![base_dir.join("tmp"), base_dir.join("history")];
    let mut all = Vec::new();
    for root in roots {
        let root_clone = root.clone();
        let mut found = tokio::task::spawn_blocking(move || collect_chat_files_sync(&root_clone))
            .await
            .unwrap_or_default();
        all.append(&mut found);
    }
    all.sort();
    all.dedup();
    all
}

fn project_alias_from_chat_path(path: &Path) -> Option<String> {
    path.parent()?
        .parent()?
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
}

fn read_project_root_file(path: PathBuf) -> Option<String> {
    let raw = std::fs::read_to_string(path).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn load_projects_alias_map(base_dir: &Path) -> HashMap<String, String> {
    let path = base_dir.join("projects.json");
    let raw = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return HashMap::new(),
    };
    let value: Value = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(_) => return HashMap::new(),
    };
    let mut map = HashMap::new();
    let Some(projects) = value.get("projects").and_then(|v| v.as_object()) else {
        return map;
    };
    for (project_path, alias_value) in projects {
        let Some(alias) = alias_value.as_str() else {
            continue;
        };
        let trimmed_alias = alias.trim();
        if trimmed_alias.is_empty() {
            continue;
        }
        map.insert(trimmed_alias.to_string(), project_path.to_string());
    }
    map
}

fn resolve_project_root(
    base_dir: &Path,
    alias: &str,
    projects_map: &HashMap<String, String>,
) -> Option<String> {
    let tmp_candidate = base_dir.join("tmp").join(alias).join(".project_root");
    if let Some(path) = read_project_root_file(tmp_candidate) {
        return Some(path);
    }
    let history_candidate = base_dir.join("history").join(alias).join(".project_root");
    if let Some(path) = read_project_root_file(history_candidate) {
        return Some(path);
    }
    projects_map.get(alias).cloned()
}

fn extract_text_from_value(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Value::Array(items) => {
            let mut parts = Vec::new();
            for item in items {
                if let Some(text) = item
                    .get("text")
                    .and_then(extract_text_from_value)
                    .or_else(|| extract_text_from_value(item))
                {
                    parts.push(text);
                }
            }
            if parts.is_empty() {
                None
            } else {
                Some(parts.join("\n"))
            }
        }
        Value::Object(map) => map
            .get("text")
            .and_then(extract_text_from_value)
            .or_else(|| map.get("message").and_then(extract_text_from_value))
            .or_else(|| map.get("content").and_then(extract_text_from_value)),
        _ => None,
    }
}

fn extract_message_text(message: &Value) -> Option<String> {
    message
        .get("content")
        .and_then(extract_text_from_value)
        .or_else(|| message.get("message").and_then(extract_text_from_value))
}

fn count_inline_images(value: &Value) -> usize {
    if let Some(array) = value.as_array() {
        return array.iter().map(count_inline_images).sum();
    }
    if let Some(object) = value.as_object() {
        if object
            .get("inlineData")
            .or_else(|| object.get("inline_data"))
            .and_then(|node| node.get("data"))
            .and_then(|value| value.as_str())
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
        {
            return 1;
        }
        return object.values().map(count_inline_images).sum();
    }
    0
}

fn extract_usage(message: &Value) -> Option<GeminiSessionUsage> {
    let tokens = message.get("tokens")?;
    Some(GeminiSessionUsage {
        input_tokens: tokens.get("input").and_then(|v| v.as_i64()),
        output_tokens: tokens.get("output").and_then(|v| v.as_i64()),
        cache_creation_input_tokens: None,
        cache_read_input_tokens: tokens.get("cached").and_then(|v| v.as_i64()),
    })
}

fn tool_call_is_error(call: &Value, output_preview: Option<&str>) -> bool {
    if call
        .get("status")
        .and_then(|v| v.as_str())
        .map(|status| {
            matches!(
                status.to_ascii_lowercase().as_str(),
                "error" | "failed" | "failure" | "cancelled" | "canceled"
            )
        })
        .unwrap_or(false)
    {
        return true;
    }
    output_preview
        .map(|output| {
            output
                .trim_start()
                .to_ascii_lowercase()
                .starts_with("error")
        })
        .unwrap_or(false)
}

fn parse_summary_from_value(path: &Path, value: &Value) -> Option<GeminiSessionSummary> {
    let session_id = value.get("sessionId").and_then(|v| v.as_str())?.to_string();
    let messages = value
        .get("messages")
        .and_then(|m| m.as_array())
        .cloned()
        .unwrap_or_default();
    let first_message_ts = messages
        .first()
        .and_then(|m| m.get("timestamp"))
        .and_then(|v| v.as_str())
        .and_then(parse_timestamp_millis);
    let last_message_ts = messages.iter().rev().find_map(|m| {
        m.get("timestamp")
            .and_then(|v| v.as_str())
            .and_then(parse_timestamp_millis)
    });

    let started_at = value
        .get("startTime")
        .and_then(|v| v.as_str())
        .and_then(parse_timestamp_millis)
        .or(first_message_ts)
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
    let updated_at = value
        .get("lastUpdated")
        .and_then(|v| v.as_str())
        .and_then(parse_timestamp_millis)
        .or(last_message_ts)
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    let first_message = messages
        .iter()
        .filter(|message| message.get("type").and_then(|v| v.as_str()) == Some("user"))
        .find_map(extract_message_text)
        .map(|text| truncate_chars(&text, 60))
        .unwrap_or_else(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("Gemini Session")
                .to_string()
        });

    Some(GeminiSessionSummary {
        session_id,
        first_message,
        updated_at,
        created_at: started_at,
        message_count: messages.len(),
    })
}

fn parse_messages_from_value(value: &Value) -> GeminiSessionLoadResult {
    let mut messages: Vec<GeminiSessionMessage> = Vec::new();
    let mut usage: Option<GeminiSessionUsage> = None;
    let mut counter = 0usize;
    let raw_messages = value
        .get("messages")
        .and_then(|m| m.as_array())
        .cloned()
        .unwrap_or_default();

    for raw in raw_messages {
        let msg_type = raw
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let timestamp = raw
            .get("timestamp")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let base_id = raw
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                counter += 1;
                format!("gemini-msg-{}", counter)
            });

        match msg_type.as_str() {
            "user" => {
                let mut text = extract_message_text(&raw).unwrap_or_default();
                let image_count = raw.get("content").map(count_inline_images).unwrap_or(0);
                if image_count > 0 {
                    let image_marker = if image_count == 1 {
                        "[image]".to_string()
                    } else {
                        format!("[image x{}]", image_count)
                    };
                    if text.trim().is_empty() {
                        text = image_marker;
                    } else {
                        text = format!("{}\n{}", text, image_marker);
                    }
                }
                if text.trim().is_empty() {
                    continue;
                }
                messages.push(GeminiSessionMessage {
                    id: base_id,
                    role: "user".to_string(),
                    text,
                    timestamp,
                    kind: "message".to_string(),
                    tool_type: None,
                    title: None,
                    tool_input: None,
                    tool_output: None,
                });
            }
            "gemini" | "assistant" | "model" => {
                if let Some(thoughts) = raw.get("thoughts").and_then(|v| v.as_array()) {
                    for thought in thoughts {
                        let subject = thought
                            .get("subject")
                            .and_then(|v| v.as_str())
                            .map(str::trim)
                            .filter(|s| !s.is_empty());
                        let description = thought
                            .get("description")
                            .and_then(|v| v.as_str())
                            .map(str::trim)
                            .filter(|s| !s.is_empty());
                        let text = match (subject, description) {
                            (Some(sub), Some(desc)) => format!("{}: {}", sub, desc),
                            (Some(sub), None) => sub.to_string(),
                            (None, Some(desc)) => desc.to_string(),
                            (None, None) => continue,
                        };
                        counter += 1;
                        messages.push(GeminiSessionMessage {
                            id: format!("{}-reasoning-{}", base_id, counter),
                            role: "assistant".to_string(),
                            text,
                            timestamp: timestamp.clone(),
                            kind: "reasoning".to_string(),
                            tool_type: None,
                            title: None,
                            tool_input: None,
                            tool_output: None,
                        });
                    }
                }

                if let Some(tool_calls) = raw.get("toolCalls").and_then(|v| v.as_array()) {
                    for call in tool_calls {
                        counter += 1;
                        let tool_use_id = call
                            .get("id")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| format!("{}-tool-{}", base_id, counter));
                        let tool_name = call
                            .get("displayName")
                            .or_else(|| call.get("name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("tool")
                            .to_string();
                        let input_value = call
                            .get("args")
                            .cloned()
                            .or_else(|| call.get("input").cloned());
                        let input_text = input_value
                            .as_ref()
                            .and_then(|v| serde_json::to_string_pretty(v).ok())
                            .unwrap_or_default();
                        messages.push(GeminiSessionMessage {
                            id: tool_use_id.clone(),
                            role: "assistant".to_string(),
                            text: input_text,
                            timestamp: timestamp.clone(),
                            kind: "tool".to_string(),
                            tool_type: Some(tool_name.clone()),
                            title: Some(tool_name),
                            tool_input: input_value,
                            tool_output: None,
                        });

                        let output_preview = call
                            .get("resultDisplay")
                            .and_then(|v| v.as_str())
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string())
                            .or_else(|| {
                                call.get("result")
                                    .and_then(|v| serde_json::to_string(v).ok())
                            });
                        if let Some(output) = output_preview {
                            let is_error = tool_call_is_error(call, Some(output.as_str()));
                            messages.push(GeminiSessionMessage {
                                id: format!("{}-result", tool_use_id),
                                role: "assistant".to_string(),
                                text: output,
                                timestamp: timestamp.clone(),
                                kind: "tool".to_string(),
                                tool_type: Some(if is_error {
                                    "error".to_string()
                                } else {
                                    "result".to_string()
                                }),
                                title: Some(if is_error {
                                    "Error".to_string()
                                } else {
                                    "Result".to_string()
                                }),
                                tool_input: None,
                                tool_output: call.get("result").cloned(),
                            });
                        }
                    }
                }

                if let Some(text) = extract_message_text(&raw) {
                    if !text.trim().is_empty() {
                        messages.push(GeminiSessionMessage {
                            id: base_id.clone(),
                            role: "assistant".to_string(),
                            text,
                            timestamp: timestamp.clone(),
                            kind: "message".to_string(),
                            tool_type: None,
                            title: None,
                            tool_input: None,
                            tool_output: None,
                        });
                    }
                }

                if let Some(extracted_usage) = extract_usage(&raw) {
                    usage = Some(extracted_usage);
                }
            }
            _ => {}
        }
    }

    GeminiSessionLoadResult { messages, usage }
}

async fn read_json(path: &Path) -> Result<Value, String> {
    let raw = fs::read_to_string(path).await.map_err(|error| {
        format!(
            "Failed to read Gemini session file {}: {}",
            path.display(),
            error
        )
    })?;
    serde_json::from_str(&raw).map_err(|error| {
        format!(
            "Failed to parse Gemini session file {}: {}",
            path.display(),
            error
        )
    })
}

async fn resolve_workspace_session_files(
    workspace_path: &Path,
    custom_home: Option<&str>,
) -> Vec<(PathBuf, Value)> {
    let base_dir = resolve_gemini_base_dir(custom_home);
    let files = collect_chat_files(&base_dir).await;
    let projects_map = load_projects_alias_map(&base_dir);
    let mut matched = Vec::new();

    for file in files {
        let Some(alias) = project_alias_from_chat_path(&file) else {
            continue;
        };
        let Some(project_root) = resolve_project_root(&base_dir, &alias, &projects_map) else {
            continue;
        };
        if !matches_workspace_path(&project_root, workspace_path) {
            continue;
        }
        let Ok(value) = read_json(&file).await else {
            continue;
        };
        matched.push((file, value));
    }
    matched
}

/// List Gemini sessions for a workspace path.
pub async fn list_gemini_sessions(
    workspace_path: &Path,
    limit: Option<usize>,
    custom_home: Option<&str>,
) -> Result<Vec<GeminiSessionSummary>, String> {
    let matched_files = resolve_workspace_session_files(workspace_path, custom_home).await;
    let mut sessions = Vec::new();
    for (path, value) in matched_files {
        if let Some(summary) = parse_summary_from_value(&path, &value) {
            sessions.push(summary);
        }
    }
    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    sessions.truncate(limit.unwrap_or(200));
    Ok(sessions)
}

/// Load full Gemini session messages by session id.
pub async fn load_gemini_session(
    workspace_path: &Path,
    session_id: &str,
    custom_home: Option<&str>,
) -> Result<GeminiSessionLoadResult, String> {
    let matched_files = resolve_workspace_session_files(workspace_path, custom_home).await;
    for (_path, value) in matched_files {
        let current_session_id = value
            .get("sessionId")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if current_session_id == session_id {
            return Ok(parse_messages_from_value(&value));
        }
    }
    Err(format!("Gemini session not found: {}", session_id))
}

/// Delete Gemini session file by session id.
pub async fn delete_gemini_session(
    workspace_path: &Path,
    session_id: &str,
    custom_home: Option<&str>,
) -> Result<(), String> {
    let normalized_session_id = session_id.trim();
    if normalized_session_id.is_empty()
        || normalized_session_id.contains('/')
        || normalized_session_id.contains('\\')
        || normalized_session_id.contains("..")
    {
        return Err("[SESSION_NOT_FOUND] Invalid Gemini session id".to_string());
    }

    let matched_files = resolve_workspace_session_files(workspace_path, custom_home).await;
    for (path, value) in matched_files {
        let current_session_id = value
            .get("sessionId")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if current_session_id != normalized_session_id {
            continue;
        }
        fs::remove_file(&path).await.map_err(|error| {
            format!(
                "[IO_ERROR] Failed to delete Gemini session file {}: {}",
                path.display(),
                error
            )
        })?;
        return Ok(());
    }

    Err(format!(
        "[SESSION_NOT_FOUND] Gemini session file not found: {}",
        normalized_session_id
    ))
}
