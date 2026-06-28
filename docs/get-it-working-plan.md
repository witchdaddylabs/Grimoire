# Grimoire — Get It Working Plan

> **Date:** 2026-06-28
> **V0.2.4 current state** — Paused after Sprint 5 (UI overhaul). Tauri v2 · React 19 · TypeScript 6 · Vite 8 · Tailwind CSS 4 · Rust · SQLite · FTS
> **Alchemist lessons applied** — Cross-platform Tauri v2 build proven, no Apple dev account required, unsigned DMG/MSI is fine.
> **Target:** Shippable v1.0 for macOS + Windows

---

## Why This Plan Works

Alchemist proved the cross-platform Tauri v2 pattern. We shipped:
- **macOS:** Unsigned DMG via GitHub Releases → download, bypass Gatekeeper, works
- **Windows:** MSI installer built from the same codebase with zero Apple bureaucracy

Grimoire is the same stack. The codebase is in better shape than Alchemist was at this stage — the monolith's already split, UI overhaul is done, the vault system is real. The gaps are:

1. **No Windows build target configured**
2. **Co-Writer AI is wired in backend but has no frontend** — this is the main feature people want
3. **No manuscript-level export** — per-item Markdown works, but you can't export a full book
4. **Some polish gaps** — settings UI, README, release pipeline

---

## Phase 1 — Windows Build (Sprint 6)

**Goal:** Grimoire compiles and runs on Windows. MSI installer published.

**Target branch:** `sprint6/windows-build`

### Checklist

- [ ] Add Windows build target to `tauri.conf.json`:
  ```json
  "bundle": {
    "active": true,
    "targets": ["dmg", "msi"],
    "icon": ["icons/32x32.png", "icons/128x128.png", "icons/128x128@2x.png", "icons/icon.icns", "icons/icon.ico"]
  }
  ```
- [ ] Generate `icons/icon.ico` — use existing SVG source via `svgexport` (or a PNG-to-ICO converter on Windows)
- [ ] Generate Windows Store logo sizes (optional but good for MSI)
- [ ] Verify `npm run tauri build` on Windows → produces `.msi` or `.exe` installer
- [ ] Test on clean Windows machine — install and launch
- [ ] Update GitHub Actions CI to build on `windows-latest` as well as `macos-latest`
- [ ] Publish Windows release alongside macOS DMG

### Alchemist-verified pattern

```yaml
# .github/workflows/release.yml — add a windows-latest job
jobs:
  build-macos:
    runs-on: macos-latest
    # ... existing config ...

  build-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: "npm"
      - uses: dtolnay/rust-toolchain@stable
      - run: npm ci
      - run: npm run tauri build
      - uses: actions/upload-artifact@v4
        with:
          name: Grimoire-windows-msi
          path: src-tauri/target/release/bundle/msi/*.msi
```

---

## Phase 2 — Co-Writer AI Frontend (Sprint 7)

**Goal:** The Co-Writer chat panel works end-to-end — user types a question, the app queries Ollama (or a configured cloud provider), and displays the answer with vault context citations.

**Why now:** The Rust backend for Co-Writer already exists in `src-tauri/src/llm.rs` (Ollama + cloud providers). The frontend chat panel was extracted but never wired. This is the single biggest value gap.

**Target branch:** `sprint7/cowriter-wired`

### Checklist

**Backend (verify/complete):**
- [ ] `src-tauri/src/llm.rs` — confirm Ollama provider works (model auto-detect, chat completion)
- [ ] `src-tauri/src/llm.rs` — confirm cloud provider path works (OpenAI-compatible, API key from store)
- [ ] If cloud provider settings UI doesn't exist yet, add a minimal provider config dialog (API key input, model selector, test connection) — or reuse the Alchemist pattern
- [ ] Wire a `chat_with_vault` command that:
  - Takes user question + optional vault context filter
  - Queries vault for relevant items (search via FTS)
  - Sends question + context to LLM
  - Returns response with cited source items

**Frontend:**
- [ ] Wire the CoWriterPanel in `src/features/cowriter/` to call the backend chat command
- [ ] Fix the chat panel layout — currently it's a separate tab, should be integrated into the Canvas writing flow
- [ ] Add inline suggestion ("Ask Co-Writer" → highlight text → right-click → insert/reject) — this is the killer feature, even if basic
- [ ] Show vault context citations inline (source item title + path)
- [ ] Provider/model selector in the chat panel

---

## Phase 3 — Manuscript Export (Sprint 8)

**Goal:** Full project export as a single, continuous document — not just per-item Markdown.

**Target branch:** `sprint8/manuscript-export`

### Checklist

- [ ] Build "Manuscript View" — a special mode in Canvas that shows all items in a wing/room ordered by a `position` field
- [ ] Add chapter/scene management — drag-to-reorder items in the vault tree
- [ ] Full project export as single `.md` or `.docx`:
  - All vault items concatenated in order
  - Wing/Room/Item hierarchy preserved as headings
  - Export as Markdown (easy) or via Pandoc to DOCX/PDF (bonus)
- [ ] Add export modal — select what to export (current item, current room, whole project), choose format

---

## Phase 4 — Polish + Ship (Sprint 9)

**Goal:** Grimoire v1.0 is polished and shippable on both platforms.

**Target branch:** `sprint9/polish-and-ship`

### Checklist

- [ ] **Settings UI** — minimal but functional:
  - Project settings (rename, change location)
  - Ollama URL/config
  - Cloud provider setup (API key, model selector) — if Co-Writer needs it
  - Theme toggle (dark/ivory — already works, just needs a settings entry)
- [ ] **README overhaul** — Windows + macOS, screenshots, install instructions
- [ ] **Windows icon** — proper `.ico` with all required sizes
- [ ] **GitHub Release pipeline** — builds both macOS and Windows on tag push
- [ ] **Version bump** → `v1.0.0`
- [ ] **Tag and release** — `git tag v1.0.0 && git push origin v1.0.0`

---

## Risk Assessment

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| Windows Rust build fails (cold compile) | MEDIUM | First build takes 2-5 mins. Run `cargo check` first to confirm code compiles before full Tauri build. |
| `npm install` fails on Windows (node-gyp, native deps) | LOW | Alchemist built clean on Windows. Same deps. |
| Co-Writer backend doesn't work after months of inactivity | MEDIUM | Test with Ollama first (no API key needed). If the Rust module needs fixes, they're surgical — the pattern is proven in Alchemist. |
| Unsigned Windows installer triggers SmartScreen | LOW | Same as macOS Gatekeeper. Users click "Run anyway". Document in README. |

---

## Quick Start for Windows Machine

```bash
# 1. Clone the repo
git clone https://github.com/witchdaddylabs/Grimoire.git
cd Grimoire

# 2. Install deps
npm install

# 3. Generate Windows icons (from existing SVG)
# npx svgexport <app-icon-svg> src-tauri/icons/icon.ico 256:256

# 4. Check Rust compiles
cd src-tauri && cargo check && cd ..

# 5. Build
npm run tauri build
```

---

## Already Working (Don't Rebuild)

These are proven solid from Sprint 5 and don't need changes:

- ✅ Project create/open with `.grimoire` project folders
- ✅ Vault tree navigation (Wings → Halls → Rooms → Drawers → Items)
- ✅ Canvas writing with autosave, word count, title editing
- ✅ Text import (paste + file) with chunking
- ✅ Vault search (FTS)
- ✅ Export per-item as Markdown
- ✅ Export full project as JSON
- ✅ Focus mode
- ✅ Dark/ivory theme toggle
- ✅ 3-panel layout with collapsible side panels
- ✅ Toast notifications
- ✅ Rust backend modules split (db, vault, wards, llm, models, errors)
- ✅ Frontend feature panels split (vault, canvas, cowriter)
- ✅ Monorepo structure ready for cross-platform build
