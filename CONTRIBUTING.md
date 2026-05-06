# Contributing

Thanks for helping improve Grimoire.

## Local Setup

Requirements:

- Node.js and npm
- Rust with `cargo`
- Tauri macOS prerequisites

Install dependencies:

```bash
npm install
```

Run the desktop app:

```bash
npm run tauri dev
```

Run the verification gate:

```bash
npm run verify
```

Build a local macOS DMG:

```bash
npm run package:mac
```

## Development Notes

- Keep Grimoire local-first by default.
- Do not add telemetry.
- Do not store API keys in SQLite, project exports, logs, or screenshots.
- Keep cloud providers user-owned-key only and behind explicit disclosure.
- Add focused Rust tests for pure helpers and provider request builders.
- Update QA docs when changing user-facing behavior.

## Pull Requests

Before opening a pull request:

```bash
npm run verify
```

For release or packaging changes, also run:

```bash
npm run package:mac
```

Include a short note about what you tested manually.
