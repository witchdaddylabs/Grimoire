# Grimoire

> A local-first writing studio for novelists, storytellers, and world-builders — now on **macOS and Windows**.
> Dark academia meets luxury telemetry. Your book of spells.

![Grimoire welcome scene](https://github.com/user-attachments/assets/82525245-0a8d-4fd6-a1c5-3998fcc3a102)

## What It Does

**Canvas** — A distraction-free writing surface. Long-form prose, saved locally, always yours.

**Grimoire Vault** — A spatial memory system for lore, characters, canon, and world-building. Organised as Wings → Halls → Rooms → Drawers → Items. Stored inside your project — no external dependencies, no random folders on your disk.

**Story Plan** — A structural layer that keeps your outline and your prose aligned. Pin the beats that are working, regenerate the ones that aren't, and compare variants before anything touches your draft.

**Co-Writer** — An AI assistant that queries your Vault before answering. Shows citations with source paths so you can verify every claim.

**Wards** — Anti-slop guardrails. Banned words, cliché detection, voice drift monitoring. Keep your prose clean.

Everything runs on your own machine — Mac or Windows. No accounts. No cloud lock-in. No telemetry.

## Requirements

- **Windows 10 or 11 (x64)**, **or** macOS 13 (Ventura) or later (Apple Silicon or Intel)
- Ollama (optional, for the local AI Co-Writer)

Grimoire keeps your provider API keys in your operating system's own vault — **Windows Credential Manager** or **macOS Keychain** — never in your project files, logs, or exports.

## Download

Grab the latest build from [GitHub Releases](https://github.com/witchdaddylabs/grimoire/releases). Pick the file for your machine, and you're a couple of clicks from writing.

### Windows

1. Download the `Grimoire_*_x64-setup.exe` installer.
2. Double-click it. Windows SmartScreen may pause and say it doesn't recognise the app — that's expected for a community build that isn't code-signed yet.
3. Click **More info → Run anyway**, then follow the installer.
4. Launch Grimoire from the Start menu.

### macOS

1. Download the `.dmg` from the latest release.
2. Open it and drag **Grimoire** to **Applications**.
3. Launch Grimoire. If Gatekeeper blocks the first open, head to **System Settings → Privacy & Security** and click **Open Anyway** (or use the included **Unlock-Grimoire.command** helper).

These are unsigned community builds, so your OS will ask you to confirm you trust them — only open Grimoire if you trust the release source. Code signing is on the roadmap.

## First Launch

When you open Grimoire for the first time, you'll be prompted to:

- **Create a New Project** — start with a fresh Vault
- **Open an Existing Project** — load a `.grimoire` project folder
- **Load Demo** — explore with sample data

Each project is a self-contained `.grimoire` folder with your writing, Vault, search index, and wards.

## Local-First By Default

Every project is a local `.grimoire` folder containing:

- Project metadata
- Grimoire Vault (your Wings, Halls, Rooms, Drawers, Items)
- Writing items and search chunks
- Wards (banned words and phrases)
- Exports

The app does not include telemetry, analytics SDKs, bundled model weights, hosted sync, or a required cloud account.

## Vault Memory System

The **Grimoire Vault** is your spatial memory:

```
Project
└── Wings (top-level categories — "Characters", "World", "Drafts")
    └── Halls (sections within a wing — "Protagonists", "Locations")
        └── Rooms (groupings — "Main Cast", "Northern Cities")
            └── Drawers (containers — "Physical Traits", "Backstory")
                └── Items (individual notes — "Mara Thorne", "Chapter 01")
```

Items can hold prose, character sheets, world notes, chapter drafts — anything your project needs. The Co-Writer searches this Vault to ground every answer in your canon.

The Vault is stored entirely within your project folder. It does not install to or depend on any external memory system.

### External Vault Connector (optional)

For users who also keep compatible external YAML knowledge stores, Grimoire can optionally connect to browse them read-only. This is separate from your project's own Vault and is entirely opt-in.

## Story Plan — structure that stays aligned

Most AI writing tools generate content. Grimoire's Story Plan does something different: it holds your **structure** steady while you revise, so the outline and the prose don't drift apart.

A Story Plan sits between you and the model:

```
Plan          (logline, synopsis, status)
└── Scenes    (title, setting, summary, optional link to a Canvas item)
    └── Beats (what happens — action, dialogue, revelation, conflict, transition)
```

### Pin what's working

Any beat can be **pinned**. Pinned beats are treated as fixed points — the model is told explicitly not to change them, and Grimoire refuses to regenerate a pinned beat directly. This is the difference between "rewrite this scene" and "rewrite this scene *without breaking the bit I already love*".

### Convergent iteration, not reroll roulette

When you regenerate a layer, you give it an **edit instruction** — "tighten the dialogue", "raise the tension", "cut the fat". Grimoire then assembles a six-point context so the model revises rather than reinvents:

1. Your logline and synopsis
2. Character facts pulled from your Vault
3. The final beat of the previous scene
4. The opening beat of the next scene
5. Every pinned beat, as hard constraints
6. Your edit instruction

The current material goes in too, so "tighten the dialogue" has actual dialogue to tighten.

### Compare before you commit

Each run produces up to five variants at a spread of temperatures — the first conservative, the last loosest. Nothing is applied automatically. Each variant is scanned against your Wards, and a variant with a blocking ward can't be accepted until you deal with it. You read them, pick one, and only then does your plan change.

Accepting a variant writes it back to the layer you targeted — plan synopsis, scene summary, or beat content. If the scene is linked to a Canvas item, Grimoire offers to take you there. Rejected variants stay in a history drawer, so nothing is lost.

### Positioning

Story Plan is a **structural editor**, not a content generator. It assumes you're writing the book — it just refuses to let the scaffolding rot while you do. Everything runs through your chosen provider (local Ollama or your own cloud key), and nothing leaves your machine without the same explicit consent the Co-Writer requires.

## Co-Writer

The Co-Writer uses a local or cloud AI model to help with your writing. Before answering, it:

1. **Searches your Vault** for relevant canon
2. **Shows citations** with source paths and confidence scores
3. **Composes an answer** grounded in your existing material

Nothing is sent off-device without your explicit consent.

### Ollama (local, free)

Ollama is optional. When available, Grimoire connects to the local Ollama server at:

```
http://127.0.0.1:11434
```

Writing, Vault search, wards, and export all work without Ollama.

### Cloud BYOK (optional)

Cloud providers are available for users who prefer them or don't have local models:

- OpenAI
- OpenAI-compatible endpoints (Groq, Together, vLLM, etc.)
- Google AI Studio

Cloud providers require you to choose a provider, enter your own API key, and accept a disclosure before any Vault excerpts or Canvas context are sent off-device.

API keys are stored in your OS credential store — Windows Credential Manager or macOS Keychain. They are never written to project files, logs, or exports.

## Development

Requirements:

- Node.js 18+ and npm
- Rust 1.80+ with `cargo`
- Tauri prerequisites for your platform ([macOS](https://tauri.app/start/prerequisites/) or Windows — on Windows you'll need the **Visual Studio Build Tools** with the Desktop C++ workload and **WebView2**, which ships with Windows 11)

```bash
git clone https://github.com/witchdaddylabs/grimoire.git
cd grimoire
npm install
npm run tauri dev    # run in development
cargo check          # check Rust backend
npm run build        # check frontend types + build

# Package a desktop installer
npm run package:win  # Windows NSIS .exe
npm run package:mac  # macOS DMG
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines and PR workflow.

## Project Structure

```
grimoire/
├── src/                        # Frontend (React + TypeScript)
│   ├── app/
│   │   ├── App.tsx             # Main app shell
│   │   ├── ai.ts               # AI provider integration
│   │   ├── vault.ts            # Vault data layer
│   │   ├── storyplan.ts        # Story Plan data layer
│   │   └── project.ts          # Project open/create
│   └── features/
│       ├── storyplan/          # Plan editor, regeneration, candidate review
│       ├── cowriter/           # Co-Writer panel + hook
│       ├── vault/              # Vault tree
│       ├── wards/              # Ward management
│       └── settings/           # Provider settings
├── src-tauri/                  # Backend (Rust)
│   └── src/
│       ├── main.rs             # Tauri command registration
│       ├── commands/           # Command modules (storyplan, vault, wards, …)
│       ├── storyplan_context.rs # Six-point regeneration context assembler
│       ├── llm.rs              # Provider generation layer
│       ├── db.rs               # SQLite/project persistence
│       ├── models.rs           # Shared type contracts
│       └── external_vault.rs   # Read-only external YAML browsing
├── docs/                       # Technical documentation
├── DESIGN.md                   # Design system spec (tokens, principles, anti-references)
├── PRODUCT.md                  # Product definition
├── PRIVACY.md                  # Privacy policy
├── SECURITY.md                 # Security boundaries
└── CONTRIBUTING.md             # Development guidelines
```

## Status

Grimoire runs on both **macOS and Windows**, and ships as unsigned community installers (macOS DMG + Windows NSIS `.exe`). Completed foundations include:

- Project open/create workflow replacing hardcoded demo startup
- Frontend/backend module split
- Grimoire Vault decoupling, manual hierarchy creation, JSON export, and read-only external YAML browsing
- Cross-platform credential storage (`keyring`), Windows icon + NSIS installer, and a `windows-latest` CI/release pipeline
- **Story Plan layer** — Plan→Scenes→Beats structural editor, beat pinning, six-point regeneration context, multi-variant generation with ward scanning, and an accept/reject flow that writes back to the plan

See the [GitHub Issues](https://github.com/witchdaddylabs/grimoire/issues) for current work items.

## License

MIT. See [LICENSE](LICENSE).

## Credits

Built by [Witch Daddy Labs](https://witchdaddylabs.com).
