use crate::errors::CommandResult;
use crate::ai::{
    AiChatRequest, AiChatResponse, AiModelInfo, AiProviderKind, AiProviderModelsResponse,
    AiProviderSettings, AiProviderSettingsResponse,
};
use crate::models::{
    BannedWord, WardScanHit, WardScanResponse,
};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

pub const CLOUD_DISCLOSURE_COPY: &str = r#"Cloud model disclosure

You are selecting a cloud model provider. Grimoire will send the prompt, relevant Vault excerpts, and active Canvas context needed for your Co-Writer request to the selected provider.

Your use of this model is subject to that provider's privacy policy, data processing terms, retention rules, and billing terms. Local-first mode remains available through Ollama, where supported by your machine.

Do not use a cloud provider for private, confidential, regulated, or sensitive manuscript material unless you are comfortable with that provider receiving it under its terms."#;

pub const PROVIDERS: [AiProviderKind; 4] = [
    AiProviderKind::Ollama,
    AiProviderKind::OpenAi,
    AiProviderKind::OpenAiCompatible,
    AiProviderKind::GoogleAiStudio,
];

pub fn cloud_provider(provider: &AiProviderKind) -> bool {
    !matches!(provider, AiProviderKind::Ollama)
}

pub fn get_setting(connection: &Connection, key: &str) -> CommandResult<Option<String>> {
    let result = connection.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |row| row.get(0),
    );

    match result {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(format!("Could not read setting {key}: {error}")),
    }
}

pub fn get_active_provider(connection: &Connection) -> CommandResult<AiProviderKind> {
    Ok(
        match get_setting(connection, "ai.activeProvider")?.as_deref() {
            Some("openAi") => AiProviderKind::OpenAi,
            Some("openAiCompatible") => AiProviderKind::OpenAiCompatible,
            Some("anthropic") => AiProviderKind::Anthropic,
            Some("googleAiStudio") => AiProviderKind::GoogleAiStudio,
            _ => AiProviderKind::Ollama,
        },
    )
}

pub fn provider_setting_key(provider: AiProviderKind, field: &str) -> String {
    format!("ai.provider.{}.{}", provider.as_key(), field)
}

pub fn provider_settings(
    connection: &Connection,
    provider: AiProviderKind,
) -> CommandResult<AiProviderSettings> {
    let selected_model = get_setting(connection, &provider_setting_key(provider, "selectedModel"))?
        .or_else(|| provider.default_model().map(ToString::to_string));
    let base_url = get_setting(connection, &provider_setting_key(provider, "baseUrl"))?
        .or_else(|| provider.default_base_url().map(ToString::to_string));
    let disclosure_accepted_at = get_setting(
        connection,
        &provider_setting_key(provider, "disclosureAcceptedAt"),
    )?;
    let api_key_present = if cloud_provider(&provider) {
        get_setting(connection, &provider_setting_key(provider, "apiKeyPresent"))?
            .as_deref()
            == Some("true")
    } else {
        false
    };

    Ok(AiProviderSettings {
        provider,
        display_name: provider.display_name().to_string(),
        base_url,
        selected_model,
        api_key_present,
        disclosure_accepted_at,
        enabled: provider == AiProviderKind::Ollama || api_key_present,
    })
}

pub fn list_ollama_models(connection: &Connection) -> CommandResult<AiProviderModelsResponse> {
    let base_url = "http://127.0.0.1:11434";
    match fetch_ollama_models(base_url) {
        Ok(models) => {
            let previous = get_setting(
                connection,
                &provider_setting_key(AiProviderKind::Ollama, "selectedModel"),
            )?
            .or_else(|| get_setting(connection, "ollama.selectedModel").ok().flatten());
            let model_names: Vec<String> = models.iter().map(|model| model.name.clone()).collect();
            let selected_model = select_ollama_model(previous, &model_names);
            if let Some(model) = selected_model.as_deref().filter(|_| model_names.len() == 1) {
                set_setting(
                    connection,
                    &provider_setting_key(AiProviderKind::Ollama, "selectedModel"),
                    model,
                )?;
            }
            let message = if models.is_empty() {
                "Ollama is running, but no local models are installed. Install one with `ollama pull <model>` and refresh models.".to_string()
            } else if selected_model.is_some() {
                "Ollama model ready.".to_string()
            } else {
                "Ollama found multiple local models. Choose one to enable Co-Writer requests.".to_string()
            };

            Ok(AiProviderModelsResponse {
                provider: AiProviderKind::Ollama,
                reachable: true,
                models,
                selected_model,
                message,
            })
        }
        Err(message) => Ok(AiProviderModelsResponse {
            provider: AiProviderKind::Ollama,
            reachable: false,
            models: Vec::new(),
            selected_model: None,
            message,
        }),
    }
}

pub fn select_ollama_model(previous: Option<String>, model_names: &[String]) -> Option<String> {
    if let Some(model) = previous.filter(|model| model_names.contains(model)) {
        Some(model)
    } else if model_names.len() == 1 {
        model_names.first().cloned()
    } else {
        None
    }
}

pub fn set_setting(connection: &Connection, key: &str, value: &str) -> CommandResult<()> {
    connection
        .execute(
            r#"
            INSERT INTO settings (key, value, updated_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
            "#,
            params![key, value, super::timestamp()],
        )
        .map_err(|error| format!("Could not write setting {key}: {error}"))?;
    Ok(())
}

fn secret_account(project_path: &str, provider: AiProviderKind) -> String {
    let mut project_hash: u64 = 1469598103934665603;
    for byte in project_path.as_bytes() {
        project_hash ^= *byte as u64;
        project_hash = project_hash.wrapping_mul(1099511628211);
    }
    format!("{:016x}-{}", project_hash, provider.as_key())
}

fn secret_service(provider: AiProviderKind) -> String {
    format!("com.witchdaddylabs.grimoire.{}", provider.as_key())
}

pub fn set_api_key_secret(
    project_path: &str,
    provider: AiProviderKind,
    api_key: &str,
) -> CommandResult<()> {
    security_framework::passwords::set_generic_password(
        &secret_service(provider),
        &secret_account(project_path, provider),
        api_key.as_bytes(),
    )
    .map_err(|error| format!("Could not save provider API key in macOS Keychain: {error}"))?;
    Ok(())
}

pub fn get_api_key_secret(
    project_path: &str,
    provider: AiProviderKind,
) -> CommandResult<Option<String>> {
    match security_framework::passwords::get_generic_password(
        &secret_service(provider),
        &secret_account(project_path, provider),
    ) {
        Ok(bytes) => {
            let key = String::from_utf8(bytes)
                .map_err(|_| "Provider API key in Keychain is not valid UTF-8".to_string())?
                .trim()
                .to_string();
            if key.is_empty() {
                Ok(None)
            } else {
                Ok(Some(key))
            }
        }
        Err(_) => Ok(None),
    }
}

pub fn delete_api_key_secret(project_path: &str, provider: AiProviderKind) -> CommandResult<()> {
    let _ = security_framework::passwords::delete_generic_password(
        &secret_service(provider),
        &secret_account(project_path, provider),
    );
    Ok(())
}

pub fn fetch_ollama_models(base_url: &str) -> Result<Vec<AiModelInfo>, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .map_err(|error| format!("Could not prepare Ollama client: {error}"))?;
    let response: Value = client
        .get(format!("{base_url}/api/tags"))
        .send()
        .map_err(|error| format!("Ollama is not reachable at {base_url}: {error}"))?
        .json()
        .map_err(|error| format!("Could not read Ollama model list: {error}"))?;
    let models = response
        .get("models")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let name = item.get("name")?.as_str()?.to_string();
                    Some(AiModelInfo {
                        name,
                        modified_at: item
                            .get("modified_at")
                            .and_then(Value::as_str)
                            .map(ToString::to_string),
                        size: item.get("size").and_then(Value::as_i64),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Ok(models)
}

pub fn chat_ollama(request: &AiChatRequest) -> CommandResult<AiChatResponse> {
    let content = if request.grounded_context.trim().is_empty() {
        request.prompt.clone()
    } else {
        format!(
            "{}\n\nUser request:\n{}",
            request.grounded_context, request.prompt
        )
    };
    let client = http_client(Duration::from_secs(120))?;
    let payload = json!({
        "model": request.model,
        "stream": false,
        "messages": [
            {
                "role": "user",
                "content": content
            }
        ]
    });
    let response: Value = client
        .post("http://127.0.0.1:11434/api/chat")
        .json(&payload)
        .send()
        .map_err(|error| format!("Could not reach Ollama chat endpoint: {error}"))?
        .json()
        .map_err(|error| format!("Could not read Ollama chat response: {error}"))?;
    let text = response
        .get("message")
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if text.is_empty() {
        return Err("Ollama returned an empty response.".to_string());
    }
    Ok(AiChatResponse {
        provider: AiProviderKind::Ollama,
        model: request.model.clone(),
        text,
        request_id: None,
        input_tokens: None,
        output_tokens: None,
    })
}

pub fn chat_openai_compatible(
    connection: &Connection,
    request: &AiChatRequest,
) -> CommandResult<AiChatResponse> {
    let api_key = get_api_key_secret(&request.project_path, request.provider)?
        .ok_or("Add an API key for this cloud provider before sending a Co-Writer request.")?;
    let base_url = provider_settings(connection, request.provider)?
        .base_url
        .ok_or("Set a base URL for this OpenAI-compatible provider.")?;
    let url = if request.provider == AiProviderKind::OpenAi {
        format!("{}/v1/chat/completions", base_url.trim_end_matches('/'))
    } else {
        openai_compatible_url(&base_url)
    };
    let client = http_client(Duration::from_secs(120))?;
    let payload = json!({
        "model": request.model,
        "messages": [
            {
                "role": "system",
                "content": "Use the provided Grimoire context. Cite local sources when present. Do not claim access to unavailable context."
            },
            {
                "role": "user",
                "content": format!("{}\n\nUser request:\n{}", request.grounded_context, request.prompt)
            }
        ]
    });
    let response = client
        .post(url)
        .bearer_auth(api_key)
        .json(&payload)
        .send()
        .map_err(|error| format!("Cloud provider request failed: {error}"))?;
    let status = response.status().as_u16();
    let raw = response
        .text()
        .map_err(|error| format!("Could not read cloud provider response: {error}"))?;
    if !(200..300).contains(&status) {
        return Err(cloud_http_error("Cloud provider", status));
    }
    let response: Value = serde_json::from_str(&raw)
        .map_err(|error| format!("Could not parse cloud provider response: {error}"))?;
    let text =
        openai_chat_text(&response).ok_or("Cloud provider returned an empty response.".to_string())?;
    Ok(AiChatResponse {
        provider: request.provider,
        model: request.model.clone(),
        text,
        request_id: response
            .get("id")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        input_tokens: response
            .get("usage")
            .and_then(|usage| usage.get("prompt_tokens"))
            .and_then(Value::as_i64),
        output_tokens: response
            .get("usage")
            .and_then(|usage| usage.get("completion_tokens"))
            .and_then(Value::as_i64),
    })
}

pub fn chat_anthropic(
    connection: &Connection,
    request: &AiChatRequest,
) -> CommandResult<AiChatResponse> {
    let api_key = get_api_key_secret(&request.project_path, request.provider)?
        .ok_or("Add an API key for Anthropic before sending a Co-Writer request.")?;
    let base_url = provider_settings(connection, request.provider)?
        .base_url
        .unwrap_or_else(|| "https://api.anthropic.com".to_string());
    let client = http_client(Duration::from_secs(120))?;
    let payload = json!({
        "model": request.model,
        "max_tokens": 1200,
        "system": "Use the provided Grimoire context. Cite local sources when present. Do not claim access to unavailable context.",
        "messages": [
            {
                "role": "user",
                "content": format!("{}\n\nUser request:\n{}", request.grounded_context, request.prompt)
            }
        ]
    });
    let mut request_builder =
        client.post(format!("{}/v1/messages", base_url.trim_end_matches('/')));
    for (name, value) in anthropic_headers(&api_key) {
        request_builder = request_builder.header(name, value);
    }
    let response = request_builder
        .json(&payload)
        .send()
        .map_err(|error| format!("Anthropic request failed: {error}"))?;
    let status = response.status().as_u16();
    let raw = response
        .text()
        .map_err(|error| format!("Could not read Anthropic response: {error}"))?;
    if !(200..300).contains(&status) {
        return Err(cloud_http_error("Anthropic", status));
    }
    let response: Value = serde_json::from_str(&raw)
        .map_err(|error| format!("Could not parse Anthropic response: {error}"))?;
    let text = anthropic_chat_text(&response).ok_or("Anthropic returned an empty response.".to_string())?;
    Ok(AiChatResponse {
        provider: request.provider,
        model: request.model.clone(),
        text,
        request_id: response
            .get("id")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        input_tokens: response
            .get("usage")
            .and_then(|usage| usage.get("input_tokens"))
            .and_then(Value::as_i64),
        output_tokens: response
            .get("usage")
            .and_then(|usage| usage.get("output_tokens"))
            .and_then(Value::as_i64),
    })
}

pub fn chat_google(connection: &Connection, request: &AiChatRequest) -> CommandResult<AiChatResponse> {
    let api_key = get_api_key_secret(&request.project_path, request.provider)?
        .ok_or("Add an API key for Google AI Studio before sending a Co-Writer request.")?;
    let base_url = provider_settings(connection, request.provider)?
        .base_url
        .unwrap_or_else(|| "https://generativelanguage.googleapis.com".to_string());
    let client = http_client(Duration::from_secs(120))?;
    let payload = json!({
        "contents": [
            {
                "role": "user",
                "parts": [
                    {
                        "text": format!("Use the provided Grimoire context. Cite local sources when present. Do not claim access to unavailable context.\n\n{}\n\nUser request:\n{}", request.grounded_context, request.prompt)
                    }
                ]
            }
        ]
    });
    let url = if base_url.trim_end_matches('/') == "https://generativelanguage.googleapis.com" {
        gemini_generate_content_url(&request.model)
    } else {
        format!(
            "{}/v1beta/models/{}:generateContent",
            base_url.trim_end_matches('/'),
            request.model
        )
    };
    let response = client
        .post(url)
        .header("x-goog-api-key", api_key)
        .json(&payload)
        .send()
        .map_err(|error| format!("Google AI Studio request failed: {error}"))?;
    let status = response.status().as_u16();
    let raw = response
        .text()
        .map_err(|error| format!("Could not read Google AI Studio response: {error}"))?;
    if !(200..300).contains(&status) {
        return Err(cloud_http_error("Google AI Studio", status));
    }
    let response: Value = serde_json::from_str(&raw)
        .map_err(|error| format!("Could not parse Google AI Studio response: {error}"))?;
    let text =
        gemini_chat_text(&response).ok_or("Google AI Studio returned an empty response.".to_string())?;
    Ok(AiChatResponse {
        provider: request.provider,
        model: request.model.clone(),
        text,
        request_id: None,
        input_tokens: response
            .get("usageMetadata")
            .and_then(|usage| usage.get("promptTokenCount"))
            .and_then(Value::as_i64),
        output_tokens: response
            .get("usageMetadata")
            .and_then(|usage| usage.get("candidatesTokenCount"))
            .and_then(Value::as_i64),
    })
}

fn http_client(timeout: Duration) -> CommandResult<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|error| format!("Could not prepare HTTP client: {error}"))
}

fn cloud_http_error(provider: &str, status: u16) -> String {
    match status {
        401 | 403 => format!("{provider} rejected the API key or account permissions."),
        404 => format!("{provider} could not find that model or endpoint."),
        408 | 429 => format!("{provider} is rate limited, over quota, or timed out."),
        500..=599 => format!("{provider} returned a temporary server error ({status})."),
        _ => format!("{provider} request failed with HTTP status {status}."),
    }
}

fn openai_compatible_url(base_url: &str) -> String {
    format!("{}/v1/chat/completions", base_url.trim_end_matches('/'))
}

pub fn openai_chat_text(response: &Value) -> Option<String> {
    response
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
}

pub fn anthropic_headers(api_key: &str) -> Vec<(String, String)> {
    vec![
        ("x-api-key".to_string(), api_key.to_string()),
        ("anthropic-version".to_string(), "2023-06-01".to_string()),
        ("content-type".to_string(), "application/json".to_string()),
    ]
}

pub fn anthropic_chat_text(response: &Value) -> Option<String> {
    response
        .get("content")
        .and_then(Value::as_array)
        .map(|blocks| {
            blocks
                .iter()
                .filter_map(|block| block.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

pub fn gemini_generate_content_url(model: &str) -> String {
    format!("https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent")
}

pub fn gemini_chat_text(response: &Value) -> Option<String> {
    response
        .get("candidates")
        .and_then(Value::as_array)
        .and_then(|candidates| candidates.first())
        .and_then(|candidate| candidate.get("content"))
        .and_then(|content| content.get("parts"))
        .and_then(Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

// -- Ward scanning (moved here from main.rs, re-exported via db.rs) --

pub fn seed_default_banned_words(connection: &Connection) -> CommandResult<()> {
    let defaults = [
        ("ward_default_very", "very", "warn"),
        ("ward_default_really", "really", "warn"),
        ("ward_default_suddenly", "suddenly", "warn"),
        ("ward_default_somehow", "somehow", "warn"),
        ("ward_default_actually", "actually", "warn"),
        ("ward_default_utilize", "utilize", "warn"),
        ("ward_default_delve", "delve", "warn"),
        ("ward_default_tapestry", "tapestry", "warn"),
    ];
    let now = super::timestamp();
    for (id, value, severity) in defaults {
        connection
            .execute(
                r#"
                INSERT OR IGNORE INTO banned_words
                  (id, value, severity, is_default, created_at, updated_at)
                VALUES (?1, ?2, ?3, 1, ?4, ?4)
                "#,
                params![id, value, severity, now],
            )
            .map_err(|error| format!("Could not seed default ward phrase: {error}"))?;
    }
    Ok(())
}

pub fn scan_wards(words: &[BannedWord], text: &str) -> WardScanResponse {
    let lower_text = text.to_lowercase();
    let mut hits = Vec::new();
    for word in words {
        let needle = word.value.to_lowercase();
        if needle.is_empty() {
            continue;
        }
        let count = lower_text.matches(&needle).count();
        if count > 0 {
            hits.push(WardScanHit {
                id: word.id.clone(),
                value: word.value.clone(),
                severity: word.severity.clone(),
                count,
            });
        }
    }

    let has_blocking_hits = hits.iter().any(|hit| hit.severity == "block");
    WardScanResponse {
        hits,
        has_blocking_hits,
    }
}

// -- Helper: search result confidence --

pub fn confidence_for_score(score: f64) -> String {
    if score >= 8.0 {
        "high".to_string()
    } else if score >= 3.0 {
        "medium".to_string()
    } else if score > 0.0 {
        "low".to_string()
    } else {
        "none".to_string()
    }
}

pub fn aggregate_confidence(results: &[crate::models::SearchChunkResult]) -> String {
    results
        .first()
        .map(|result| result.confidence.clone())
        .unwrap_or_else(|| "none".to_string())
}
