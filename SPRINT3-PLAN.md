# Sprint 3 — Vault Decoupling + External Connector

**Branch:** `sprint3/vault-decouple`
**Goal:** Complete Vault rename, add hierarchy/item creation, import/export, external YAML connector, trademark sweep

## Task Breakdown

### 1. Vault Rename (Claudia)
- Replaced stale Palace naming in backend copy, AI disclosure text, SQL seed data, and UI copy
- Removed MemPalace references from user-facing app code

### 2. New Item Creation (Claudia + Bobbi)
- Backend: `db_create_vault_node` Tauri command in main.rs
- Frontend: "New Item / New Drawer / New Room / New Hall / New Wing" buttons
- Supports manual creation across the full Wing → Hall → Room → Drawer → Item hierarchy

### 3. Vault Import/Export (Sub-agent)
- Markdown import (enhance existing `db_import_text`)
- JSON export for full project vault
- Markdown export per-item (check if exists)

### 4. External Vault Connector (Sub-agent)
- Added read-only parser for compatible external Vault YAML files
- Added "External Vault YAML" UI in the Vault panel
- Read-only browsing only; Grimoire Vault remains primary and no external data is written into SQLite

### 5. Trademark Sweep (Sub-agent)
- Grep app code for Palace/MemPalace terms — zero matches after connector cleanup
- Verify all UI strings use "Vault" / "Grimoire Vault" / "External Vault YAML"

## Verification
- `npx tsc --noEmit` passes
- `cargo check --manifest-path src-tauri/Cargo.toml` passes
- `cargo test --manifest-path src-tauri/Cargo.toml` passes
- `npm run build` passes
- Create project → add items at every level → persist → restart → still there
- Import markdown → appears in vault
- Export vault → valid JSON
- Trademark: zero "MemPalace" in user-facing strings
