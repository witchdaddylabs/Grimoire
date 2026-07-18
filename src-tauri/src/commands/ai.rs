use super::*;
use crate::ai::{
    cloud_provider, AiApiKeyRequest, AiChatRequest, AiChatResponse, AiModelInfo, AiProviderKind,
    AiProviderModelsResponse, AiProviderSelectionRequest, AiProviderSettings,
    AiProviderSettingsResponse, AiProviderSettingsSaveRequest, CloudDisclosureAcceptRequest,
    CLOUD_DISCLOSURE_COPY, PROVIDERS,
};
use crate::helpers::timestamp;
use crate::llm;
use rusqlite::params;

// ── Settings helpers ──

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

pub fn set_setting(connection: &Connection, key: &str, value: &str) -> CommandResult<()> {
    connection
        .execute(
            r#"
            INSERT INTO settings (key, value, updated_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
            "#,
            params![key, value, timestamp()],
        )
        .map_err(|error| format!("Could not write setting {key}: {error}"))?;
    Ok(())
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
        get_setting(connection, &provider_setting_key(provider, "apiKeyPresent"))?.as_deref()
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

// ── Commands ──

#[tauri::command]
pub fn ai_get_provider_settings(project_path: String) -> CommandResult<AiProviderSettingsResponse> {
    let connection = open_project_database(&project_path)?;
    let mut active_provider = get_active_provider(&connection)?;
    if !PROVIDERS.contains(&active_provider) {
        active_provider = AiProviderKind::Ollama;
        set_setting(&connection, "ai.activeProvider", active_provider.as_key())?;
    }
    let providers = PROVIDERS
        .iter()
        .copied()
        .map(|provider| provider_settings(&connection, provider))
        .collect::<CommandResult<Vec<_>>>()?;

    Ok(AiProviderSettingsResponse {
        active_provider,
        providers,
        cloud_disclosure_copy: CLOUD_DISCLOSURE_COPY.to_string(),
    })
}

#[tauri::command]
pub fn ai_save_provider_settings(
    request: AiProviderSettingsSaveRequest,
) -> CommandResult<AiProviderSettingsResponse> {
    let connection = open_project_database(&request.project_path)?;
    if let Some(base_url) = request.base_url.as_deref() {
        let value = base_url.trim();
        if !value.is_empty() {
            set_setting(
                &connection,
                &provider_setting_key(request.provider, "baseUrl"),
                value,
            )?;
        }
    }
    if let Some(model) = request.selected_model.as_deref() {
        let value = model.trim();
        if !value.is_empty() {
            set_setting(
                &connection,
                &provider_setting_key(request.provider, "selectedModel"),
                value,
            )?;
        }
    }
    ai_get_provider_settings(request.project_path)
}

pub fn ai_save_provider_settings_inner(
    request: AiProviderSettingsSaveRequest,
) -> CommandResult<AiProviderSettingsResponse> {
    ai_save_provider_settings(request)
}

#[tauri::command]
pub fn ai_set_api_key(request: AiApiKeyRequest) -> CommandResult<AiProviderSettingsResponse> {
    if !cloud_provider(&request.provider) {
        return Err("Ollama does not need an API key.".to_string());
    }
    let connection = open_project_database(&request.project_path)?;
    let api_key = request.api_key.trim();
    if api_key.is_empty() {
        return Err("API key cannot be empty.".to_string());
    }
    llm::set_api_key_secret(&request.project_path, request.provider, api_key)?;
    set_setting(
        &connection,
        &provider_setting_key(request.provider, "apiKeyPresent"),
        "true",
    )?;
    ai_get_provider_settings(request.project_path)
}

#[tauri::command]
pub fn ai_delete_api_key(
    project_path: String,
    provider: AiProviderKind,
) -> CommandResult<AiProviderSettingsResponse> {
    let connection = open_project_database(&project_path)?;
    llm::delete_api_key_secret(&project_path, provider)?;
    set_setting(
        &connection,
        &provider_setting_key(provider, "apiKeyPresent"),
        "false",
    )?;
    ai_get_provider_settings(project_path)
}

#[tauri::command]
pub fn ai_accept_cloud_disclosure(
    request: CloudDisclosureAcceptRequest,
) -> CommandResult<AiProviderSettingsResponse> {
    if !cloud_provider(&request.provider) {
        return ai_get_provider_settings(request.project_path);
    }
    let connection = open_project_database(&request.project_path)?;
    set_setting(
        &connection,
        &provider_setting_key(request.provider, "disclosureAcceptedAt"),
        &timestamp(),
    )?;
    ai_get_provider_settings(request.project_path)
}

#[tauri::command]
pub fn ai_select_provider(
    request: AiProviderSelectionRequest,
) -> CommandResult<AiProviderSettingsResponse> {
    let connection = open_project_database(&request.project_path)?;
    set_setting(&connection, "ai.activeProvider", request.provider.as_key())?;
    ai_get_provider_settings(request.project_path)
}

#[tauri::command]
pub fn ai_list_models(
    project_path: String,
    provider: AiProviderKind,
) -> CommandResult<AiProviderModelsResponse> {
    ai_list_models_inner(project_path, provider)
}

pub fn ai_list_models_inner(
    project_path: String,
    provider: AiProviderKind,
) -> CommandResult<AiProviderModelsResponse> {
    let connection = open_project_database(&project_path)?;
    match provider {
        AiProviderKind::Ollama => llm::list_ollama_models(&connection),
        _ => {
            let settings = provider_settings(&connection, provider)?;
            let selected_model = settings
                .selected_model
                .or_else(|| provider.default_model().map(ToString::to_string));
            let models = selected_model
                .iter()
                .map(|name| AiModelInfo {
                    name: name.clone(),
                    modified_at: None,
                    size: None,
                })
                .collect();
            Ok(AiProviderModelsResponse {
                provider,
                reachable: settings.api_key_present,
                models,
                selected_model,
                message: if settings.api_key_present {
                    "Cloud provider key present. Model names are user-configured.".to_string()
                } else {
                    "Add your API key to enable this cloud provider.".to_string()
                },
            })
        }
    }
}

#[tauri::command]
pub fn ai_chat(request: AiChatRequest) -> CommandResult<AiChatResponse> {
    ai_chat_inner(request)
}

pub fn ai_chat_inner(request: AiChatRequest) -> CommandResult<AiChatResponse> {
    let connection = open_project_database(&request.project_path)?;
    if cloud_provider(&request.provider) {
        let settings = provider_settings(&connection, request.provider)?;
        if settings.disclosure_accepted_at.is_none() {
            return Err(
                "Accept the cloud model disclosure before sending Vault context to this provider."
                    .to_string(),
            );
        }
        if !settings.api_key_present {
            return Err(
                "Add an API key for this cloud provider before sending a Co-Writer request."
                    .to_string(),
            );
        }
    }

    match request.provider {
        AiProviderKind::Ollama => llm::chat_ollama(&request),
        AiProviderKind::OpenAi | AiProviderKind::OpenAiCompatible => {
            llm::chat_openai_compatible(&connection, &request)
        }
        AiProviderKind::Anthropic => llm::chat_anthropic(&connection, &request),
        AiProviderKind::GoogleAiStudio => llm::chat_google(&connection, &request),
    }
}

#[tauri::command]
pub fn chat_with_vault(request: ChatWithVaultRequest) -> CommandResult<ChatWithVaultResponse> {
    let connection = open_project_database(&request.project_path)?;
    llm::chat_with_vault(&connection, &request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn cloud_provider_detection_matches_local_first_rule() {
        assert!(!cloud_provider(&AiProviderKind::Ollama));
        assert!(cloud_provider(&AiProviderKind::OpenAi));
    }

    #[test]
    fn disclosure_copy_names_privacy_policy() {
        assert!(CLOUD_DISCLOSURE_COPY.contains("privacy policy"));
    }

    #[test]
    fn provider_settings_use_stored_key_presence_without_keychain_lookup() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                r#"
                CREATE TABLE settings (
                  key TEXT PRIMARY KEY,
                  value TEXT NOT NULL,
                  updated_at TEXT NOT NULL
                );
                INSERT INTO settings (key, value, updated_at)
                VALUES ('ai.provider.openAi.apiKeyPresent', 'true', 'test');
                "#,
            )
            .unwrap();

        let settings = provider_settings(&connection, AiProviderKind::OpenAi).unwrap();
        assert!(settings.api_key_present);
    }
}
