// src-tauri/src/models.rs
// Shared type definitions extracted from main.rs

use serde::{Deserialize, Serialize};

// ── Project ──

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCreateRequest {
    pub name: String,
    pub parent_dir: Option<String>,
    pub seed_demo_data: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMetadata {
    pub name: String,
    pub app_version: String,
    pub schema_version: i64,
    pub project_path: String,
    pub database_path: String,
    pub created_at: String,
    pub updated_at: String,
}

// ── Vault (formerly Palace) ──

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultTreeResponse {
    pub wings: Vec<VaultWingNode>,
    pub item_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultWingNode {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub halls: Vec<VaultHallNode>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultHallNode {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub rooms: Vec<VaultRoomNode>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultRoomNode {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub drawers: Vec<VaultDrawerNode>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultDrawerNode {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub items: Vec<VaultItemNode>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VaultItemNode {
    pub id: String,
    pub title: String,
    pub item_type: String,
    pub content: Option<String>,
    pub word_count: i64,
    pub path: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VaultItemDetail {
    pub id: String,
    pub title: String,
    pub item_type: String,
    pub content: String,
    pub plain_text: String,
    pub word_count: i64,
    pub path: String,
    pub updated_at: String,
}

// ── Item commands ──

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemUpdateRequest {
    pub project_path: String,
    pub item_id: String,
    pub title: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportTextRequest {
    pub project_path: String,
    pub title: String,
    pub content: String,
    pub source_name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportTextResponse {
    pub item: VaultItemDetail,
    pub progress_labels: Vec<String>,
    pub created_chunks: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemArchiveRequest {
    pub project_path: String,
    pub item_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemDeleteRequest {
    pub project_path: String,
    pub item_id: String,
}

// ── Search ──

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchChunksRequest {
    pub project_path: String,
    pub query: String,
    pub limit: Option<i64>,
    pub mode: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SearchChunkResult {
    pub chunk_id: String,
    pub item_id: String,
    pub title: String,
    pub item_type: String,
    pub vault_path: String,
    pub snippet: String,
    pub score: f64,
    pub confidence: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchChunksResponse {
    pub query: String,
    pub results: Vec<SearchChunkResult>,
    pub confidence: String,
}

// ── Wards ──

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BannedWord {
    pub id: String,
    pub value: String,
    pub severity: String,
    pub is_default: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WardPhraseRequest {
    pub project_path: String,
    pub value: String,
    pub severity: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WardScanRequest {
    pub project_path: String,
    pub text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WardScanHit {
    pub id: String,
    pub value: String,
    pub severity: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WardScanResponse {
    pub hits: Vec<WardScanHit>,
    pub has_blocking_hits: bool,
}

// ── Ollama / LLM ──

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OllamaModel {
    pub name: String,
    pub modified_at: Option<String>,
    pub size: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OllamaStatus {
    pub base_url: String,
    pub reachable: bool,
    pub models: Vec<OllamaModel>,
    pub selected_model: Option<String>,
    pub message: String,
}

impl From<crate::ai::AiModelInfo> for OllamaModel {
    fn from(model: crate::ai::AiModelInfo) -> Self {
        Self {
            name: model.name,
            modified_at: model.modified_at,
            size: model.size,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OllamaSelectModelRequest {
    pub project_path: String,
    pub model: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OllamaChatRequest {
    pub project_path: String,
    pub model: String,
    pub prompt: String,
    pub context: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OllamaChatResponse {
    pub model: String,
    pub text: String,
}

// ── Export ──

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportItemRequest {
    pub project_path: String,
    pub item_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResponse {
    pub path: String,
    pub message: String,
}

// Note: AI provider types (AiProviderKind, AiProviderSettings, etc.) are in ai/mod.rs
// and imported at the top of main.rs. Do not duplicate them here.
// The OllamaModel From<AiModelInfo> impl is also in ai/mod.rs to avoid circular deps.
