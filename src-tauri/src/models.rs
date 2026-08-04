// src-tauri/src/models.rs
// Shared type definitions extracted from main.rs

use serde::{Deserialize, Serialize};

// Re-exported from ai/mod.rs for use in model definitions
pub use crate::ai::AiProviderKind;

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

// ── Vault hierarchy ──

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

// ── Vault node creation ──

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateVaultNodeRequest {
    pub project_path: String,
    /// "wing", "hall", "room", "drawer", or "item"
    pub node_type: String,
    /// Parent ID: not required for wings, required for all others
    pub parent_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    /// Only for items: "chapter", "scene", "character", "location", "lore", "timeline", "faction", "research", "note"
    pub item_type: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateVaultNodeResponse {
    pub id: String,
    pub node_type: String,
    /// Updated tree after creation
    pub tree: VaultTreeResponse,
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

// ── Co-Writer grounded chat ──

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatWithVaultRequest {
    pub project_path: String,
    pub provider: AiProviderKind,
    pub model: String,
    pub prompt: String,
    pub vault_query: Option<String>,
    pub canvas_context: Option<String>,
    pub max_retrieval_items: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatWithVaultCitation {
    pub item_id: String,
    pub title: String,
    pub vault_path: String,
    pub snippet: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatWithVaultResponse {
    pub provider: AiProviderKind,
    pub model: String,
    pub text: String,
    pub citations: Vec<ChatWithVaultCitation>,
    pub ward_hits: Vec<WardScanHit>,
    pub request_id: Option<String>,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExternalVaultStructure {
    pub wings: Vec<ExternalVaultWing>,
    pub total_wings: usize,
    pub total_rooms: usize,
    pub total_drawers: usize,
    pub source_file: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExternalVaultWing {
    pub name: String,
    pub path: Option<String>,
    pub rooms: Vec<ExternalVaultRoom>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExternalVaultRoom {
    pub name: String,
    pub keywords: Vec<String>,
    pub entities: Vec<String>,
    pub drawers: Vec<ExternalVaultDrawer>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExternalVaultDrawer {
    pub name: String,
    pub keywords: Vec<String>,
    pub descriptions: Vec<String>,
    pub entities: Vec<String>,
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

// ── Manuscript export ──

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManuscriptExportRequest {
    pub project_path: String,
    pub project_name: String,
    /// Reserved for future export formats; only "markdown" is supported today.
    /// Kept on the wire contract because the frontend sends it.
    #[allow(dead_code)]
    pub format: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemReorderRequest {
    pub project_path: String,
    pub item_id: String,
    pub direction: Option<String>,
}

// ── Story Plan (Schema v3 — Fabula-style structure layer) ──

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StoryPlan {
    pub id: String,
    pub project_name: String,
    pub logline: Option<String>,
    pub synopsis: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StoryScene {
    pub id: String,
    pub plan_id: String,
    pub title: String,
    pub setting: Option<String>,
    pub summary: Option<String>,
    pub linked_item_id: Option<String>,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StoryBeat {
    pub id: String,
    pub scene_id: String,
    pub beat_type: String,
    pub content: String,
    /// Character names; stored in SQLite as a JSON array text column.
    pub characters: Option<Vec<String>>,
    pub locked: bool,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StoryCandidate {
    pub id: String,
    pub target_kind: String,
    pub target_id: String,
    pub provider: String,
    pub model: String,
    pub prompt_summary: Option<String>,
    pub candidate_index: i64,
    pub content: String,
    pub status: String,
    pub created_at: String,
}

/// Full tree for a single plan: plan → scenes → beats.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorySceneWithBeats {
    #[serde(flatten)]
    pub scene: StoryScene,
    pub beats: Vec<StoryBeat>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryPlanDetail {
    #[serde(flatten)]
    pub plan: StoryPlan,
    pub scenes: Vec<StorySceneWithBeats>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryPlanListResponse {
    pub plans: Vec<StoryPlan>,
}

// Story Plan requests

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryPlanCreateRequest {
    pub project_path: String,
    pub project_name: String,
    pub logline: Option<String>,
    pub synopsis: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryPlanUpdateRequest {
    pub project_path: String,
    pub plan_id: String,
    pub project_name: Option<String>,
    pub logline: Option<String>,
    pub synopsis: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorySceneCreateRequest {
    pub project_path: String,
    pub plan_id: String,
    pub title: String,
    pub setting: Option<String>,
    pub summary: Option<String>,
    pub linked_item_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorySceneUpdateRequest {
    pub project_path: String,
    pub scene_id: String,
    pub title: Option<String>,
    pub setting: Option<String>,
    pub summary: Option<String>,
    pub linked_item_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryBeatCreateRequest {
    pub project_path: String,
    pub scene_id: String,
    pub beat_type: Option<String>,
    pub content: String,
    pub characters: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryBeatUpdateRequest {
    pub project_path: String,
    pub beat_id: String,
    pub beat_type: Option<String>,
    pub content: Option<String>,
    pub characters: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryReorderRequest {
    pub project_path: String,
    /// "scene" or "beat"
    pub kind: String,
    pub id: String,
    /// "up" or "down"
    pub direction: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryBeatLockRequest {
    pub project_path: String,
    pub beat_id: String,
    pub locked: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryPlanDeleteRequest {
    pub project_path: String,
    pub plan_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorySceneDeleteRequest {
    pub project_path: String,
    pub scene_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryBeatDeleteRequest {
    pub project_path: String,
    pub beat_id: String,
}

// Candidate requests / responses

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryCandidateStoreRequest {
    pub project_path: String,
    pub target_kind: String,
    pub target_id: String,
    pub provider: String,
    pub model: String,
    pub prompt_summary: Option<String>,
    pub candidate_index: i64,
    pub content: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryCandidateResolveRequest {
    pub project_path: String,
    pub candidate_id: String,
    /// "accepted" or "rejected"
    pub resolution: String,
}

// Note: AI provider types (AiProviderKind, AiProviderSettings, etc.) are in ai/mod.rs
// and imported at the top of main.rs. Do not duplicate them here.
// The OllamaModel From<AiModelInfo> impl is also in ai/mod.rs to avoid circular deps.
