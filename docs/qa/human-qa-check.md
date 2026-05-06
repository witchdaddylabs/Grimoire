# Human QA Check

Date: 2026-05-06
Purpose: a simple clicked-through checklist for the remaining human QA before GitHub release.

## Must Test Before GitHub Release

- [ ] Install the latest DMG from the local build or GitHub draft release.
  - Current local DMG: `src-tauri/target/release/bundle/dmg/Grimoire_0.1.0_aarch64.dmg`
  - Confirm macOS unsigned-app instructions are understandable.

- [x] Onboarding file import.
  - Replay onboarding with the top-right info button.
  - Confirmed working in human QA.
  - Needs refinement after release to explain more clearly what Grimoire is doing during import.

- [x] Wards onboarding clarity.
  - Replay onboarding to the Wards step.
  - Improved and acceptable for this pass.
  - Flag for further refinement after release based on user feedback.

- [ ] Co-Writer whole-Palace retrieval.
  - Import or use at least two separate files/items in the Palace.
  - Ask Co-Writer a question whose answer is only in a different Palace item from the active Canvas.
  - Do not manually select the answer file first.
  - Human QA: worked across two separate items, but hung slightly and still struggled unless the relevant drawer was selected.
  - Status: not release-polished; needs follow-up retrieval/scope design work.

- [ ] Co-Writer action buttons.
  - Generate an answer.
  - Test `Insert` or `Insert Anyway`.
  - Test `Copy`.
  - Test `Rewrite clean`.
  - Test `Discard`.

- [ ] Ward warning before insertion.
  - Add a custom banned word/phrase.
  - Generate or paste Co-Writer output that contains it.
  - Confirm Grimoire warns before insertion.
  - Confirm insertion still requires deliberate user action.

- [ ] Markdown export.
  - Select an item.
  - Click `Export Markdown`.
  - Confirm a `.md` file appears in the project `exports` folder.
  - Confirm it is clean Markdown with `# Title` and body text only.
  - Human QA found confusing/random metadata in export; patched to plain Markdown and needs retest.

- [ ] Project JSON export.
  - Click `Export Project`.
  - Confirm JSON appears in the project `exports` folder.
  - Confirm it contains project/Palace/Wards data.
  - Confirm it does not contain API keys, hidden prompts, model binaries, or raw provider responses.

- [ ] Restart persistence.
  - Edit a title and body.
  - Wait for save state to show saved.
  - Collapse/expand sidebars and choose dark/ivory theme.
  - Quit and reopen Grimoire.
  - Confirm edits, onboarding completion, layout preferences, and theme persisted.

- [ ] Keychain prompt friction.
  - Add one cloud API key.
  - Restart Grimoire and switch/refresh providers.
  - Confirm how many macOS Keychain prompts appear.
  - Expected after the latest patch: Grimoire should not prompt just to list providers or refresh model status; Keychain access should be limited to saving/deleting a key or sending a request with the active provider.
  - Human QA: still an issue. App now includes plain-language Keychain explanation near the key form; retest needed.
  - Flag as release blocker if prompts still appear repeatedly instead of only when needed.

- [x] Cloud BYOK missing-key behavior.
  - Select OpenAI and Google AI Studio without saved keys.
  - Human QA confirmed nothing works without the key in place.

- [ ] Cloud BYOK invalid-key behavior.
  - Try fake keys for OpenAI and Google AI Studio again after the latest patch.
  - Confirm the error copy is clearer than `empty response`.

- [ ] Cloud BYOK valid-key behavior.
  - OpenAI: already passed once; rerun if you want final release confidence.
  - Google AI Studio: rerun if you want final release confidence.

## Nice To Check

- [x] Full-app dark/ivory theme still feels acceptable in normal mode.
- [ ] Focus Mode stays dark even when normal mode is ivory.
  - Human QA preference: Focus Mode should be dark. Patched and needs retest.
- [ ] Window width around 1280px remains usable.
- [ ] No-result search shows a clear empty state.
- [ ] Safe Remove hides an item from Palace/search without crashing.
  - Human QA: did not work. Button copy/status improved; needs retest.
- [ ] Delete Item removes an item only after confirmation.
  - Human QA: did not work. Empty-selection failure copy improved; needs retest.

## Known Public Release Friction

- The DMG is unsigned and not notarized because there is no Apple Developer account yet.
- macOS Keychain prompt friction was observed during QA. A patch now avoids reading Keychain just to list provider status; retest is still needed to confirm prompts only appear on save/delete/request.
- Keychain prompts need plain-language explanation; copy has been added near cloud key fields.
- Anthropic is hidden/deferred in this build so QA can focus on OpenAI and Google AI Studio.
- Co-Writer whole-Palace retrieval needs one more human retest with real manuscript/context files.
