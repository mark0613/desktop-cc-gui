//! Gemini engine implementation
//!
//! Handles Gemini CLI execution via:
//! `gemini -p "<prompt>" --output-format stream-json`

use serde_json::{json, Value};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{broadcast, Mutex, RwLock};

use super::events::EngineEvent;
use super::{EngineConfig, EngineType, SendMessageParams};

#[derive(Debug, Clone)]
pub struct GeminiTurnEvent {
    pub turn_id: String,
    pub event: EngineEvent,
}

#[derive(Debug, Default)]
struct GeminiVendorRuntimeConfig {
    env: HashMap<String, String>,
    auth_mode: Option<String>,
}

/// Gemini session for a workspace
pub struct GeminiSession {
    pub workspace_id: String,
    pub workspace_path: PathBuf,
    session_id: RwLock<Option<String>>,
    event_sender: broadcast::Sender<GeminiTurnEvent>,
    bin_path: Option<String>,
    home_dir: Option<String>,
    custom_args: Option<String>,
    active_processes: Mutex<HashMap<String, Child>>,
    interrupted: AtomicBool,
}

impl GeminiSession {
    pub fn new(
        workspace_id: String,
        workspace_path: PathBuf,
        config: Option<EngineConfig>,
    ) -> Self {
        let (event_sender, _) = broadcast::channel(1024);
        let config = config.unwrap_or_default();
        Self {
            workspace_id,
            workspace_path,
            session_id: RwLock::new(None),
            event_sender,
            bin_path: config.bin_path,
            home_dir: config.home_dir,
            custom_args: config.custom_args,
            active_processes: Mutex::new(HashMap::new()),
            interrupted: AtomicBool::new(false),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<GeminiTurnEvent> {
        self.event_sender.subscribe()
    }

    pub async fn get_session_id(&self) -> Option<String> {
        self.session_id.read().await.clone()
    }

    async fn set_session_id(&self, id: Option<String>) {
        *self.session_id.write().await = id;
    }

    fn emit_turn_event(&self, turn_id: &str, event: EngineEvent) {
        let _ = self.event_sender.send(GeminiTurnEvent {
            turn_id: turn_id.to_string(),
            event,
        });
    }

    pub fn emit_error(&self, turn_id: &str, error: String) {
        self.emit_turn_event(
            turn_id,
            EngineEvent::TurnError {
                workspace_id: self.workspace_id.clone(),
                error,
                code: None,
            },
        );
    }

    fn with_external_spec_hint(text: &str, custom_spec_root: Option<&str>) -> String {
        let Some(spec_root) = custom_spec_root
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return text.to_string();
        };
        if !Path::new(spec_root).is_absolute() {
            return text.to_string();
        }
        format!(
            "[External OpenSpec Root]\n- Path: {spec_root}\n- Treat this as the active spec root when checking or reading project specs.\n[/External OpenSpec Root]\n\n{text}"
        )
    }

    fn normalize_auth_mode(raw_mode: Option<&str>) -> Option<&'static str> {
        let normalized = raw_mode
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_ascii_lowercase())?;
        match normalized.as_str() {
            "custom" => Some("custom"),
            "login_google" => Some("login_google"),
            "gemini_api_key" => Some("gemini_api_key"),
            "vertex_adc" => Some("vertex_adc"),
            "vertex_service_account" => Some("vertex_service_account"),
            "vertex_api_key" => Some("vertex_api_key"),
            _ => None,
        }
    }

    fn selected_auth_type_for_mode(raw_mode: Option<&str>) -> &'static str {
        match Self::normalize_auth_mode(raw_mode) {
            Some("login_google") => "oauth-personal",
            Some("vertex_adc") | Some("vertex_service_account") | Some("vertex_api_key") => {
                "vertex-ai"
            }
            Some("custom") | Some("gemini_api_key") => "gemini-api-key",
            _ => "oauth-personal",
        }
    }

    fn resolve_global_gemini_dir(home_override: Option<&str>) -> Option<PathBuf> {
        let Some(home) = dirs::home_dir() else {
            return None;
        };
        let override_path = home_override
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        let Some(raw_root) = override_path.or(Some(home)) else {
            return None;
        };
        if raw_root
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value == ".gemini")
        {
            return Some(raw_root);
        }
        Some(raw_root.join(".gemini"))
    }

    fn persist_auth_mode_hint(auth_mode: Option<&str>, home_override: Option<&str>) {
        let Some(gemini_dir) = Self::resolve_global_gemini_dir(home_override) else {
            return;
        };
        let selected_type = Self::selected_auth_type_for_mode(auth_mode);
        let settings_path = gemini_dir.join("settings.json");
        let mut root = std::fs::read_to_string(&settings_path)
            .ok()
            .and_then(|content| serde_json::from_str::<Value>(&content).ok())
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();

        let security = root
            .entry("security".to_string())
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
        if !security.is_object() {
            *security = Value::Object(serde_json::Map::new());
        }
        let Some(security_obj) = security.as_object_mut() else {
            return;
        };

        let auth = security_obj
            .entry("auth".to_string())
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
        if !auth.is_object() {
            *auth = Value::Object(serde_json::Map::new());
        }
        let Some(auth_obj) = auth.as_object_mut() else {
            return;
        };
        auth_obj.insert(
            "selectedType".to_string(),
            Value::String(selected_type.to_string()),
        );
        auth_obj.insert("useExternal".to_string(), Value::Bool(false));

        if let Some(parent) = settings_path.parent() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                log::warn!(
                    "failed to ensure Gemini settings dir {}: {}",
                    parent.display(),
                    error
                );
                return;
            }
        }
        let content = match serde_json::to_string_pretty(&Value::Object(root)) {
            Ok(serialized) => serialized,
            Err(error) => {
                log::warn!("failed to serialize Gemini settings auth hint: {}", error);
                return;
            }
        };
        if let Err(error) = std::fs::write(&settings_path, content) {
            log::warn!(
                "failed to persist Gemini settings auth hint to {}: {}",
                settings_path.display(),
                error
            );
        }
    }

    fn apply_auth_mode_env_overrides(cmd: &mut Command, auth_mode: Option<&str>) {
        match Self::normalize_auth_mode(auth_mode) {
            Some("login_google") => {
                cmd.env("GOOGLE_GENAI_USE_GCA", "true");
                cmd.env_remove("GOOGLE_GENAI_USE_VERTEXAI");
            }
            Some("vertex_adc") | Some("vertex_service_account") | Some("vertex_api_key") => {
                cmd.env("GOOGLE_GENAI_USE_VERTEXAI", "true");
                cmd.env_remove("GOOGLE_GENAI_USE_GCA");
            }
            Some("custom") | Some("gemini_api_key") => {
                cmd.env_remove("GOOGLE_GENAI_USE_GCA");
                cmd.env_remove("GOOGLE_GENAI_USE_VERTEXAI");
            }
            _ => {}
        }
    }

    fn load_vendor_runtime_config() -> GeminiVendorRuntimeConfig {
        let mut result = GeminiVendorRuntimeConfig::default();
        let Some(home) = dirs::home_dir() else {
            return result;
        };
        let config_path = home.join(".codemoss").join("config.json");
        let Ok(content) = std::fs::read_to_string(config_path) else {
            return result;
        };
        let Ok(root) = serde_json::from_str::<Value>(&content) else {
            return result;
        };
        let Some(gemini) = root.get("gemini").and_then(Value::as_object) else {
            return result;
        };
        let enabled = gemini
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        if !enabled {
            return result;
        }
        result.auth_mode = gemini
            .get("auth_mode")
            .or_else(|| gemini.get("authMode"))
            .and_then(Value::as_str)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let Some(env_obj) = gemini.get("env").and_then(Value::as_object) else {
            return result;
        };
        for (key, value) in env_obj {
            let normalized_key = key.trim();
            if normalized_key.is_empty() {
                continue;
            }
            let normalized_value = value.as_str().map(|v| v.trim().to_string()).or_else(|| {
                if value.is_null() {
                    None
                } else {
                    Some(value.to_string())
                }
            });
            let Some(normalized_value) = normalized_value else {
                continue;
            };
            if normalized_value.is_empty() {
                continue;
            }
            result
                .env
                .insert(normalized_key.to_string(), normalized_value);
        }
        result
    }

    fn build_command(&self, params: &SendMessageParams) -> Command {
        let bin = if let Some(ref custom) = self.bin_path {
            custom.clone()
        } else {
            crate::backend::app_server::find_cli_binary("gemini", None)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "gemini".to_string())
        };

        let mut cmd = crate::backend::app_server::build_command_for_binary(&bin);
        cmd.current_dir(&self.workspace_path);
        cmd.arg("--output-format");
        cmd.arg("stream-json");

        if let Some(model) = params
            .model
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        {
            cmd.arg("--model");
            cmd.arg(model);
        }

        match params.access_mode.as_deref() {
            Some("full-access") => {
                cmd.arg("--approval-mode");
                cmd.arg("yolo");
            }
            Some("read-only") => {
                cmd.arg("--approval-mode");
                cmd.arg("plan");
            }
            Some("default") => {
                cmd.arg("--approval-mode");
                cmd.arg("default");
            }
            _ => {
                cmd.arg("--approval-mode");
                cmd.arg("auto_edit");
            }
        }

        if params.continue_session {
            if let Some(session_id) = params
                .session_id
                .as_ref()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
            {
                cmd.arg("--resume");
                cmd.arg(session_id);
            }
        }

        if let Some(args) = self.custom_args.as_ref() {
            for arg in args.split_whitespace() {
                cmd.arg(arg);
            }
        }

        let message_text =
            Self::with_external_spec_hint(&params.text, params.custom_spec_root.as_deref());
        let safe_text = if message_text.starts_with('-') {
            format!(" {}", message_text)
        } else {
            message_text
        };
        cmd.arg("--prompt");
        cmd.arg(safe_text);

        let vendor_runtime = Self::load_vendor_runtime_config();
        for (key, value) in vendor_runtime.env {
            cmd.env(key, value);
        }
        Self::apply_auth_mode_env_overrides(&mut cmd, vendor_runtime.auth_mode.as_deref());
        Self::persist_auth_mode_hint(
            vendor_runtime.auth_mode.as_deref(),
            self.home_dir.as_deref(),
        );

        if let Some(home) = self.home_dir.as_ref() {
            cmd.env("GEMINI_CLI_HOME", home);
        }

        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd
    }

    pub async fn send_message(
        &self,
        params: SendMessageParams,
        turn_id: &str,
    ) -> Result<String, String> {
        let mut cmd = self.build_command(&params);
        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(error) => {
                let error_msg = format!("Failed to spawn gemini: {}", error);
                self.emit_error(turn_id, error_msg.clone());
                return Err(error_msg);
            }
        };

        let stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                let error_msg = "Failed to capture stdout".to_string();
                self.emit_error(turn_id, error_msg.clone());
                return Err(error_msg);
            }
        };
        let stderr = match child.stderr.take() {
            Some(stderr) => stderr,
            None => {
                let error_msg = "Failed to capture stderr".to_string();
                self.emit_error(turn_id, error_msg.clone());
                return Err(error_msg);
            }
        };

        {
            let mut active = self.active_processes.lock().await;
            active.insert(turn_id.to_string(), child);
        }

        self.emit_turn_event(
            turn_id,
            EngineEvent::SessionStarted {
                workspace_id: self.workspace_id.clone(),
                session_id: "pending".to_string(),
                engine: EngineType::Gemini,
            },
        );
        self.emit_turn_event(
            turn_id,
            EngineEvent::TurnStarted {
                workspace_id: self.workspace_id.clone(),
                turn_id: turn_id.to_string(),
            },
        );

        let stderr_reader = BufReader::new(stderr);
        let stderr_task = tokio::spawn(async move {
            let mut lines = stderr_reader.lines();
            let mut text = String::new();
            while let Ok(Some(line)) = lines.next_line().await {
                text.push_str(&line);
                text.push('\n');
            }
            text
        });

        let mut response_text = String::new();
        let mut saw_turn_completed = false;
        let mut saw_turn_error = false;
        let mut saw_tool_activity = false;
        let mut error_output = String::new();
        let mut session_started_emitted = false;
        let mut new_session_id: Option<String> = None;
        let mut observed_event_types = BTreeSet::new();

        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }
            match serde_json::from_str::<Value>(&line) {
                Ok(event) => {
                    if let Some(event_type) = event.get("type").and_then(|value| value.as_str()) {
                        observed_event_types.insert(event_type.to_string());
                    }
                    if let Some(session_id) = extract_session_id(&event) {
                        if !session_started_emitted {
                            session_started_emitted = true;
                            new_session_id = Some(session_id.clone());
                            self.emit_turn_event(
                                turn_id,
                                EngineEvent::SessionStarted {
                                    workspace_id: self.workspace_id.clone(),
                                    session_id,
                                    engine: EngineType::Gemini,
                                },
                            );
                        }
                    }
                    if let Some(unified_event) = parse_gemini_event(&self.workspace_id, &event) {
                        match &unified_event {
                            EngineEvent::TextDelta { text, .. } => {
                                response_text.push_str(text);
                            }
                            EngineEvent::ToolStarted { .. } | EngineEvent::ToolCompleted { .. } => {
                                saw_tool_activity = true;
                            }
                            EngineEvent::TurnError { .. } => {
                                saw_turn_error = true;
                            }
                            EngineEvent::TurnCompleted { result, .. } => {
                                saw_turn_completed = true;
                                if response_text.trim().is_empty() {
                                    if let Some(result_text) = result
                                        .as_ref()
                                        .and_then(|value| extract_text_from_value(value, 0))
                                    {
                                        response_text = result_text;
                                    }
                                }
                            }
                            _ => {}
                        }
                        self.emit_turn_event(turn_id, unified_event);
                    }
                }
                Err(_) => {
                    error_output.push_str(&line);
                    error_output.push('\n');
                }
            }
        }

        let mut child = {
            let mut active = self.active_processes.lock().await;
            active.remove(turn_id)
        };
        let status = if let Some(mut process) = child.take() {
            process.wait().await.ok()
        } else {
            None
        };
        let stderr_text = stderr_task.await.unwrap_or_default();
        if !stderr_text.trim().is_empty() {
            error_output.push_str(&stderr_text);
        }

        if let Some(status) = status {
            if !status.success() {
                let error_msg = if self.interrupted.swap(false, Ordering::SeqCst) {
                    "Session stopped.".to_string()
                } else if !error_output.trim().is_empty() {
                    error_output.trim().to_string()
                } else {
                    format!("Gemini exited with status: {}", status)
                };
                self.emit_error(turn_id, error_msg.clone());
                return Err(error_msg);
            }
        } else if self.interrupted.swap(false, Ordering::SeqCst) {
            let error_msg = "Session stopped.".to_string();
            self.emit_error(turn_id, error_msg.clone());
            return Err(error_msg);
        }

        if response_text.trim().is_empty() && !error_output.trim().is_empty() {
            let error_msg = error_output.trim().to_string();
            self.emit_error(turn_id, error_msg.clone());
            return Err(error_msg);
        }

        if response_text.trim().is_empty() && saw_turn_error {
            return Err("Gemini returned an error event.".to_string());
        }

        if response_text.trim().is_empty() {
            let observed = if observed_event_types.is_empty() {
                "none".to_string()
            } else {
                observed_event_types
                    .iter()
                    .cloned()
                    .collect::<Vec<String>>()
                    .join(", ")
            };
            let reason = if saw_turn_completed {
                "Gemini completed but produced no assistant output."
            } else {
                "Gemini exited without a completion event or assistant output."
            };
            let diagnostic = format!("{reason} Observed event types: {observed}.");
            if !saw_tool_activity {
                self.emit_error(turn_id, diagnostic.clone());
                return Err(diagnostic);
            }
        }

        if let Some(session_id) = new_session_id {
            self.set_session_id(Some(session_id)).await;
        }

        if !saw_turn_completed && !saw_turn_error {
            self.emit_turn_event(
                turn_id,
                EngineEvent::TurnCompleted {
                    workspace_id: self.workspace_id.clone(),
                    result: Some(json!({
                        "text": response_text,
                    })),
                },
            );
        }

        Ok(response_text)
    }

    pub async fn interrupt(&self) -> Result<(), String> {
        self.interrupted.store(true, Ordering::SeqCst);
        let mut active = self.active_processes.lock().await;
        for child in active.values_mut() {
            child
                .kill()
                .await
                .map_err(|e| format!("Failed to kill process: {}", e))?;
        }
        active.clear();
        Ok(())
    }
}

fn first_non_empty_str<'a>(candidates: &[Option<&'a str>]) -> Option<&'a str> {
    for value in candidates {
        if let Some(text) = value {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
    }
    None
}

fn extract_text_from_value(value: &Value, depth: usize) -> Option<String> {
    if depth > 4 {
        return None;
    }
    if let Some(text) = value
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        return Some(text.to_string());
    }
    if let Some(array) = value.as_array() {
        let mut merged = String::new();
        for item in array {
            if let Some(text) = extract_text_from_value(item, depth + 1) {
                if !merged.is_empty() {
                    merged.push('\n');
                }
                merged.push_str(&text);
            }
        }
        return if merged.trim().is_empty() {
            None
        } else {
            Some(merged)
        };
    }
    if let Some(object) = value.as_object() {
        let direct = first_non_empty_str(&[
            object.get("delta").and_then(|v| v.as_str()),
            object.get("text").and_then(|v| v.as_str()),
            object.get("message").and_then(|v| v.as_str()),
            object.get("content").and_then(|v| v.as_str()),
        ]);
        if let Some(text) = direct {
            return Some(text.to_string());
        }
        for key in [
            "content", "message", "part", "parts", "result", "output", "response", "data",
            "payload",
        ] {
            if let Some(nested) = object.get(key) {
                if let Some(text) = extract_text_from_value(nested, depth + 1) {
                    return Some(text);
                }
            }
        }
    }
    None
}

fn extract_session_id(event: &Value) -> Option<String> {
    let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");
    if event_type != "init" {
        return None;
    }
    let session_id = first_non_empty_str(&[
        event.get("session_id").and_then(|v| v.as_str()),
        event.get("sessionId").and_then(|v| v.as_str()),
    ])?;
    Some(session_id.to_string())
}

fn extract_result_error_message(event: &Value) -> Option<String> {
    if let Some(error) = event.get("error") {
        if let Some(message) = extract_text_from_value(error, 0) {
            return Some(message);
        }
        if let Some(message) = error
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(message.to_string());
        }
    }
    first_non_empty_str(&[event.get("message").and_then(|value| value.as_str())])
        .map(|value| value.to_string())
}

fn parse_gemini_event(workspace_id: &str, event: &Value) -> Option<EngineEvent> {
    let event_type = event.get("type").and_then(|v| v.as_str())?;
    match event_type {
        "message" => {
            let role = event
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if role == "user" || role == "system" {
                return None;
            }
            let text = first_non_empty_str(&[
                event.get("delta").and_then(|v| v.as_str()),
                event.get("text").and_then(|v| v.as_str()),
            ])
            .map(|s| s.to_string())
            .or_else(|| {
                event
                    .get("content")
                    .and_then(|value| extract_text_from_value(value, 0))
            })
            .or_else(|| extract_text_from_value(event, 0))?;
            if text.trim().is_empty() {
                return None;
            }
            Some(EngineEvent::TextDelta {
                workspace_id: workspace_id.to_string(),
                text,
            })
        }
        "tool_use" => {
            let tool_id = first_non_empty_str(&[
                event.get("tool_id").and_then(|v| v.as_str()),
                event.get("toolId").and_then(|v| v.as_str()),
                event.get("id").and_then(|v| v.as_str()),
            ])?
            .to_string();
            let tool_name = first_non_empty_str(&[
                event.get("tool_name").and_then(|v| v.as_str()),
                event.get("toolName").and_then(|v| v.as_str()),
                event.get("name").and_then(|v| v.as_str()),
            ])
            .unwrap_or("tool")
            .to_string();
            let input = event
                .get("parameters")
                .cloned()
                .or_else(|| event.get("args").cloned())
                .or_else(|| event.get("input").cloned());
            Some(EngineEvent::ToolStarted {
                workspace_id: workspace_id.to_string(),
                tool_id,
                tool_name,
                input,
            })
        }
        "tool_result" => {
            let tool_id = first_non_empty_str(&[
                event.get("tool_id").and_then(|v| v.as_str()),
                event.get("toolId").and_then(|v| v.as_str()),
                event.get("id").and_then(|v| v.as_str()),
            ])?
            .to_string();
            let status = event
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let error = first_non_empty_str(&[
                event.get("error").and_then(|v| v.as_str()),
                event.get("message").and_then(|v| v.as_str()),
            ])
            .map(|s| s.to_string())
            .or_else(|| {
                if status.contains("fail") || status.contains("error") {
                    Some("Tool execution failed".to_string())
                } else {
                    None
                }
            });
            let output = event
                .get("output")
                .cloned()
                .or_else(|| event.get("result").cloned())
                .or_else(|| event.get("response").cloned());
            Some(EngineEvent::ToolCompleted {
                workspace_id: workspace_id.to_string(),
                tool_id,
                tool_name: event
                    .get("tool_name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                output,
                error,
            })
        }
        "error" => {
            let message = first_non_empty_str(&[
                event.get("error").and_then(|v| v.as_str()),
                event.get("message").and_then(|v| v.as_str()),
            ])
            .map(|s| s.to_string())
            .unwrap_or_else(|| serde_json::to_string(event).unwrap_or_default());
            Some(EngineEvent::TurnError {
                workspace_id: workspace_id.to_string(),
                error: message,
                code: None,
            })
        }
        "result" => {
            let status = event
                .get("status")
                .and_then(|value| value.as_str())
                .map(|value| value.trim().to_ascii_lowercase())
                .filter(|value| !value.is_empty());
            let is_error_status = status
                .as_deref()
                .is_some_and(|value| matches!(value, "error" | "failed" | "cancelled" | "canceled"));
            let has_error_payload = event.get("error").is_some_and(|value| !value.is_null());
            if is_error_status || has_error_payload {
                let message = extract_result_error_message(event).unwrap_or_else(|| {
                    if let Some(value) = status.as_deref() {
                        format!("Gemini result status: {}", value)
                    } else {
                        "Gemini returned an error result.".to_string()
                    }
                });
                return Some(EngineEvent::TurnError {
                    workspace_id: workspace_id.to_string(),
                    error: message,
                    code: None,
                });
            }
            let result_text = event
                .get("text")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| {
                    event
                        .get("response")
                        .and_then(|value| extract_text_from_value(value, 0))
                })
                .or_else(|| {
                    event
                        .get("result")
                        .and_then(|value| extract_text_from_value(value, 0))
                });
            let result_payload = if let Some(text) = result_text {
                Some(json!({
                    "text": text,
                    "raw": event,
                }))
            } else {
                Some(event.clone())
            };
            Some(EngineEvent::TurnCompleted {
                workspace_id: workspace_id.to_string(),
                result: result_payload,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_gemini_event, GeminiSession};
    use serde_json::json;
    use super::EngineEvent;

    #[test]
    fn selected_auth_type_for_api_key_modes() {
        assert_eq!(
            GeminiSession::selected_auth_type_for_mode(Some("custom")),
            "gemini-api-key"
        );
        assert_eq!(
            GeminiSession::selected_auth_type_for_mode(Some("gemini_api_key")),
            "gemini-api-key"
        );
    }

    #[test]
    fn selected_auth_type_for_vertex_modes() {
        assert_eq!(
            GeminiSession::selected_auth_type_for_mode(Some("vertex_api_key")),
            "vertex-ai"
        );
        assert_eq!(
            GeminiSession::selected_auth_type_for_mode(Some("vertex_adc")),
            "vertex-ai"
        );
        assert_eq!(
            GeminiSession::selected_auth_type_for_mode(Some("vertex_service_account")),
            "vertex-ai"
        );
    }

    #[test]
    fn selected_auth_type_for_login_google_mode() {
        assert_eq!(
            GeminiSession::selected_auth_type_for_mode(Some("login_google")),
            "oauth-personal"
        );
        assert_eq!(
            GeminiSession::selected_auth_type_for_mode(Some("unknown")),
            "oauth-personal"
        );
    }

    #[test]
    fn parse_result_error_maps_to_turn_error() {
        let payload = json!({
            "type": "result",
            "status": "error",
            "error": {
                "message": "quota exceeded"
            }
        });
        let parsed = parse_gemini_event("workspace-1", &payload);
        match parsed {
            Some(EngineEvent::TurnError { error, .. }) => {
                assert!(error.contains("quota exceeded"));
            }
            _ => panic!("expected TurnError"),
        }
    }

    #[test]
    fn parse_result_success_maps_to_turn_completed() {
        let payload = json!({
            "type": "result",
            "status": "success",
            "text": "你好"
        });
        let parsed = parse_gemini_event("workspace-1", &payload);
        assert!(matches!(parsed, Some(EngineEvent::TurnCompleted { .. })));
    }
}
