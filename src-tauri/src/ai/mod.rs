use serde::{Deserialize, Serialize};

pub const CLOUD_DISCLOSURE_COPY: &str = r#"Cloud model disclosure

You are selecting a cloud model provider. Grimoire will send the prompt, relevant Vault excerpts, and active Canvas context needed for your Co-Writer request to the selected provider.

Your use of this model is subject to that provider's privacy policy, data processing terms, retention rules, and billing terms. Local-first mode remains available through Ollama, where supported by your machine.

Do not use a cloud provider for private, confidential, regulated, or sensitive manuscript material unless you are comfortable with that provider receiving it under its terms."#;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AiProviderKind {
    Ollama,
    OpenAi,
    OpenAiCompatible,
    Anthropic,
    GoogleAiStudio,
}

impl AiProviderKind {
    pub fn as_key(self) -> &'static str {
        match self {
            Self::Ollama => "ollama",
            Self::OpenAi => "openAi",
            Self::OpenAiCompatible => "openAiCompatible",
            Self::Anthropic => "anthropic",
            Self::GoogleAiStudio => "googleAiStudio",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Ollama => "Ollama",
            Self::OpenAi => "OpenAI",
            Self::OpenAiCompatible => "OpenAI-compatible",
            Self::Anthropic => "Anthropic",
            Self::GoogleAiStudio => "Google AI Studio",
        }
    }

    pub fn default_base_url(self) -> Option<&'static str> {
        match self {
            Self::Ollama => Some("http://127.0.0.1:11434"),
            Self::OpenAi => Some("https://api.openai.com"),
            Self::OpenAiCompatible => None,
            Self::Anthropic => Some("https://api.anthropic.com"),
            Self::GoogleAiStudio => Some("https://generativelanguage.googleapis.com"),
        }
    }

    pub fn default_model(self) -> Option<&'static str> {
        match self {
            Self::Ollama => None,
            Self::OpenAi => Some("gpt-5-mini"),
            Self::OpenAiCompatible => None,
            Self::Anthropic => Some("claude-sonnet-4-5"),
            Self::GoogleAiStudio => Some("gemini-3-flash-preview"),
        }
    }
}

pub const PROVIDERS: [AiProviderKind; 5] = [
    AiProviderKind::Ollama,
    AiProviderKind::OpenAi,
    AiProviderKind::OpenAiCompatible,
    AiProviderKind::Anthropic,
    AiProviderKind::GoogleAiStudio,
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderSettings {
    pub provider: AiProviderKind,
    pub display_name: String,
    pub base_url: Option<String>,
    pub selected_model: Option<String>,
    pub api_key_present: bool,
    pub disclosure_accepted_at: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderSettingsResponse {
    pub active_provider: AiProviderKind,
    pub providers: Vec<AiProviderSettings>,
    pub cloud_disclosure_copy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiModelInfo {
    pub name: String,
    pub modified_at: Option<String>,
    pub size: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderModelsResponse {
    pub provider: AiProviderKind,
    pub reachable: bool,
    pub models: Vec<AiModelInfo>,
    pub selected_model: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiChatRequest {
    pub project_path: String,
    pub provider: AiProviderKind,
    pub model: String,
    pub prompt: String,
    pub grounded_context: String,
}

/// Structured generation request for the Story Plan regeneration pipeline.
/// Unlike `AiChatRequest`, the system prompt and sampling temperature are
/// explicit — the candidate loop needs varied temperatures, and regeneration
/// must carry locked-beat constraints in the system prompt.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiGenerationRequest {
    pub project_path: String,
    pub provider: AiProviderKind,
    pub model: String,
    pub system_prompt: String,
    pub user_prompt: String,
    /// Sampling temperature; providers clamp it to their supported range.
    pub temperature: f64,
    /// Output cap in tokens. Only enforced where the provider requires it
    /// (Anthropic); other providers use their own defaults.
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiChatResponse {
    pub provider: AiProviderKind,
    pub model: String,
    pub text: String,
    pub request_id: Option<String>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudDisclosureAcceptRequest {
    pub project_path: String,
    pub provider: AiProviderKind,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderSelectionRequest {
    pub project_path: String,
    pub provider: AiProviderKind,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderSettingsSaveRequest {
    pub project_path: String,
    pub provider: AiProviderKind,
    pub base_url: Option<String>,
    pub selected_model: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiApiKeyRequest {
    pub project_path: String,
    pub provider: AiProviderKind,
    pub api_key: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disclosure_mentions_privacy_policy() {
        assert!(CLOUD_DISCLOSURE_COPY.contains("privacy policy"));
    }

    #[test]
    fn providers_roster_includes_every_provider_kind() {
        // Regression guard: Anthropic was added to the enum but forgotten in
        // this roster (Codex catch, 2026-08). ai_get_provider_settings resets
        // any provider missing here back to Ollama, silently breaking it.
        let every_kind = [
            AiProviderKind::Ollama,
            AiProviderKind::OpenAi,
            AiProviderKind::OpenAiCompatible,
            AiProviderKind::Anthropic,
            AiProviderKind::GoogleAiStudio,
        ];
        assert_eq!(PROVIDERS.len(), every_kind.len());
        for kind in every_kind {
            assert!(PROVIDERS.contains(&kind), "missing provider: {:?}", kind);
        }
    }
}
