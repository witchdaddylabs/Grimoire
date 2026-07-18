use crate::ai::{AiProviderKind, AiProviderSettings};
use crate::errors::CommandResult;
use crate::helpers::timestamp;
use rusqlite::{params, Connection};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_provider_true_for_cloud() {
        assert!(cloud_provider(&AiProviderKind::OpenAi));
        assert!(cloud_provider(&AiProviderKind::Anthropic));
        assert!(cloud_provider(&AiProviderKind::GoogleAiStudio));
        assert!(cloud_provider(&AiProviderKind::OpenAiCompatible));
    }

    #[test]
    fn cloud_provider_false_for_local() {
        assert!(!cloud_provider(&AiProviderKind::Ollama));
    }

    #[test]
    fn provider_setting_key_format() {
        let key = provider_setting_key(AiProviderKind::OpenAi, "apiKeyPresent");
        assert_eq!(key, "ai.provider.openAi.apiKeyPresent");
    }

    #[test]
    fn provider_setting_key_ollama() {
        let key = provider_setting_key(AiProviderKind::Ollama, "selectedModel");
        assert_eq!(key, "ai.provider.ollama.selectedModel");
    }
}
