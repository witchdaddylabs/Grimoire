# M14 Cloud BYOK Providers Implementation Plan

> _Historical milestone plan (completed). In this plan "Palace" is today's "Vault". The current roadmap is [README](../../../README.md)._

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add cloud model support through user-owned API keys while preserving Grimoire's local-first default, explicit user consent, local retrieval pipeline, and no-secrets-export promise.

**Architecture:** Introduce a provider-neutral AI layer between local Palace retrieval and Co-Writer generation. Ollama remains the default local provider; OpenAI, OpenAI-compatible, Anthropic, and Google AI Studio/Gemini are opt-in cloud providers. Provider settings live in SQLite, API keys live in macOS Keychain, and cloud disclosure acceptance is required before any off-device request can run.

**Tech Stack:** Tauri v2, Rust 1.95, reqwest blocking JSON clients, rusqlite, macOS Keychain via `keyring-core` plus `apple-native-keyring-store`, React 19, TypeScript, SQLite settings table, local Ollama HTTP API, OpenAI API, Anthropic Messages API, Google Gemini API.

---

## Reference Docs Checked

- OpenAI API auth and request conventions: https://developers.openai.com/api/reference/overview
- Anthropic Messages API: https://docs.anthropic.com/en/api/messages
- Google Gemini API: https://ai.google.dev/api
- Google AI Studio/Gemini API terms and data-use distinction: https://ai.google.dev/gemini-api/terms
- Rust keyring crate guidance: https://docs.rs/crate/keyring/latest

## Product Rules

- Local-first remains the default. A fresh install should use local writing, local search, local wards, local export, and optional Ollama without requiring a cloud account.
- Cloud providers are BYOK only. Grimoire does not supply keys, proxy keys, or bill users for model usage.
- No cloud provider can be used until the user selects it, enters a key, and accepts the cloud disclosure.
- Each cloud request may send the prompt, relevant Palace excerpts, active Canvas context, and generated answer metadata to the selected provider.
- API keys must never be stored in SQLite, exported, logged, shown in screenshots, or included in error messages.
- Cloud provider failure must never block Canvas editing, local search, import, wards, or export.
- AI output is never auto-inserted into the Canvas.

## Required Cloud Disclosure

Use this exact disclosure copy for M14:

```text
Cloud model disclosure

You are selecting a cloud model provider. Grimoire will send the prompt, relevant Palace excerpts, and active Canvas context needed for your Co-Writer request to the selected provider.

Your use of this model is subject to that provider's privacy policy, data processing terms, retention rules, and billing terms. Local-first mode remains available through Ollama, where supported by your machine.

Do not use a cloud provider for private, confidential, regulated, or sensitive manuscript material unless you are comfortable with that provider receiving it under its terms.
```

The user must explicitly acknowledge this before the selected cloud provider can be used.

## File Map

- Create: `src-tauri/src/ai/mod.rs` for shared provider types and dispatch.
- Create: `src-tauri/src/ai/secrets.rs` for Keychain storage wrapper.
- Create: `src-tauri/src/ai/ollama.rs` for migrated Ollama adapter.
- Create: `src-tauri/src/ai/openai.rs` for OpenAI and OpenAI-compatible adapters.
- Create: `src-tauri/src/ai/anthropic.rs` for Anthropic adapter.
- Create: `src-tauri/src/ai/google.rs` for Google AI Studio/Gemini adapter.
- Modify: `src-tauri/src/main.rs` to register AI commands and remove direct Ollama-only Co-Writer logic.
- Modify: `src-tauri/Cargo.toml` to add Keychain crates and keep `reqwest`.
- Modify: `src/app/palace.ts` or create `src/app/ai.ts` for frontend AI command types.
- Modify: `src/app/App.tsx` to add provider settings, disclosure modal, cloud status, and provider-neutral Co-Writer calls.
- Modify: `src/styles/global.css` for provider settings UI and disclosure states.
- Modify: `README.md` to document cloud BYOK behavior, privacy disclosure, key storage, and limitations.
- Create: `docs/qa/m14-cloud-provider-qa.md` for manual QA evidence.

## Provider Types

Define this shape in Rust and mirror it in TypeScript with camelCase serialization:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AiProviderKind {
    Ollama,
    OpenAi,
    OpenAiCompatible,
    Anthropic,
    GoogleAiStudio,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiChatRequest {
    pub project_path: String,
    pub provider: AiProviderKind,
    pub model: String,
    pub prompt: String,
    pub grounded_context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiChatResponse {
    pub provider: AiProviderKind,
    pub model: String,
    pub text: String,
    pub request_id: Option<String>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
}
```

## Task 1: Provider Readiness Baseline

**Files:**
- Read: `docs/architecture/ai-provider-readiness.md`
- Create: `docs/qa/m14-cloud-provider-qa.md`

- [ ] Run M13 verification before touching M14:

```bash
npm run build
cd src-tauri && . "$HOME/.cargo/env" && cargo test && cargo check
cd .. && . "$HOME/.cargo/env" && ./script/build_and_run.sh --verify
```

Expected: all commands exit 0.

- [ ] Create `docs/qa/m14-cloud-provider-qa.md`:

```markdown
# M14 Cloud Provider QA Log

Date: 2026-04-30
Scope: Cloud BYOK providers for Grimoire

## Baseline

- `npm run build`: PASS/FAIL
- `cargo test`: PASS/FAIL
- `cargo check`: PASS/FAIL
- `./script/build_and_run.sh --verify`: PASS/FAIL

## Provider Matrix

| Provider | Key State | Model State | Chat State | Disclosure | Result |
| --- | --- | --- | --- | --- | --- |
| Ollama | n/a | unchecked | unchecked | n/a | pending |
| OpenAI | missing | unchecked | unchecked | pending | pending |
| OpenAI-compatible | missing | unchecked | unchecked | pending | pending |
| Anthropic | missing | unchecked | unchecked | pending | pending |
| Google AI Studio | missing | unchecked | unchecked | pending | pending |
```

## Task 2: Split Provider Types Into Rust Module

**Files:**
- Create: `src-tauri/src/ai/mod.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] Move provider-neutral structs into `src-tauri/src/ai/mod.rs` using the `Provider Types` section above.

- [ ] Add command result helpers:

```rust
pub type AiResult<T> = Result<T, String>;

pub fn cloud_provider(provider: &AiProviderKind) -> bool {
    !matches!(provider, AiProviderKind::Ollama)
}
```

- [ ] Add unit tests in `src-tauri/src/ai/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_cloud_providers() {
        assert!(!cloud_provider(&AiProviderKind::Ollama));
        assert!(cloud_provider(&AiProviderKind::OpenAi));
        assert!(cloud_provider(&AiProviderKind::OpenAiCompatible));
        assert!(cloud_provider(&AiProviderKind::Anthropic));
        assert!(cloud_provider(&AiProviderKind::GoogleAiStudio));
    }
}
```

- [ ] Run `. "$HOME/.cargo/env" && cargo test` in `src-tauri`.
  Expected: new provider test passes.

## Task 3: Keychain Secret Storage

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/ai/secrets.rs`
- Modify: `src-tauri/src/ai/mod.rs`

- [ ] Add macOS Keychain-compatible dependencies. Because `keyring` 4 docs say app developers should use `keyring-core` and provider-specific credential stores, add:

```toml
keyring-core = "1"
apple-native-keyring-store = "1"
zeroize = "1"
```

- [ ] Implement `AiSecretStore` in `src-tauri/src/ai/secrets.rs` with methods:

```rust
pub struct AiSecretStore;

impl AiSecretStore {
    pub fn service_name(project_path: &str, provider: &AiProviderKind) -> String;
    pub fn set_api_key(project_path: &str, provider: &AiProviderKind, api_key: &str) -> AiResult<()>;
    pub fn get_api_key(project_path: &str, provider: &AiProviderKind) -> AiResult<Option<String>>;
    pub fn delete_api_key(project_path: &str, provider: &AiProviderKind) -> AiResult<()>;
    pub fn has_api_key(project_path: &str, provider: &AiProviderKind) -> bool;
}
```

- [ ] Add tests for `service_name` and `has_api_key` behavior through a mock path-safe helper. Do not test real user secrets in CI.

- [ ] Run `. "$HOME/.cargo/env" && cargo test` in `src-tauri`.
  Expected: tests pass and no API key appears in test output.

## Task 4: Provider Settings And Disclosure Commands

**Files:**
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/src/ai/mod.rs`

- [ ] Add Tauri commands:

```text
ai_get_provider_settings(project_path)
ai_save_provider_settings(request)
ai_set_api_key(request)
ai_delete_api_key(project_path, provider)
ai_accept_cloud_disclosure(request)
ai_select_provider(request)
```

- [ ] Store provider settings in the existing SQLite `settings` table with keys:

```text
ai.activeProvider
ai.provider.ollama.selectedModel
ai.provider.openAi.selectedModel
ai.provider.openAiCompatible.baseUrl
ai.provider.openAiCompatible.selectedModel
ai.provider.anthropic.selectedModel
ai.provider.googleAiStudio.selectedModel
ai.provider.<provider>.disclosureAcceptedAt
```

- [ ] Ensure `ai_set_api_key` writes only to Keychain and stores only `apiKeyPresent=true` style status in command responses.

- [ ] Add Rust tests for disclosure gating:

```rust
#[test]
fn cloud_provider_requires_disclosure() {
    assert!(cloud_provider(&AiProviderKind::OpenAi));
}
```

- [ ] Run `. "$HOME/.cargo/env" && cargo test`.

## Task 5: Migrate Ollama Into Provider Adapter

**Files:**
- Create: `src-tauri/src/ai/ollama.rs`
- Modify: `src-tauri/src/main.rs`
- Modify: `src/app/App.tsx`

- [ ] Move existing Ollama status/model/chat logic out of `main.rs` and into `ai/ollama.rs`.

- [ ] Keep existing frontend behavior unchanged:
  - missing Ollama shows local unavailable state
  - one model auto-selects
  - multiple models require selection
  - selected model persists

- [ ] Replace direct frontend command use with provider-neutral commands where practical:

```text
ai_get_provider_settings
ai_list_models
ai_chat
```

- [ ] Run `npm run build`, `cargo test`, and `cargo check`.

## Task 6: OpenAI And OpenAI-Compatible Adapters

**Files:**
- Create: `src-tauri/src/ai/openai.rs`
- Modify: `src-tauri/src/ai/mod.rs`

- [ ] Implement OpenAI official request path using the current OpenAI API auth pattern:
  - base URL: `https://api.openai.com`
  - auth header: `Authorization: Bearer <key>`
  - response text extraction from the selected endpoint

- [ ] Implement OpenAI-compatible request path:
  - user-configured base URL
  - default path compatible with `/v1/chat/completions`
  - auth header: `Authorization: Bearer <key>`

- [ ] Add request builders as pure functions that can be unit-tested without a network call:

```rust
pub fn openai_headers(api_key: &str) -> Vec<(String, String)>;
pub fn openai_compatible_url(base_url: &str) -> String;
```

- [ ] Add unit tests:

```rust
#[test]
fn openai_compatible_url_trims_trailing_slash() {
    assert_eq!(
        openai_compatible_url("https://example.test/"),
        "https://example.test/v1/chat/completions"
    );
}
```

- [ ] Run `. "$HOME/.cargo/env" && cargo test`.

## Task 7: Anthropic Adapter

**Files:**
- Create: `src-tauri/src/ai/anthropic.rs`
- Modify: `src-tauri/src/ai/mod.rs`

- [ ] Implement Anthropic Messages adapter:
  - URL: `https://api.anthropic.com/v1/messages`
  - headers: `x-api-key`, `anthropic-version`, `content-type`
  - request body includes top-level `system` and `messages`
  - response text is collected from text content blocks

- [ ] Add pure request-building tests:

```rust
#[test]
fn anthropic_version_header_is_set() {
    let headers = anthropic_headers("test-key");
    assert!(headers.iter().any(|(name, value)| name == "anthropic-version" && value == "2023-06-01"));
}
```

- [ ] Run `. "$HOME/.cargo/env" && cargo test`.

## Task 8: Google AI Studio / Gemini Adapter

**Files:**
- Create: `src-tauri/src/ai/google.rs`
- Modify: `src-tauri/src/ai/mod.rs`

- [ ] Implement Gemini `generateContent` adapter:
  - URL format: `https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent`
  - header: `x-goog-api-key`
  - request body uses `contents` and `parts`
  - response text is collected from candidate content parts

- [ ] Add pure request-building tests:

```rust
#[test]
fn gemini_url_uses_selected_model() {
    assert_eq!(
        gemini_generate_content_url("gemini-2.5-flash"),
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent"
    );
}
```

- [ ] Run `. "$HOME/.cargo/env" && cargo test`.

## Task 9: Provider Settings UI And Disclosure Modal

**Files:**
- Create: `src/app/ai.ts`
- Modify: `src/app/App.tsx`
- Modify: `src/styles/global.css`

- [ ] Add TypeScript provider types mirroring Rust:

```ts
export type AiProviderKind =
  | "ollama"
  | "openAi"
  | "openAiCompatible"
  | "anthropic"
  | "googleAiStudio";
```

- [ ] Add provider settings controls:
  - provider selector
  - model input/select
  - API key field for cloud providers
  - custom base URL field for OpenAI-compatible
  - local/cloud status indicator
  - delete key action

- [ ] Add cloud disclosure modal using the exact M14 disclosure copy.

- [ ] Require disclosure acceptance before cloud provider validation or Co-Writer request.

- [ ] Persist disclosure acceptance per provider, not globally.

- [ ] Keep Ollama as the first/default provider.

- [ ] Run `npm run build`.

## Task 10: Provider-Neutral Co-Writer Flow

**Files:**
- Modify: `src/app/App.tsx`
- Modify: `src/app/ai.ts`
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/src/ai/mod.rs`

- [ ] Keep local retrieval first for every provider.

- [ ] Build the grounded prompt once and send it through `ai_chat`.

- [ ] For cloud providers, show a visible cloud status before sending:

```text
Cloud request: Palace excerpts and Canvas context will be sent to <provider>.
```

- [ ] After response:
  - show answer
  - show citations
  - show provider/model
  - show ward warnings
  - require Insert / Copy / Rewrite clean / Discard

- [ ] Confirm no cloud output auto-inserts into Canvas.

- [ ] Run `npm run build`, `cargo test`, and `cargo check`.

## Task 11: Export And Logging Safeguards

**Files:**
- Modify: `src-tauri/src/main.rs`
- Modify: `README.md`
- Modify: `docs/qa/m14-cloud-provider-qa.md`

- [ ] Inspect project JSON export and confirm it excludes:
  - API keys
  - masked API keys
  - provider secrets
  - hidden prompts
  - request bodies
  - raw provider responses

- [ ] Add README note:

```markdown
Cloud providers are BYOK. API keys are stored in macOS Keychain, not SQLite, and are never exported. Cloud requests send the selected prompt and required local context to the selected provider under that provider's terms.
```

- [ ] Run an export after configuring a fake cloud provider key and inspect JSON manually.

## Task 12: M14 QA Matrix

**Files:**
- Modify: `docs/qa/m14-cloud-provider-qa.md`
- Modify code only for confirmed failures.

- [ ] Ollama unavailable: local writing still works.

- [ ] Ollama available with one model: auto-selects.

- [ ] OpenAI missing key: clear missing-key state.

- [ ] OpenAI invalid key: clear invalid-key state without leaking key.

- [ ] OpenAI-compatible missing base URL: clear configuration error.

- [ ] Anthropic missing key: clear missing-key state.

- [ ] Google AI Studio missing key: clear missing-key state.

- [ ] Cloud disclosure not accepted: request is blocked.

- [ ] Cloud disclosure accepted: request may proceed.

- [ ] Network offline: cloud provider fails without blocking local workflows.

- [ ] Export after cloud configuration: no secrets exported.

## Task 13: Final Verification

**Files:**
- Modify: `README.md`
- Modify: `docs/qa/m14-cloud-provider-qa.md`

- [ ] Run:

```bash
npm run build
cd src-tauri && . "$HOME/.cargo/env" && cargo test && cargo check
cd .. && . "$HOME/.cargo/env" && ./script/build_and_run.sh --verify
```

- [ ] M14 is complete only when:
  - local Ollama still works as before
  - each cloud provider has missing-key and invalid-key states
  - at least one successful cloud provider request has been manually verified with a user-owned key
  - disclosure is required before cloud requests
  - no API key appears in SQLite, export JSON, logs, or UI after save
  - README documents cloud BYOK clearly
