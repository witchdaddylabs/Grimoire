# Contributing

Thanks for helping improve Grimoire.

## Local Setup

Requirements:

- Node.js 18+ and npm
- Rust 1.80+ with `cargo`
- Tauri macOS prerequisites (Xcode Command Line Tools)

Install dependencies:

```bash
npm install
```

Run the desktop app:

```bash
npm run tauri dev
```

Build a local macOS DMG:

```bash
npm run tauri build
```

Run checks before committing:

```bash
npm run build    # frontend type-check + build
cargo check      # Rust backend
```

## Development Notes

- Keep Grimoire local-first by default. No telemetry, no analytics.
- The Grimoire Vault lives inside the `.grimoire` project bundle — never write to external or random paths on the user's machine.
- "Palace" is legacy terminology from the Codex genesis sprints. All new code uses "Vault" (`GrimoireVault`, `VaultWingNode`, `db_get_vault_tree`, etc.).
- Do not store API keys in SQLite, project exports, logs, or screenshots.
- Keep cloud providers user-owned-key only and behind explicit disclosure.
- Add focused Rust tests for pure helpers and provider request builders.
- Update docs when changing user-facing behavior.

## Pull Requests

Branch pattern: `sprintN/description` → PR → review → merge. No direct pushes to `main`.

Before opening a pull request, run:

```bash
npm run build && cargo check
```

For release or packaging changes, also run:

```bash
npm run tauri build
```

Include a short note about what you tested manually.
