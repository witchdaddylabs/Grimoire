# Privacy

Grimoire is designed as a local-first writing studio.

## Local Project Data

Grimoire stores writing, Grimoire Vault structure, search chunks, wards, and project metadata in a local `.grimoire` project folder on the user's Mac.

The app does not include telemetry, analytics SDKs, bundled model weights, or a hosted sync service.

## Ollama

Ollama is optional. When selected, Grimoire talks to the local Ollama server at `http://127.0.0.1:11434`.

Writing, import, search, wards, and export should continue to work when Ollama is not installed or has no models.

## Cloud Providers

Cloud providers are stable BYOK options for users who do not have local models available or who prefer a cloud model. If a user selects a cloud provider, enters an API key, and accepts the disclosure, Grimoire may send the prompt, relevant Vault excerpts, and active Canvas context needed for that request to the selected provider.

Use of a cloud provider is governed by that provider's privacy policy, data-processing terms, retention rules, and billing terms.

## API Keys

API keys are stored through macOS Keychain. They must not be exported, written to SQLite as raw values, shown after save, or included in logs/error text.

## Exports

Project JSON exports include project metadata, Grimoire Vault content, and wards. They should not include API keys, masked keys, provider secrets, hidden prompts, model binaries, or raw provider responses.
