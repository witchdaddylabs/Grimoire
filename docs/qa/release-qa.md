# Release QA

> _Historical evidence from the v0.1.0 macOS release era. In this log "Palace" is today's "Vault", and "macOS Keychain" is now the cross-platform OS credential store (`keyring`). Forward-looking status lives in [README](../../README.md)._

Date: 2026-05-06
Scope: GitHub DMG readiness for Grimoire v0.1.0

## Automated Baseline

- `npm run build`: PASS on 2026-05-06
- `cargo test`: PASS on 2026-05-06 (`10 passed; 0 failed`)
- `cargo check`: PASS on 2026-05-06
- `npm run tauri build -- --bundles dmg`: PASS on 2026-05-06
- `npm run verify`: PASS on 2026-05-06
- `npm run verify`: PASS after cloud parser hardening on 2026-05-06 (`12 passed; 0 failed`)
- `npm run verify`: PASS after layout/onboarding/provider UX hardening on 2026-05-06 (`12 passed; 0 failed`)
- `npm run package:mac`: PASS on 2026-05-06
- `npm run verify`: PASS after launch-bar polish on 2026-05-06 (`14 passed; 0 failed`)
- `npm run package:mac`: PASS after launch-bar polish on 2026-05-06
- `npm run verify`: PASS after broad Palace retrieval and cloud HTTP error copy on 2026-05-06 (`15 passed; 0 failed`)
- `npm run package:mac`: PASS after broad Palace retrieval and cloud HTTP error copy on 2026-05-06
- `npm run verify`: PASS after Anthropic hiding and Keychain prompt mitigation on 2026-05-06 (`16 passed; 0 failed`)
- `npm run package:mac`: PASS after Anthropic hiding and Keychain prompt mitigation on 2026-05-06
- `npm run verify`: PASS after human-QA fixes for Markdown export, Keychain copy, and Focus Mode theme on 2026-05-06 (`16 passed; 0 failed`)
- `npm run package:mac`: PASS after human-QA fixes for Markdown export, Keychain copy, and Focus Mode theme on 2026-05-06

## Artifact Evidence

- Local DMG: `src-tauri/target/release/bundle/dmg/Grimoire_0.1.0_aarch64.dmg`
- Local DMG size observed: 4.6 MB
- SHA-256: `4961c34a2a4f51e2c6275b523f6060fafae7f6de6ea8da930a8d6812c8b14d8d`
- `hdiutil verify`: PASS on 2026-05-06
- `spctl` status: FAIL for public trust, expected for unsigned release (`source=no usable signature`)

## Release Mode

Current release mode: unsigned community DMG.

Reason: no Apple Developer account is currently available for Developer ID signing and notarization.

Cloud BYOK mode: stable release requirement for v0.1.0, focused on OpenAI, OpenAI-compatible endpoints, and Google AI Studio. Anthropic is hidden/deferred in this build.

## Remaining QA

- [x] `npm run verify` after release scripts are added.
- [x] `npm run package:mac` after Tauri config changes.
- [x] `hdiutil verify` on final DMG.
- [x] Real Ollama endpoint lists multiple local models (`ministral-3:8b`, `gemma4:e4b`) on 2026-05-06.
- [x] Real Ollama chat smoke test passes for both listed local models on 2026-05-06.
- [x] Unit coverage for Ollama zero-model, one-model auto-select, multiple-model selection, and stale previous model behavior.
- [x] OpenAI valid-key smoke test passed by user on 2026-05-06.
- [x] OpenAI fake-key failure path observed by user on 2026-05-06; HTTP status handling was improved afterward for clearer invalid-key copy.
- [x] Google AI Studio fake-key failure path observed by user on 2026-05-06; HTTP status handling was improved afterward for clearer invalid-key copy.
- [x] Imported-text Search smoke test passed by user on 2026-05-06.
- [x] Full-app dark/ivory theme accepted by user on 2026-05-06.
- [x] Broad Palace retrieval mode added for Co-Writer after user reported a cross-file recall miss on 2026-05-06.
- [x] Cloud HTTP error handling improved after fake OpenAI/Google keys produced vague empty-response copy.
- [x] Anthropic hidden/deferred for this build so public QA focuses on OpenAI and Google AI Studio.
- [x] Keychain status checks now use SQLite `apiKeyPresent` flags instead of reading macOS Keychain for every provider listing.
- [x] Unit coverage confirms provider settings read stored key presence without a Keychain lookup.
- [x] Markdown item export changed to plain Markdown (`# Title` plus body) after human QA found metadata confusing.
- [x] Plain-language Keychain explanation added beside the cloud key form.
- [x] Focus Mode now stays dark even when the normal app theme is ivory.
- [x] Code audit confirms reduced-motion media query is present.
- [x] Code audit confirms no telemetry SDK/call sites or bundled model weights in app sources.
- [ ] Clean install from final GitHub release artifact.
- [ ] Onboarding file-import smoke test after launch-bar polish.
- [ ] Three-pane scroll/collapse smoke test on final packaged app.
- [x] Full-app ivory/dark theme visual smoke test in dev app.
- [ ] Canvas persistence smoke test across app restart.
- [ ] Markdown export smoke test from final packaged app.
- [ ] Project JSON export smoke test from final packaged app.
- [ ] Ollama smoke test where a local model is available.
- [ ] Cloud BYOK missing-key smoke test for OpenAI and Google AI Studio.
- [ ] Cloud BYOK invalid-key smoke test for OpenAI and Google AI Studio.
- [x] At least one successful cloud request with a user-owned key. OpenAI passed on 2026-05-06.
- [ ] Keychain prompt retest: provider listing/refresh should not repeatedly prompt after keys are saved/deleted.

## Codex-Verified Launch Bar

- Automated build/test/check gate: PASS.
- Unsigned Apple Silicon DMG artifact: PASS.
- DMG checksum and `hdiutil verify`: PASS.
- Gatekeeper signature rejection documented as expected unsigned behavior: PASS.
- Local Ollama two-model endpoint and chat smoke: PASS.
- Zero/one/multiple/stale Ollama model-selection logic: PASS by unit test.
- JSON export secrecy path: PASS by code audit; project export payload contains project metadata, Palace tree/content, and wards only.
- No telemetry and no bundled model weights: PASS by source audit.

## Human Release Smoke Still Required

These are the only items I would not honestly mark fully complete without a human clicked-through packaged-app pass:

- Install the DMG from GitHub Releases rather than the local build folder.
- Confirm first-launch Gatekeeper instructions are understandable.
- Confirm onboarding file import feels obvious and mentions the 10,000-word cap.
- Confirm full-app ivory/dark theme feels good visually on the target Mac.
- Confirm Co-Writer whole-Palace retrieval feels useful on real manuscript material.
- Confirm cloud BYOK missing-key, invalid-key, and successful-key flows with real user-owned keys. OpenAI valid-key and OpenAI/Google fake-key paths have been checked; Anthropic is deferred.
