use super::*;

pub(super) fn extract_json_object_from_text(input: &str) -> Option<String> {
    let start = input.find('{')?;
    let end = input.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(input[start..=end].to_string())
}

pub(super) fn extract_first_url(input: &str) -> Option<String> {
    let clean = strip_ansi_codes(input);
    for token in clean.split_whitespace() {
        if token.starts_with("http://") || token.starts_with("https://") {
            return Some(
                token
                    .trim_matches(|c: char| c == ')' || c == ']' || c == ',' || c == '.')
                    .to_string(),
            );
        }
    }
    None
}

pub(super) fn parse_imported_session_id(output: &str) -> Option<String> {
    for line in strip_ansi_codes(output).lines() {
        let trimmed = line.trim();
        let Some((_, right)) = trimmed.split_once("Imported session:") else {
            continue;
        };
        let candidate = right.split_whitespace().next().unwrap_or_default().trim();
        if !candidate.is_empty() {
            return Some(candidate.to_string());
        }
    }
    None
}

pub(super) fn parse_json_value(output: &str) -> Option<Value> {
    let trimmed = strip_ansi_codes(output).trim().to_string();
    if trimmed.is_empty() {
        return None;
    }
    serde_json::from_str::<Value>(&trimmed).ok()
}

pub(super) fn parse_opencode_help_commands(stdout: &str) -> Vec<OpenCodeCommandEntry> {
    let clean = strip_ansi_codes(stdout);
    let mut entries = Vec::new();
    let mut in_commands = false;
    for raw in clean.lines() {
        let line = raw.trim_end();
        if line.trim() == "Commands:" {
            in_commands = true;
            continue;
        }
        if in_commands && line.trim().is_empty() {
            break;
        }
        if !in_commands {
            continue;
        }
        let trimmed = line.trim_start();
        if !trimmed.starts_with("opencode ") {
            continue;
        }
        let without_prefix = trimmed.trim_start_matches("opencode ").trim();
        if without_prefix.is_empty() {
            continue;
        }
        let mut chunks = without_prefix.splitn(2, "  ");
        let command_name = chunks.next().unwrap_or_default().trim();
        let description = chunks.next().map(str::trim).filter(|s| !s.is_empty());
        let name = command_name
            .split_whitespace()
            .take_while(|token| !token.starts_with('[') && !token.starts_with('<'))
            .collect::<Vec<_>>()
            .join(" ");
        if name.is_empty() {
            continue;
        }
        entries.push(OpenCodeCommandEntry {
            name: name.replace(' ', ":"),
            description: description.map(ToOwned::to_owned),
            argument_hint: None,
        });
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries.dedup_by(|a, b| a.name == b.name);
    entries
}

pub(super) fn parse_opencode_agent_list(stdout: &str) -> Vec<OpenCodeAgentEntry> {
    let clean = strip_ansi_codes(stdout);
    let mut entries = Vec::new();
    for raw in clean.lines() {
        let line = raw.trim();
        if line.is_empty()
            || line.starts_with('[')
            || line.starts_with('{')
            || line.starts_with('"')
        {
            continue;
        }
        if let Some(first) = line.chars().next() {
            if matches!(first, '{' | '}' | '[' | ']' | ',') {
                continue;
            }
        }
        if line.contains(':') || line.starts_with(']') {
            continue;
        }
        let (id, flag_part) = if let Some((left, right)) = line.split_once('(') {
            (left.trim(), Some(right.trim_end_matches(')').trim()))
        } else {
            (line, None)
        };
        if id.is_empty() || !id.chars().any(|ch| ch.is_alphanumeric()) {
            continue;
        }
        let is_primary = flag_part
            .map(|flag| flag.eq_ignore_ascii_case("primary"))
            .unwrap_or(false);
        entries.push(OpenCodeAgentEntry {
            id: id.to_string(),
            description: flag_part
                .filter(|flag| !flag.eq_ignore_ascii_case("primary"))
                .map(ToOwned::to_owned),
            is_primary,
        });
    }
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    entries.dedup_by(|a, b| a.id == b.id);
    entries
}

pub(super) fn parse_opencode_debug_config_agents(stdout: &str) -> Vec<OpenCodeAgentEntry> {
    let clean = strip_ansi_codes(stdout);
    let Ok(value) = serde_json::from_str::<Value>(clean.trim()) else {
        return Vec::new();
    };
    let Some(agent_map) = value.get("agent").and_then(|item| item.as_object()) else {
        return Vec::new();
    };

    let mut entries = Vec::new();
    for (id, config) in agent_map {
        let trimmed_id = id.trim();
        if trimmed_id.is_empty() || !trimmed_id.chars().any(|ch| ch.is_alphanumeric()) {
            continue;
        }
        let mode = config
            .get("mode")
            .and_then(|item| item.as_str())
            .map(|item| item.trim().to_lowercase())
            .unwrap_or_default();
        let is_primary = mode == "primary";
        let description = config
            .get("description")
            .and_then(|item| item.as_str())
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(ToOwned::to_owned);
        entries.push(OpenCodeAgentEntry {
            id: trimmed_id.to_string(),
            description,
            is_primary,
        });
    }
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    entries.dedup_by(|a, b| a.id == b.id);
    entries
}

pub(super) fn merge_opencode_agents(
    mut primary: Vec<OpenCodeAgentEntry>,
    supplemental: Vec<OpenCodeAgentEntry>,
) -> Vec<OpenCodeAgentEntry> {
    let mut merged: HashMap<String, OpenCodeAgentEntry> = HashMap::new();
    for item in primary.drain(..) {
        merged.insert(item.id.clone(), item);
    }
    for item in supplemental {
        merged
            .entry(item.id.clone())
            .and_modify(|existing| {
                existing.is_primary = existing.is_primary || item.is_primary;
                if existing.description.is_none() {
                    existing.description = item.description.clone();
                }
            })
            .or_insert(item);
    }
    let mut out = merged.into_values().collect::<Vec<_>>();
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

pub(super) fn derive_provider_from_model(model: Option<&str>) -> Option<String> {
    let raw = model?.trim();
    if raw.is_empty() {
        return None;
    }
    if let Some((provider, _)) = raw.split_once('/') {
        let key = provider.trim().to_lowercase();
        return if key.is_empty() { None } else { Some(key) };
    }

    let key = raw.to_lowercase();
    if key.starts_with("gpt-")
        || key.starts_with("o1")
        || key.starts_with("o3")
        || key.starts_with("o4")
        || key.starts_with("codex")
    {
        return Some("openai".to_string());
    }
    if key.starts_with("claude-") {
        return Some("anthropic".to_string());
    }
    if key.starts_with("gemini-") {
        return Some("google".to_string());
    }
    if key.contains("minimax") {
        return Some("minimax-cn-coding-plan".to_string());
    }
    None
}

pub(super) fn normalize_provider_key(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

fn tokenize_provider_key(value: &str) -> Vec<String> {
    value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter_map(|token| {
            let trimmed = token.trim().to_lowercase();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
        .collect()
}

fn is_provider_noise_token(token: &str) -> bool {
    matches!(
        token,
        "cn" | "us" | "eu" | "jp" | "uk" | "ap" | "sg" | "global"
    )
}

pub(super) fn provider_keys_match(target: &str, candidate: &str) -> bool {
    let target_key = normalize_provider_key(target);
    let candidate_key = normalize_provider_key(candidate);
    if target_key == candidate_key
        || target_key.contains(&candidate_key)
        || candidate_key.contains(&target_key)
    {
        return true;
    }

    let target_tokens = tokenize_provider_key(target)
        .into_iter()
        .filter(|token| !is_provider_noise_token(token))
        .collect::<Vec<_>>();
    let candidate_tokens = tokenize_provider_key(candidate)
        .into_iter()
        .filter(|token| !is_provider_noise_token(token))
        .collect::<Vec<_>>();
    if target_tokens.is_empty() || candidate_tokens.is_empty() {
        return false;
    }
    candidate_tokens
        .iter()
        .all(|token| target_tokens.iter().any(|item| item == token))
        || target_tokens
            .iter()
            .all(|token| candidate_tokens.iter().any(|item| item == token))
}

pub(super) fn parse_opencode_auth_providers(stdout: &str) -> Vec<String> {
    let clean = strip_ansi_codes(stdout);
    let mut providers: Vec<String> = Vec::new();
    for raw in clean.lines() {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('┌')
            || trimmed.starts_with('│')
            || trimmed.starts_with('└')
            || trimmed.starts_with("Credentials")
        {
            continue;
        }
        let line = trimmed
            .trim_start_matches(|c: char| matches!(c, '●' | '○' | '•' | '-' | '*'))
            .trim();
        if line.is_empty() || line.starts_with('~') {
            continue;
        }
        let mut name_parts = Vec::new();
        for token in line.split_whitespace() {
            let token_lower = token.to_lowercase();
            if token_lower == "oauth" || token_lower == "api" {
                break;
            }
            if token.starts_with('(') {
                break;
            }
            name_parts.push(token);
        }
        if name_parts.is_empty() {
            continue;
        }
        providers.push(name_parts.join(" ").to_lowercase());
    }
    providers.sort();
    providers.dedup();
    providers
}

pub(super) fn parse_opencode_mcp_servers(stdout: &str) -> Vec<OpenCodeMcpServerState> {
    let clean = strip_ansi_codes(stdout);
    if clean.to_lowercase().contains("no mcp servers configured") {
        return Vec::new();
    }
    let mut servers = Vec::new();
    for raw in clean.lines() {
        let trimmed = raw.trim();
        if trimmed.is_empty()
            || trimmed.starts_with('┌')
            || trimmed.starts_with('│')
            || trimmed.starts_with('└')
            || trimmed.to_lowercase().contains("mcp servers")
        {
            continue;
        }
        let line = trimmed
            .trim_start_matches(|c: char| matches!(c, '●' | '○' | '•' | '-' | '*'))
            .trim();
        if line.is_empty() {
            continue;
        }
        let mut tokens = line.split_whitespace();
        let name = tokens.next().unwrap_or_default().trim().to_string();
        if name.is_empty() {
            continue;
        }
        let lower_line = line.to_lowercase();
        let status = if lower_line.contains("connected") {
            Some("connected".to_string())
        } else if lower_line.contains("auth") {
            Some("auth-required".to_string())
        } else if lower_line.contains("error") || lower_line.contains("failed") {
            Some("error".to_string())
        } else if lower_line.contains("running") {
            Some("running".to_string())
        } else {
            Some("unknown".to_string())
        };
        let permission_hint = if lower_line.contains("file") {
            Some("filesystem".to_string())
        } else if lower_line.contains("web") || lower_line.contains("network") {
            Some("network".to_string())
        } else if lower_line.contains("git") {
            Some("git".to_string())
        } else {
            None
        };
        let enabled = !lower_line.contains("disabled");
        servers.push(OpenCodeMcpServerState {
            name,
            enabled,
            status,
            permission_hint,
        });
    }
    servers.sort_by(|a, b| a.name.cmp(&b.name));
    servers.dedup_by(|a, b| a.name == b.name);
    servers
}

pub(super) fn resolve_session_id_from_thread(thread_id: Option<&str>) -> Option<String> {
    let raw = thread_id?.trim();
    if raw.starts_with("opencode:") {
        let session = raw.trim_start_matches("opencode:").trim();
        if !session.is_empty() {
            return Some(session.to_string());
        }
    }
    None
}

fn parse_opencode_date_token(input: &str) -> Option<NaiveDate> {
    let token = input.trim();
    if token.is_empty() {
        return None;
    }
    NaiveDate::parse_from_str(token, "%m/%d/%Y")
        .ok()
        .or_else(|| NaiveDate::parse_from_str(token, "%-m/%-d/%Y").ok())
        .or_else(|| NaiveDate::parse_from_str(token, "%Y-%m-%d").ok())
}

fn parse_opencode_time_token(input: &str) -> Option<NaiveTime> {
    let token = input.trim();
    if token.is_empty() {
        return None;
    }
    NaiveTime::parse_from_str(token, "%I:%M %p")
        .ok()
        .or_else(|| NaiveTime::parse_from_str(token, "%-I:%M %p").ok())
        .or_else(|| NaiveTime::parse_from_str(token, "%H:%M").ok())
}

fn parse_relative_updated_at_millis(input: &str, now: DateTime<Local>) -> Option<i64> {
    let label = input.trim().to_lowercase();
    if label.is_empty() {
        return None;
    }
    if label == "just now" || label == "刚刚" {
        return Some(now.timestamp_millis());
    }

    let parse_amount = |text: &str, suffix: &str| -> Option<i64> {
        text.strip_suffix(suffix)?.trim().parse::<i64>().ok()
    };
    let apply_seconds = |seconds: i64| -> Option<i64> {
        Some((now - ChronoDuration::seconds(seconds.max(0))).timestamp_millis())
    };

    if let Some(value) = parse_amount(&label, "秒前") {
        return apply_seconds(value);
    }
    if let Some(value) = parse_amount(&label, "分钟前").or_else(|| parse_amount(&label, "分前"))
    {
        return apply_seconds(value * 60);
    }
    if let Some(value) = parse_amount(&label, "小时前").or_else(|| parse_amount(&label, "小時前"))
    {
        return apply_seconds(value * 3600);
    }
    if let Some(value) = parse_amount(&label, "天前") {
        return apply_seconds(value * 86_400);
    }
    if let Some(value) = parse_amount(&label, "周前") {
        return apply_seconds(value * 604_800);
    }

    let mut compact_value: Option<i64> = None;
    let mut compact_unit: Option<String> = None;
    let mut number_chars = String::new();
    for ch in label.chars() {
        if ch.is_ascii_digit() {
            number_chars.push(ch);
            continue;
        }
        compact_value = number_chars.parse::<i64>().ok();
        compact_unit = Some(label[number_chars.len()..].trim().to_string());
        break;
    }
    if let (Some(value), Some(unit)) = (compact_value, compact_unit) {
        if unit.starts_with("s") {
            return apply_seconds(value);
        }
        if unit.starts_with('m') {
            return apply_seconds(value * 60);
        }
        if unit.starts_with('h') {
            return apply_seconds(value * 3600);
        }
        if unit.starts_with('d') {
            return apply_seconds(value * 86_400);
        }
        if unit.starts_with('w') {
            return apply_seconds(value * 604_800);
        }
    }
    None
}

pub(super) fn parse_opencode_updated_at(updated_label: &str, now: DateTime<Local>) -> Option<i64> {
    let trimmed = updated_label.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(parsed) = DateTime::parse_from_rfc3339(trimmed) {
        return Some(parsed.with_timezone(&Local).timestamp_millis());
    }

    let normalized = trimmed.replace('•', "·");
    let mut parts = normalized
        .split('·')
        .map(str::trim)
        .filter(|part| !part.is_empty());
    let first = parts.next();
    let second = parts.next();

    if let (Some(time_part), Some(date_part)) = (first, second) {
        if let (Some(time), Some(date)) = (
            parse_opencode_time_token(time_part),
            parse_opencode_date_token(date_part),
        ) {
            let local_result = Local.from_local_datetime(&NaiveDateTime::new(date, time));
            if let Some(value) = local_result.single().or_else(|| local_result.earliest()) {
                return Some(value.timestamp_millis());
            }
        }
    }

    if let Some(single_part) = first {
        if let Some(time) = parse_opencode_time_token(single_part) {
            let today = now.date_naive();
            let local_result = Local.from_local_datetime(&NaiveDateTime::new(today, time));
            if let Some(mut value) = local_result.single().or_else(|| local_result.earliest()) {
                if value > now + ChronoDuration::minutes(5) {
                    value = value - ChronoDuration::days(1);
                }
                return Some(value.timestamp_millis());
            }
        }
        if let Some(date) = parse_opencode_date_token(single_part) {
            let local_result = Local.from_local_datetime(&NaiveDateTime::new(date, NaiveTime::MIN));
            if let Some(value) = local_result.single().or_else(|| local_result.earliest()) {
                return Some(value.timestamp_millis());
            }
        }
    }

    parse_relative_updated_at_millis(trimmed, now)
}

pub(super) fn parse_opencode_session_list(stdout: &str) -> Vec<OpenCodeSessionEntry> {
    let clean = strip_ansi_codes(stdout);
    let now = Local::now();
    let mut entries = Vec::new();
    for raw in clean.lines() {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with("Session ID") || trimmed.starts_with('─') {
            continue;
        }
        let Some(session_id_end) = trimmed.find(char::is_whitespace) else {
            continue;
        };
        let session_id = trimmed[..session_id_end].trim();
        if session_id.is_empty() || !session_id.starts_with("ses_") {
            continue;
        }
        let rest = trimmed[session_id_end..].trim_start();
        if rest.is_empty() {
            continue;
        }
        let split_idx = rest.rfind("  ");
        let (title, updated) = if let Some(index) = split_idx {
            let title_text = rest[..index].trim();
            let updated_text = rest[index..].trim();
            (
                if title_text.is_empty() {
                    "Untitled"
                } else {
                    title_text
                },
                updated_text,
            )
        } else {
            (rest, "")
        };
        entries.push(OpenCodeSessionEntry {
            session_id: session_id.to_string(),
            title: title.to_string(),
            updated_label: updated.to_string(),
            updated_at: parse_opencode_updated_at(updated, now),
        });
    }
    entries
}
