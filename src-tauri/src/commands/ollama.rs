use super::*;
use crate::ai::{AiChatRequest, AiProviderKind, AiProviderSettingsSaveRequest};

#[tauri::command]
pub fn ollama_get_status(project_path: String) -> CommandResult<OllamaStatus> {
    let response = crate::commands::ai::ai_list_models_inner(project_path, AiProviderKind::Ollama)?;
    Ok(OllamaStatus {
        base_url: "http://127.0.0.1:11434".to_string(),
        reachable: response.reachable,
        models: response.models.into_iter().map(Into::into).collect(),
        selected_model: response.selected_model,
        message: response.message,
    })
}

#[tauri::command]
pub fn ollama_select_model(request: OllamaSelectModelRequest) -> CommandResult<OllamaStatus> {
    crate::commands::ai::ai_save_provider_settings_inner(AiProviderSettingsSaveRequest {
        project_path: request.project_path.clone(),
        provider: AiProviderKind::Ollama,
        base_url: None,
        selected_model: Some(request.model),
    })?;
    ollama_get_status(request.project_path)
}

#[tauri::command]
pub fn ollama_chat(request: OllamaChatRequest) -> CommandResult<OllamaChatResponse> {
    let response = crate::commands::ai::ai_chat_inner(AiChatRequest {
        project_path: request.project_path,
        provider: AiProviderKind::Ollama,
        model: request.model,
        prompt: request.prompt,
        grounded_context: request.context.unwrap_or_default(),
    })?;
    Ok(OllamaChatResponse {
        model: response.model,
        text: response.text,
    })
}
