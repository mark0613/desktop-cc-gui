use serde_json::Value;

pub(crate) fn extract_error_message(error_payload: Option<&Value>, fallback: &str) -> String {
    let Some(payload) = error_payload else {
        return fallback.to_string();
    };

    payload
        .as_str()
        .or_else(|| payload.get("message").and_then(|value| value.as_str()))
        .or_else(|| payload.get("error").and_then(|value| value.as_str()))
        .or_else(|| {
            payload
                .get("error")
                .and_then(|value| value.get("message"))
                .and_then(|value| value.as_str())
        })
        .unwrap_or(fallback)
        .to_string()
}
