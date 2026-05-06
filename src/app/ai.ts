import { invoke } from "@tauri-apps/api/core";

export type AiProviderKind =
  | "ollama"
  | "openAi"
  | "openAiCompatible"
  | "anthropic"
  | "googleAiStudio";

export type AiProviderSettings = {
  provider: AiProviderKind;
  displayName: string;
  baseUrl: string | null;
  selectedModel: string | null;
  apiKeyPresent: boolean;
  disclosureAcceptedAt: string | null;
  enabled: boolean;
};

export type AiProviderSettingsResponse = {
  activeProvider: AiProviderKind;
  providers: AiProviderSettings[];
  cloudDisclosureCopy: string;
};

export type AiModelInfo = {
  name: string;
  modifiedAt: string | null;
  size: number | null;
};

export type AiProviderModelsResponse = {
  provider: AiProviderKind;
  reachable: boolean;
  models: AiModelInfo[];
  selectedModel: string | null;
  message: string;
};

export type AiChatResponse = {
  provider: AiProviderKind;
  model: string;
  text: string;
  requestId: string | null;
  inputTokens: number | null;
  outputTokens: number | null;
};

export const providerLabels: Record<AiProviderKind, string> = {
  ollama: "Ollama",
  openAi: "OpenAI",
  openAiCompatible: "OpenAI-compatible",
  anthropic: "Anthropic",
  googleAiStudio: "Google AI Studio",
};

export function cloudProvider(provider: AiProviderKind) {
  return provider !== "ollama";
}

export function getProviderSettings(projectPath: string) {
  return invoke<AiProviderSettingsResponse>("ai_get_provider_settings", { projectPath });
}

export function saveProviderSettings(
  projectPath: string,
  provider: AiProviderKind,
  baseUrl?: string | null,
  selectedModel?: string | null,
) {
  return invoke<AiProviderSettingsResponse>("ai_save_provider_settings", {
    request: { projectPath, provider, baseUrl, selectedModel },
  });
}

export function setProviderApiKey(projectPath: string, provider: AiProviderKind, apiKey: string) {
  return invoke<AiProviderSettingsResponse>("ai_set_api_key", {
    request: { projectPath, provider, apiKey },
  });
}

export function deleteProviderApiKey(projectPath: string, provider: AiProviderKind) {
  return invoke<AiProviderSettingsResponse>("ai_delete_api_key", { projectPath, provider });
}

export function acceptCloudDisclosure(projectPath: string, provider: AiProviderKind) {
  return invoke<AiProviderSettingsResponse>("ai_accept_cloud_disclosure", {
    request: { projectPath, provider },
  });
}

export function selectProvider(projectPath: string, provider: AiProviderKind) {
  return invoke<AiProviderSettingsResponse>("ai_select_provider", {
    request: { projectPath, provider },
  });
}

export function listProviderModels(projectPath: string, provider: AiProviderKind) {
  return invoke<AiProviderModelsResponse>("ai_list_models", { projectPath, provider });
}

export function aiChat(
  projectPath: string,
  provider: AiProviderKind,
  model: string,
  prompt: string,
  groundedContext: string,
) {
  return invoke<AiChatResponse>("ai_chat", {
    request: { projectPath, provider, model, prompt, groundedContext },
  });
}
