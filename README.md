# Grimoire
<img width="1672" height="941" alt="magical_grimoire_welcome_scene" src="https://github.com/user-attachments/assets/82525245-0a8d-4fd6-a1c5-3998fcc3a102" />


Grimoire is a local-first macOS writing studio for long-form projects, Palace-style memory, and AI assistance.

It stores your writing locally, uses SQLite for project memory/search, and can talk to local Ollama models when you choose to use them. Cloud providers are stable BYOK options for users who do not have local models available; they require explicit disclosure before any Palace excerpts or Canvas context are sent off-device.

## Status

Grimoire is currently pre-1.0 software. It can be built and packaged as a macOS DMG, but the public release path is currently unsigned because the project does not yet have an Apple Developer account.

Unsigned builds are usable, but macOS Gatekeeper may block first launch.

## Download

Releases are intended to be published from GitHub Releases.

For unsigned community builds:

1. Download the DMG from the official GitHub release.
2. Open the DMG.
3. Drag Grimoire to Applications.
4. Launch Grimoire.
5. If macOS blocks the launch, open System Settings > Privacy & Security and choose Open Anyway.

Only bypass Gatekeeper if you trust the release source.

## Local-First Behavior

Grimoire creates local `.grimoire` project folders containing:

- Project metadata
- Palace structure
- Writing items
- Local search chunks
- Wards
- Exports

The app does not include telemetry, analytics SDKs, bundled model weights, hosted sync, or a required cloud account.

## Ollama

Ollama is optional. When available, Grimoire checks the local Ollama server at:

```text
http://127.0.0.1:11434
```

Writing, import, local search, wards, and export should still work when Ollama is missing or has no models.

## Cloud Providers

Cloud providers are stable BYOK options:

- OpenAI
- OpenAI-compatible endpoints
- Google AI Studio

Anthropic support is intentionally hidden in this build so the first public pass can focus on OpenAI and Google AI Studio.

Cloud providers require provider selection, an API key, and explicit disclosure acceptance before a request can send Palace excerpts or Canvas context off-device. API keys are stored through macOS Keychain and must not be exported.

## Development

Requirements:

- Node.js and npm
- Rust with `cargo`
- Tauri macOS prerequisites

Install dependencies:

```bash
npm install
```

Run the app in development:

```bash
npm run tauri dev
```

Run the verification gate:

```bash
npm run verify
```

Build an unsigned local DMG:

```bash
npm run package:mac
```

The DMG is written under:

```text
src-tauri/target/release/bundle/dmg/
```

## Release Process

Current release mode: unsigned community DMG.

Useful docs:

- [Release checklist](docs/release/release-checklist.md)
- [Signing and notarization](docs/release/signing-notarization.md)
- [Release QA](docs/qa/release-qa.md)
- [Privacy](PRIVACY.md)
- [Security](SECURITY.md)
- [Contributing](CONTRIBUTING.md)

## Project Storage

The current development shell creates a demo project at:

```text
~/Documents/Grimoire Projects/Grimoire Demo.grimoire/
```

Future release work should add first-class user project open/create controls before calling the app complete.

## Known Limitations

- Public DMGs are unsigned until an Apple Developer account is available.
- A final packaged-app click-through is still needed before publishing the GitHub release.
- First-class user project open/create controls still need a final product pass; the current development shell creates a demo project.
- Ollama/Co-Writer behavior depends on locally installed user models.
- Cloud provider release QA is currently focused on OpenAI, OpenAI-compatible endpoints, and Google AI Studio; Anthropic is deferred.

## License

MIT. See [LICENSE](LICENSE).
