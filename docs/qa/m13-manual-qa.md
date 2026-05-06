# M13 Manual QA Log

Date: 2026-04-30
Platform: macOS
Scope: Grimoire local-first MVP shell

## Automated Baseline

- `npm run build`: PASS on 2026-04-30
- `cargo check`: PASS on 2026-04-30
- `cargo test`: PASS on 2026-04-30
- `./script/build_and_run.sh --verify`: PASS on 2026-04-30

## Scope Notes

- M13 remains the local-first stabilization and QA milestone.
- M14 provider work may be present in the tree, but M13 manual evidence should not claim cloud provider behavior unless it is explicitly tested.
- Local writing, import, search, wards, export, and Canvas persistence should work without any cloud account or API key.
- Restart persistence means quit/reload the app and confirm the previous project, onboarding completion state, active layout preferences, and saved Canvas edits are still present.
- Save failure copy means the user-facing autosave error should be understandable if persistence fails; this needs a forced-failure test rather than normal happy-path editing.
- Ollama one-model auto-select means if Ollama is reachable and exactly one local model is installed, Grimoire should select it automatically. Multiple-model selection means if more than one model is installed, the user should have to choose one explicitly.

## Onboarding

- Welcome step: PASS (user retest 2026-05-06)
- Palace step project readiness: PASS (user retest 2026-05-06)
- Feed skip: PASS (user retest 2026-05-06)
- Engine skip: PASS (user retest 2026-05-06)
- Wards skip: PASS (user retest 2026-05-06)
- Canvas exit into main shell: PASS (user retest 2026-05-06)
- Feed file import affordance and 10,000-word guidance: IMPLEMENTED / NEEDS USER VISUAL RETEST (file picker and 10,000-word copy added 2026-05-06)
- Restart persistence: PENDING CLARIFICATION / RETEST (definition added above)

## Canvas Persistence

- Title edit autosaves: PASS (user retest 2026-05-06)
- Body edit autosaves: PASS (user retest 2026-05-06)
- Word count updates: LIKELY PASS / NEEDS CONFIRMATION (user observed likely pass 2026-05-06)
- Restart persistence: PENDING CLARIFICATION / RETEST
- Save failure copy: IMPLEMENTED / PENDING FORCED-FAILURE TEST (autosave failure now explains that visible text is not safely saved yet)

## Focus Mode

- Sidebars hide: PASS (user retest 2026-05-06)
- Editor remains usable: PASS (user retest 2026-05-06)
- Word count/save state visible: NEEDS CONFIRMATION
- Ivory manuscript mode remains Canvas-only: IMPLEMENTED / NEEDS USER VISUAL RETEST (theme toggle now applies to the full app shell, not only Canvas)

## Import And Search

- Pasted text import: PASS (user retest 2026-05-06)
- `.txt` import: PASS (user retest 2026-05-06)
- `.md` import: PASS (user retest 2026-05-06)
- Empty import failure: PASS (user retest 2026-05-06)
- Known-term search: PASS (user retest 2026-05-06)
- No-result search: PENDING
- Multi-file import and 10,000-word cap warning: IMPLEMENTED / NEEDS USER RETEST (frontend truncates per import/file and backend rejects over-limit direct command payloads)
- Co-Writer whole-Palace retrieval after import without manual item selection: NEEDS RETEST (post-import active-item loading patched earlier; broader OR-style retrieval and fuller chunk context added after user cross-file recall miss on 2026-05-06)

## Ollama And Co-Writer

- Ollama unavailable: PENDING
- Ollama reachable with no models: PASS BY UNIT TEST / NEEDS LIVE EMPTY-OLLAMA RETEST (message now tells user to install a model with `ollama pull <model>`)
- One-model auto-select: PASS BY UNIT TEST (definition added above)
- Multiple-model selection: PASS BY UNIT TEST AND LIVE ENDPOINT (live Ollama listed `ministral-3:8b` and `gemma4:e4b` on 2026-05-06)
- Previous Ollama selection is restored only when the model is still installed: PASS BY UNIT TEST
- Dynamic model refresh after installing another model: PASS BY LIVE ENDPOINT CHECK (two local models detected on 2026-05-06)
- Grounded answer with citations: PARTIAL / NEEDS RETEST (OpenAI key success confirmed by user; whole-Palace cross-file recall still missed once and was patched with broader retrieval on 2026-05-06)
- Insert/Copy/Rewrite clean/Discard: PENDING

## Wards And Export

- Default wards seeded: PASS (user retest 2026-05-06)
- Add custom ward: PASS (user retest 2026-05-06)
- Remove custom ward: PASS (user retest 2026-05-06)
- Ward warning before insertion: PASS BY CODE AUDIT / NEEDS USER UI RETEST (Co-Writer scans Wards before rendering answer controls; insert label becomes `Insert Anyway` when hits exist)
- Markdown export: PENDING
- Project JSON export excludes secrets/prompts/model binaries: PASS (code audit 2026-05-06; export payload includes project metadata, Palace tree/content, and wards only; API keys remain in macOS Keychain)

## Accessibility, Layout, Network

- Keyboard focus: PASS (user retest 2026-05-06)
- Dark Canvas contrast / full-app theme behavior: IMPLEMENTED / NEEDS USER VISUAL RETEST (theme toggle now affects app shell, panels, tools, and Canvas)
- Ivory Canvas contrast: PASS (user retest 2026-05-06)
- Desktop widths 1100/1280/1440/wide: PARTIAL (1280px MacBook Pro M1 checked by user 2026-05-06; other widths pending)
- Reduced motion: PASS BY CODE AUDIT (global `prefers-reduced-motion: reduce` rule disables animations/transitions)
- No telemetry: PASS (code audit 2026-04-30; no telemetry SDK/call sites found in `src/` or `src-tauri/`)
- No bundled models: PASS (code audit 2026-04-30; no model weight artifacts such as `.gguf`, `.safetensors`, `.onnx`, or `.pt` tracked in project sources)
- No cloud provider calls unless the user explicitly selects a BYOK cloud provider in M14 UI: PASS (code audit 2026-04-30; default provider falls back to Ollama and cloud calls are gated by disclosure + API key)

## AI Provider Readiness

- Local retrieval provider-neutral enough to reuse: PASS (code audit 2026-04-30; `ai_chat` dispatches by provider with shared request shape and grounded context)
- Direct Ollama call locations identified: PASS (`ollama_get_status`, `ollama_select_model`, `ollama_chat`, `fetch_ollama_models`, `chat_ollama` in `src-tauri/src/main.rs`)
- Cloud provider disclosure shown before off-device Palace/Canvas context is sent: PASS (code audit 2026-04-30; `ai_chat` rejects cloud calls unless disclosure is accepted)
- API keys stored outside project export data through macOS Keychain: PASS (code audit 2026-04-30; `set_api_key_secret`/`get_api_key_secret` use macOS Keychain, not SQLite payloads)
- No API keys in SQLite, exports, logs, or error text: PASS (code audit 2026-04-30; export payload includes only project/palace/wards and key presence is boolean-only in settings response)

## Known Blockers

- UI interaction checks still need a clicked-through desktop pass (onboarding flow, Canvas UX, Focus Mode behavior, import/search, Ollama model UX, Wards insertion flow, and visual/accessibility checks).

## This Session Evidence (2026-04-30)

- `npm run build`: PASS
- `./script/build_and_run.sh --verify`: PASS (`Grimoire launched.`)
- `cargo check`: PASS
- `cargo test`: PASS (`10 passed; 0 failed`)
- Local project exists at `~/Documents/Grimoire Projects/Grimoire Demo.grimoire/` with `metadata.json` and `grimoire.sqlite`
- User rerun evidence: `tauri dev` launch PASS and `cargo check && cargo test` PASS (`10 passed; 0 failed`) on 2026-04-30.
- User product feedback: onboarding currently feels non-existent and overall app feels prototype/incomplete; Ollama worked only partially.
- Implementation follow-up: per-project, replayable onboarding was added after the failed report and still needs clicked-through retest.
- Implementation follow-up: three-pane independent scrolling, collapsible Palace/Co-Writer rails, Co-Writer accordions, and functional onboarding actions were added on 2026-05-06; `npm run verify` PASS afterward.

## This Session Evidence (2026-05-06)

- User clicked through M13 manual QA and reported onboarding, Palace readiness, skip steps, Canvas entry, autosave, Focus Mode, import/search, and ward basics as passing where marked above.
- User reported Ivory/day-night behavior needs refinement because the selector currently changes the Canvas rather than the full application theme.
- User reported Co-Writer initially failed to answer from a newly imported file until the imported note was manually selected; implementation follow-up patched post-import active item loading and whole-Palace retrieval fallback on 2026-05-06.
- `npm run verify`: PASS on 2026-05-06 after import/retrieval fixes (`vite build` PASS; Rust tests `12 passed`; `cargo check` PASS).
- Launch-bar implementation follow-up: full-app ivory/dark theme, archive/delete item controls, clearer save-failure copy, Ollama no-model messaging, and Ollama model-selection unit tests were added on 2026-05-06.
- Live Ollama evidence: endpoint listed `ministral-3:8b` and `gemma4:e4b`; both models responded to a minimal chat smoke test on 2026-05-06.
- `npm run verify`: PASS on 2026-05-06 after launch-bar polish (`vite build` PASS; Rust tests `14 passed`; `cargo check` PASS).
- `npm run package:mac`: PASS on 2026-05-06; `hdiutil verify` PASS; `spctl` rejected the DMG as expected for unsigned release (`source=no usable signature`).
- User QA update: OpenAI key succeeded; Search worked on imported text; fake OpenAI and fake Google AI Studio keys produced invalid-key failures; Anthropic valid-key test remains untested because no key is available.
- User QA update: full-app ivory/dark theme is acceptable; Co-Writer whole-Palace retrieval still needed work after missing deliberate information from another Palace file.
- Implementation follow-up: Co-Writer retrieval now uses broad Palace recall mode with stop-word filtering, OR-style FTS terms, and fuller chunk context instead of tiny snippets.
- Implementation follow-up: Wards onboarding now explicitly describes banned words/banned phrases and shows starter pre-fill options.
