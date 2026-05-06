# AI Provider Readiness Audit

Date: 2026-04-30
Status: M13 audit plus M14 cloud BYOK provider foundation readiness

## Local-First Rule

Grimoire remains local-first. Ollama is the default local provider. Cloud providers are optional BYOK providers and require explicit user configuration, explicit provider selection, the user's API key, and an in-product disclosure before Palace context or Canvas text is sent off-device.

## Current Co-Writer Path

1. Search local SQLite FTS chunks.
2. Build grounded context from retrieved Palace chunks and active Canvas text.
3. Send context to the selected provider. Ollama is local; cloud providers are off-device.
4. Show answer, citations, confidence, and ward warnings.
5. Require a user action before insertion.

## Provider Seam

M14 should route Co-Writer requests through:

```text
ProviderSettings + GroundedPrompt -> AiProviderClient -> AiChatResponse
```

Ollama model discovery must remain dynamic: each launch and refresh should query the local Ollama model list, restore the previous selection only if it still exists, auto-select only when exactly one model exists, and let the user switch in Co-Writer/settings when multiple models are available.

## Secrets Boundary

- API keys must never be stored in SQLite.
- API keys must never be exported.
- API keys, masked keys, and provider secrets must never be included in logs, retrieval records, crash messages, screenshots, error text, export JSON, or Markdown export.
- On macOS, provider keys are stored in Keychain through the Rust secret layer.
- Project export must remain portable without carrying secrets.

## Disclosure Requirement

When a user selects a cloud provider, Grimoire must show:

```text
Cloud model disclosure

You are selecting a cloud model provider. Grimoire will send the prompt, relevant Palace excerpts, and active Canvas context needed for your Co-Writer request to the selected provider.

Your use of this model is subject to that provider's privacy policy, data processing terms, retention rules, and billing terms. Local-first mode remains available through Ollama, where supported by your machine.

Do not use a cloud provider for private, confidential, regulated, or sensitive manuscript material unless you are comfortable with that provider receiving it under its terms.
```

The user must explicitly acknowledge this before the selected cloud provider can be used.

## M14 Provider Targets

- Ollama local
- OpenAI
- OpenAI-compatible custom base URL
- Google AI Studio / Gemini API

Anthropic remains in the internal adapter code but is hidden/deferred for the first public build so QA can focus on OpenAI and Google AI Studio.

## QA Status

- Automated verification: PASS on 2026-04-30 for `npm run build`, `cargo check`, `cargo test`, and `./script/build_and_run.sh --verify`.
- Manual M13 stabilization QA: PENDING running desktop pass.
- Manual M14 provider QA: PENDING provider matrix execution with appropriate missing-key, invalid-key, disclosure, export, and offline states.
