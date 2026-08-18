use crate::errors::CommandResult;
use crate::helpers::timestamp;
use crate::ai::{
    AiChatRequest, AiChatResponse, AiGenerationRequest, AiModelInfo, AiProviderKind,
    AiProviderModelsResponse,
};
use crate::models::{
    BannedWord, WardScanHit, WardScanResponse,
};
use crate::settings::{
    get_setting, provider_setting_key, provider_settings,
    set_setting,
};
use keyring::{Entry, Error as KeyringError};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::time::Duration;



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

fn secret_entry(project_path: &str, provider: AiProviderKind) -> CommandResult<Entry> {
    Entry::new(
        &secret_service(provider),
        &secret_account(project_path, provider),
    )
    .map_err(|error| format!("Could not open the secure credential store: {error}"))
}

pub fn set_api_key_secret(
    project_path: &str,
    provider: AiProviderKind,
    api_key: &str,
) -> CommandResult<()> {
    secret_entry(project_path, provider)?
        .set_password(api_key)
        .map_err(|error| format!("Could not save provider API key in the credential store: {error}"))
}

pub fn get_api_key_secret(
    project_path: &str,
    provider: AiProviderKind,
) -> CommandResult<Option<String>> {
    match secret_entry(project_path, provider)?.get_password() {
        Ok(key) => {
            let key = key.trim().to_string();
            if key.is_empty() {
                Ok(None)
            } else {
                Ok(Some(key))
            }
        }
        Err(KeyringError::NoEntry) => Ok(None),
        Err(error) => Err(format!("Could not read provider API key: {error}")),
    }
}

pub fn delete_api_key_secret(project_path: &str, provider: AiProviderKind) -> CommandResult<()> {
    match secret_entry(project_path, provider)?.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(error) => Err(format!("Could not delete provider API key: {error}")),
    }
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
        stop_reason: None,
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
        stop_reason: None,
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
        stop_reason: None,
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
        stop_reason: None,
    })
}

fn http_client(timeout: Duration) -> CommandResult<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|error| format!("Could not prepare HTTP client: {error}"))
}

// ── Structured generation (Story Plan regeneration pipeline) ──
//
// Unlike the chat_* functions above (fixed Co-Writer system prompt, no
// temperature control), generation takes an explicit system prompt and
// sampling temperature so the candidate loop can vary sampling while
// locked-beat constraints ride in the system prompt.

/// Provider-safe temperature: every supported backend accepts roughly 0..1,
/// with OpenAI extending to 2. Clamp defensively — a rogue temperature from
/// the frontend must not 400 the whole regeneration run.
fn clamp_temperature(temperature: f64, provider: AiProviderKind) -> f64 {
    let upper = if provider == AiProviderKind::OpenAi {
        2.0
    } else {
        1.0
    };
    if !temperature.is_finite() {
        return 0.7;
    }
    temperature.clamp(0.0, upper)
}

pub fn generate_ollama(request: &AiGenerationRequest) -> CommandResult<AiChatResponse> {
    let client = http_client(Duration::from_secs(180))?;
    let payload = json!({
        "model": request.model,
        "stream": false,
        "options": { "temperature": clamp_temperature(request.temperature, AiProviderKind::Ollama) },
        "messages": [
            { "role": "system", "content": request.system_prompt },
            { "role": "user", "content": request.user_prompt }
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
        stop_reason: ollama_stop_reason(&response),
    })
}

/// Models that reject non-default sampling parameters on the OpenAI
/// Chat Completions API: the GPT-5 family and the o-series reasoning models.
/// Sending a non-default `temperature` to them fails the whole request, so
/// the field must be omitted rather than sent (Codex P1 on PR #25 — the
/// default OpenAI model is gpt-5-mini, which would make every regeneration
/// fail before producing a single candidate).
fn model_supports_temperature(model: &str) -> bool {
    let normalized = model.to_lowercase();
    !(normalized.starts_with("gpt-5")
        || normalized == "o1"
        || normalized.starts_with("o1-")
        || normalized == "o3"
        || normalized.starts_with("o3-")
        || normalized.starts_with("o4-"))
}

pub fn generate_openai_compatible(
    connection: &Connection,
    request: &AiGenerationRequest,
) -> CommandResult<AiChatResponse> {
    let api_key = get_api_key_secret(&request.project_path, request.provider)?
        .ok_or("Add an API key for this cloud provider before regenerating story content.")?;
    let base_url = provider_settings(connection, request.provider)?
        .base_url
        .ok_or("Set a base URL for this OpenAI-compatible provider.")?;
    let url = if request.provider == AiProviderKind::OpenAi {
        format!("{}/v1/chat/completions", base_url.trim_end_matches('/'))
    } else {
        openai_compatible_url(&base_url)
    };
    let client = http_client(Duration::from_secs(180))?;
    // GPT-5 / o-series models reject non-default sampling parameters — send
    // the field only when the model supports it (Codex P1 on PR #25).
    let payload = if model_supports_temperature(&request.model) {
        json!({
            "model": request.model,
            "temperature": clamp_temperature(request.temperature, request.provider),
            "messages": [
                { "role": "system", "content": request.system_prompt },
                { "role": "user", "content": request.user_prompt }
            ]
        })
    } else {
        json!({
            "model": request.model,
            "messages": [
                { "role": "system", "content": request.system_prompt },
                { "role": "user", "content": request.user_prompt }
            ]
        })
    };
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
    let text = openai_chat_text(&response)
        .ok_or("Cloud provider returned an empty response.".to_string())?;
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
        stop_reason: openai_stop_reason(&response),
    })
}

pub fn generate_anthropic(
    connection: &Connection,
    request: &AiGenerationRequest,
) -> CommandResult<AiChatResponse> {
    let api_key = get_api_key_secret(&request.project_path, request.provider)?
        .ok_or("Add an API key for Anthropic before regenerating story content.")?;
    let base_url = provider_settings(connection, request.provider)?
        .base_url
        .unwrap_or_else(|| "https://api.anthropic.com".to_string());
    let client = http_client(Duration::from_secs(180))?;
    // Anthropic requires max_tokens; the caller decides the cap.
    let payload = json!({
        "model": request.model,
        "max_tokens": request.max_tokens,
        "temperature": clamp_temperature(request.temperature, AiProviderKind::Anthropic),
        "system": request.system_prompt,
        "messages": [
            { "role": "user", "content": request.user_prompt }
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
        stop_reason: anthropic_stop_reason(&response),
    })
}

pub fn generate_google(
    connection: &Connection,
    request: &AiGenerationRequest,
) -> CommandResult<AiChatResponse> {
    let api_key = get_api_key_secret(&request.project_path, request.provider)?
        .ok_or("Add an API key for Google AI Studio before regenerating story content.")?;
    let base_url = provider_settings(connection, request.provider)?
        .base_url
        .unwrap_or_else(|| "https://generativelanguage.googleapis.com".to_string());
    let client = http_client(Duration::from_secs(180))?;
    let payload = json!({
        "systemInstruction": {
            "parts": [{ "text": request.system_prompt }]
        },
        "contents": [
            {
                "role": "user",
                "parts": [{ "text": request.user_prompt }]
            }
        ],
        "generationConfig": {
            "temperature": clamp_temperature(request.temperature, AiProviderKind::GoogleAiStudio)
        }
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
        stop_reason: gemini_stop_reason(&response),
    })
}

/// Route a generation request to the right provider backend. Mirrors the
/// `ai_chat_inner` dispatch so every configured provider works for
/// regeneration, not just the Co-Writer path.
pub fn generate_story_text(
    connection: &Connection,
    request: &AiGenerationRequest,
) -> CommandResult<AiChatResponse> {
    match request.provider {
        AiProviderKind::Ollama => generate_ollama(request),
        AiProviderKind::OpenAi | AiProviderKind::OpenAiCompatible => {
            generate_openai_compatible(connection, request)
        }
        AiProviderKind::Anthropic => generate_anthropic(connection, request),
        AiProviderKind::GoogleAiStudio => generate_google(connection, request),
    }
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

pub fn openai_compatible_url(base_url: &str) -> String {
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

// ── Stop-reason extraction (per-provider field names) ──

/// OpenAI-compatible APIs report "stop" or "length" on the chosen candidate.
pub fn openai_stop_reason(response: &Value) -> Option<String> {
    response
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("finish_reason"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

/// Anthropic reports "end_turn", "max_tokens", "stop_sequence", ...
pub fn anthropic_stop_reason(response: &Value) -> Option<String> {
    response
        .get("stop_reason")
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

/// Gemini reports "STOP", "MAX_TOKENS", "SAFETY", ... on the candidate.
pub fn gemini_stop_reason(response: &Value) -> Option<String> {
    response
        .get("candidates")
        .and_then(Value::as_array)
        .and_then(|candidates| candidates.first())
        .and_then(|candidate| candidate.get("finishReason"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

/// Ollama reports "stop" or "length" via `done_reason`.
pub fn ollama_stop_reason(response: &Value) -> Option<String> {
    response
        .get("done_reason")
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

/// True when the provider stopped because it hit its output token cap rather
/// than finishing naturally — the text is truncated mid-thought.
pub fn stopped_by_token_limit(stop_reason: Option<&str>) -> bool {
    matches!(
        stop_reason.map(|reason| reason.to_lowercase()).as_deref(),
        Some("length") | Some("max_tokens")
    )
}

/// Safe per-provider output ceiling (tokens). Anthropic *requires* `max_tokens`
/// and rejects requests that exceed the model's limit, but the model list
/// doesn't carry output-limit metadata — so we clamp to the lowest common
/// denominator per provider (Claude 3 Opus = 4096 for Anthropic). This keeps
/// script regeneration safe across every user-configurable cloud model (Codex
/// P2 on PR #25).
pub fn provider_output_ceiling(provider: AiProviderKind) -> u32 {
    match provider {
        AiProviderKind::Anthropic => 4096,
        AiProviderKind::OpenAi => 4096,
        AiProviderKind::OpenAiCompatible => 4096,
        AiProviderKind::GoogleAiStudio => 8192,
        // Local providers: leave headroom; the candidate loop already rejects
        // token-truncated output via stop-reason detection.
        AiProviderKind::Ollama => 16384,
    }
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
    let now = timestamp();
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



pub fn build_grounded_context(
    retrieval_items: &[crate::models::SearchChunkResult],
    canvas_context: Option<&str>,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !retrieval_items.is_empty() {
        let snippets: Vec<String> = retrieval_items
            .iter()
            .take(5)
            .map(|r| {
                format!(
                    "Source: {} ({})\n{}",
                    r.title,
                    r.vault_path,
                    r.snippet.trim()
                )
            })
            .collect();
        parts.push(format!(
            "Relevant Vault excerpts:\n{}",
            snippets.join("\n\n")
        ));
    }
    if let Some(canvas) = canvas_context.filter(|c| !c.trim().is_empty()) {
        parts.push(format!("Active Canvas context:\n{}", canvas.trim()));
    }
    if parts.is_empty() {
        "No additional Vault or Canvas context available for this request.".to_string()
    } else {
        parts.join("\n\n")
    }
}

pub fn chat_with_vault(
    connection: &Connection,
    request: &crate::models::ChatWithVaultRequest,
) -> CommandResult<crate::models::ChatWithVaultResponse> {
    let retrieval_query = request
        .vault_query
        .as_deref()
        .filter(|q| !q.trim().is_empty())
        .unwrap_or(&request.prompt);
    let max_items = request.max_retrieval_items.unwrap_or(5).clamp(1, 12);
    let retrieval_items =
        crate::db::search_chunks_internal(connection, retrieval_query, max_items)?;

    let grounded_context = build_grounded_context(
        &retrieval_items,
        request.canvas_context.as_deref(),
    );

    let chat_request = AiChatRequest {
        project_path: request.project_path.clone(),
        provider: request.provider,
        model: request.model.clone(),
        prompt: request.prompt.clone(),
        grounded_context,
    };

    let chat_response = match request.provider {
        AiProviderKind::Ollama => chat_ollama(&chat_request),
        AiProviderKind::OpenAi | AiProviderKind::OpenAiCompatible => {
            chat_openai_compatible(connection, &chat_request)
        }
        AiProviderKind::Anthropic => chat_anthropic(connection, &chat_request),
        AiProviderKind::GoogleAiStudio => chat_google(connection, &chat_request),
    }?;

    let ward_hits = crate::db::scan_wards_internal(connection, &chat_response.text)?.hits;

    let citations: Vec<crate::models::ChatWithVaultCitation> = retrieval_items
        .into_iter()
        .take(5)
        .map(|r| crate::models::ChatWithVaultCitation {
            item_id: r.item_id,
            title: r.title,
            vault_path: r.vault_path,
            snippet: r.snippet,
        })
        .collect();

    Ok(crate::models::ChatWithVaultResponse {
        provider: request.provider,
        model: chat_response.model,
        text: chat_response.text,
        citations,
        ward_hits,
        request_id: chat_response.request_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_temperature_support_detection() {
        // GPT-5 family and o-series reject sampling parameters (Codex P1).
        assert!(!model_supports_temperature("gpt-5-mini"));
        assert!(!model_supports_temperature("gpt-5"));
        assert!(!model_supports_temperature("GPT-5.1"));
        assert!(!model_supports_temperature("o1"));
        assert!(!model_supports_temperature("o1-pro"));
        assert!(!model_supports_temperature("o3-mini"));
        assert!(!model_supports_temperature("o4-mini"));
        // Everything else keeps temperature.
        assert!(model_supports_temperature("gpt-4o"));
        assert!(model_supports_temperature("gpt-4.1"));
        assert!(model_supports_temperature("gpt-4o-mini"));
        assert!(model_supports_temperature("claude-sonnet-4-5"));
    }

    #[test]
    fn provider_output_ceiling_clamps_anthropic_script_budget() {
        // Script regeneration requests 12K tokens, but Claude 3 Opus only
        // allows 4096 — the ceiling must clamp it (Codex P2 on PR #25).
        use crate::ai::AiProviderKind;
        assert_eq!(
            provider_output_ceiling(AiProviderKind::Anthropic),
            4096
        );
        assert_eq!(
            provider_output_ceiling(AiProviderKind::OpenAi),
            4096
        );
        assert_eq!(
            provider_output_ceiling(AiProviderKind::GoogleAiStudio),
            8192
        );
    }

    #[test]
    fn stop_reason_helpers_detect_provider_token_limits() {
        assert!(stopped_by_token_limit(Some("length")));
        assert!(stopped_by_token_limit(Some("MAX_TOKENS")));
        assert!(!stopped_by_token_limit(Some("stop")));
        assert!(!stopped_by_token_limit(None));

        let openai = serde_json::json!({"choices": [{"finish_reason": "length"}]});
        assert_eq!(openai_stop_reason(&openai).as_deref(), Some("length"));

        let anthropic = serde_json::json!({"stop_reason": "max_tokens"});
        assert_eq!(anthropic_stop_reason(&anthropic).as_deref(), Some("max_tokens"));

        let gemini = serde_json::json!({"candidates": [{"finishReason": "MAX_TOKENS"}]});
        assert_eq!(gemini_stop_reason(&gemini).as_deref(), Some("MAX_TOKENS"));

        let ollama = serde_json::json!({"done_reason": "length"});
        assert_eq!(ollama_stop_reason(&ollama).as_deref(), Some("length"));
    }

    #[test]
    fn punctuated_character_search_terms_are_valid_fts() {
        assert_eq!(crate::db::fts_query_terms("O'Connor").unwrap(), "\"O'Connor\"");
        assert_eq!(crate::db::fts_query_terms("Mary-Jane").unwrap(), "\"Mary-Jane\"");
    }

    #[test]
    fn select_ollama_model_picks_previous_if_present() {
        let models = vec!["llama3".to_string(), "mistral".to_string()];
        let result = select_ollama_model(Some("mistral".to_string()), &models);
        assert_eq!(result.as_deref(), Some("mistral"));
    }

    #[test]
    fn select_ollama_model_picks_first_when_single() {
        let models = vec!["llama3".to_string()];
        let result = select_ollama_model(None, &models);
        assert_eq!(result.as_deref(), Some("llama3"));
    }

    #[test]
    fn select_ollama_model_returns_none_for_multiple_without_previous() {
        let models = vec!["llama3".to_string(), "mistral".to_string()];
        let result = select_ollama_model(None, &models);
        assert!(result.is_none());
    }

    #[test]
    fn select_ollama_model_returns_none_for_empty() {
        let result = select_ollama_model(None, &[]);
        assert!(result.is_none());
    }
}
