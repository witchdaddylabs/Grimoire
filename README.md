# Grimoire

> A local-first macOS writing studio for novelists, storytellers, and world-builders.
> Dark academia meets luxury telemetry. Your book of spells.

![Grimoire welcome scene](https://github.com/user-attachments/assets/82525245-0a8d-4fd6-a1c5-3998fcc3a102)

## What It Does

**Canvas** — A distraction-free writing surface. Long-form prose, saved locally, always yours.

**Grimoire Vault** — A spatial memory system for lore, characters, canon, and world-building. Organised as Wings → Halls → Rooms → Drawers → Items. Stored inside your project — no external dependencies, no random folders on your disk.

**Co-Writer** — An AI assistant that queries your Vault before answering. Shows citations with source paths so you can verify every claim.

**Wards** — Anti-slop guardrails. Banned words, cliché detection, voice drift monitoring. Keep your prose clean.

Everything runs on your Mac. No accounts. No cloud lock-in. No telemetry.

## Requirements

- macOS 13 (Ventura) or later
- Apple Silicon or Intel
- Ollama (optional, for local AI Co-Writer)

## Download

Releases are published on [GitHub Releases](https://github.com/witchdaddylabs/grimoire/releases).

For unsigned community builds:

1. Download the DMG from the latest GitHub release.
2. Open the DMG.
3. Drag Grimoire to Applications.
4. Launch Grimoire.
5. If macOS Gatekeeper blocks the first launch, open **System Settings → Privacy & Security** and click **Open Anyway**.

Only bypass Gatekeeper if you trust the release source.

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

API keys are stored in macOS Keychain. They are never written to project files, logs, or exports.

## Development

Requirements:

- Node.js 18+ and npm
- Rust 1.80+ with `cargo`
- Tauri macOS prerequisites

```bash
git clone https://github.com/witchdaddylabs/grimoire.git
cd grimoire
npm install
npm run tauri dev    # run in development
cargo check          # check Rust backend
npm run build        # check frontend types + build
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines and PR workflow.

## Project Structure

```
grimoire/
├── src/                    # Frontend (React + TypeScript)
│   └── app/
│       ├── App.tsx         # Main app shell
│       ├── ai.ts           # AI provider integration
│       ├── vault.ts        # Vault data layer
│       └── project.ts      # Project open/create
├── src-tauri/              # Backend (Rust)
│   └── src/
│       ├── main.rs         # Tauri command registration
│       ├── db.rs           # SQLite/project persistence
│       ├── models.rs       # Shared type contracts
│       └── external_vault.rs # Read-only external YAML browsing
├── docs/                   # Technical documentation
├── DESIGN.md               # Design system spec (tokens, principles, anti-references)
├── PRODUCT.md              # Product definition
├── PRIVACY.md              # Privacy policy
├── SECURITY.md             # Security boundaries
└── CONTRIBUTING.md         # Development guidelines
```

## Status

Grimoire is **pre-1.0** software. The current build compiles, runs locally, and is being prepared for its first unsigned public DMG release. Completed foundations include:

- Project open/create workflow replacing hardcoded demo startup
- Frontend/backend module split
- Grimoire Vault decoupling, manual hierarchy creation, JSON export, and read-only external YAML browsing
- Sprint 4 packaging work: bundle identifier, app version, DMG workflow, and release documentation

See the [GitHub Issues](https://github.com/witchdaddylabs/grimoire/issues) for current work items.

## License

MIT. See [LICENSE](LICENSE).

## Credits

Built by [Witch Daddy Labs](https://witchdaddylabs.com).
