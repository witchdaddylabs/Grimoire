# M14 Cloud Provider QA Log

> _Historical evidence from the v0.1.0 macOS release era. In this log "Palace" is today's "Vault", and "macOS Keychain" is now the cross-platform OS credential store (`keyring`). Forward-looking status lives in [get-it-working-plan](../get-it-working-plan.md)._

Date: 2026-04-30
Scope: Cloud BYOK providers for Grimoire

## Baseline

- `npm run build`: PASS on 2026-04-30
- `cargo test`: PASS on 2026-04-30
- `cargo check`: PASS on 2026-04-30
- `./script/build_and_run.sh --verify`: PASS on 2026-04-30

## Scope Notes

- M14 is a contained provider foundation milestone, not a shift away from local-first Grimoire.
- Ollama remains the default local provider and must keep local writing, import, search, wards, and export usable without cloud configuration.
- Cloud providers are BYOK only. This build exposes OpenAI, OpenAI-compatible endpoints, and Google AI Studio. Anthropic is hidden/deferred for a later pass.
- Cloud BYOK providers are intended to ship as stable in v0.1.0 so users without local models still have an AI option.
- Do not mark provider rows passing until testing has been run with the relevant key/state.

## Provider Matrix

| Provider | Key State | Model State | Chat State | Disclosure | Result |
| --- | --- | --- | --- | --- | --- |
| Ollama | n/a | unchecked | unchecked | n/a | pending |
| OpenAI | missing | unchecked | unchecked | pending | pending |
| OpenAI-compatible | missing | unchecked | unchecked | pending | pending |
| Anthropic | hidden/deferred | not in this build | not in this build | pending later | deferred |
| Google AI Studio | valid user key | `gemini-2.5-flash` tested before default update | chat works after import/retrieval | accepted in app | partial pass |

## Release Requirement

Cloud BYOK must be stable before v0.1.0 is complete. At minimum, each provider needs tested missing-key and invalid-key behavior without key leaks, plus at least one successful real-key cloud request verified before release.

## Required Checks

- Ollama unavailable: PENDING
- Ollama one model auto-selects: PENDING
- Ollama multiple models allow user selection: PENDING
- Ollama refresh detects newly downloaded models: PENDING
- Ollama restores previous selection only if that model is still installed: PENDING
- OpenAI missing key: PENDING
- OpenAI invalid key without key leak: PENDING
- OpenAI-compatible missing base URL: PENDING
- OpenAI-compatible invalid base URL/key fails without key leak: PENDING
- Anthropic missing key: DEFERRED (provider hidden in this build)
- Anthropic invalid key without key leak: DEFERRED (provider hidden in this build)
- Google AI Studio missing key: PENDING
- Google AI Studio invalid key without key leak: PENDING
- Cloud disclosure blocks use until accepted: PASS (code audit 2026-04-30; `ai_chat` rejects cloud requests when `disclosureAcceptedAt` is unset)
- Cloud disclosure acceptance persists per provider: PENDING
- Cloud request warning states Palace excerpts and Canvas context go to the selected provider: PASS (code audit 2026-04-30; exact disclosure copy is defined and returned from backend provider settings)
- Network offline failure does not block local writing: PENDING
- Export after cloud provider configuration contains no secrets: PENDING
- API keys absent from SQLite: PASS (local DB inspection 2026-04-30; `settings` currently contains only `ollama.selectedModel`, and API key persistence stores presence flags + Keychain secret)
- API keys absent from logs/error text: PENDING manual provider verification
- API keys stored outside project export data through macOS Keychain: PASS (code audit 2026-04-30; `set_api_key_secret`/`get_api_key_secret` use macOS Keychain)

## Disclosure Copy

```text
Cloud model disclosure

You are selecting a cloud model provider. Grimoire will send the prompt, relevant Palace excerpts, and active Canvas context needed for your Co-Writer request to the selected provider.

Your use of this model is subject to that provider's privacy policy, data processing terms, retention rules, and billing terms. Local-first mode remains available through Ollama, where supported by your machine.

Do not use a cloud provider for private, confidential, regulated, or sensitive manuscript material unless you are comfortable with that provider receiving it under its terms.
```

## Manual Evidence

- 2026-04-30: Automated baseline reconfirmed (`npm run build`, `cargo check`, `cargo test`, `./script/build_and_run.sh --verify` all PASS).
- 2026-04-30: User reran `tauri dev` + `cargo check && cargo test`; launch succeeded and tests passed (`10 passed; 0 failed`).
- 2026-04-30: Static/code audit confirms disclosure gating before cloud chat, default local-first provider fallback, and Keychain-based API key storage.
- 2026-04-30: Local SQLite inspection shows no stored API keys (only `ollama.selectedModel` present in current project `settings`).
- 2026-05-06: Product decision recorded: cloud BYOK providers ship as stable in v0.1.0.
- 2026-05-06: Added automated Rust coverage for OpenAI-compatible, Anthropic, and Gemini response text parsing; `cargo test` PASS (`12 passed; 0 failed`).
- 2026-05-06: User confirmed Google AI Studio key and querying worked after importing text.
- 2026-05-06: Anthropic was hidden/deferred by product decision so the public release can focus on OpenAI and Google AI Studio.
- 2026-05-06: Keychain prompt friction observed; provider status now uses SQLite `apiKeyPresent` flags instead of reading macOS Keychain for every provider listing.
- 2026-05-06: Google default model updated to `gemini-3-flash-preview`; presets include `gemini-3.1-pro-preview` and `gemini-2.5-flash`.
- 2026-05-06: Provider test action added in Engine panel; `npm run verify` PASS.
- Remaining provider matrix rows still require clicked-through manual testing with real provider states (missing/invalid/valid key scenarios).
