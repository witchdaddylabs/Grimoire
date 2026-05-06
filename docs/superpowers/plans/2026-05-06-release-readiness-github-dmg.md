# Release Readiness + GitHub DMG Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn Grimoire from a local prototype into an open-source macOS app that can be built, tested, signed, notarized, and distributed as a GitHub Release DMG.

**Architecture:** Keep the app local-first and macOS-first, with Tauri producing the native bundle and GitHub Actions producing release artifacts. Split the work into product completion, automated verification, open-source hygiene, packaging, signing/notarization, and release operations so each phase can be tested independently.

**Tech Stack:** Tauri v2, Rust, React 19, TypeScript, Vite, SQLite, macOS Keychain, GitHub Actions, Apple Developer ID signing, Apple notarization, DMG distribution.

---

## Current Baseline

- `npm run build`: PASS on 2026-05-06.
- `cargo test`: PASS on 2026-05-06 (`10 passed; 0 failed`).
- `cargo check`: PASS on 2026-05-06.
- `npm run tauri build -- --bundles dmg`: PASS on 2026-05-06.
- `npm run verify`: PASS on 2026-05-06 after release scripts were added.
- `npm run package:mac`: PASS on 2026-05-06.
- `hdiutil verify`: PASS on 2026-05-06 for `Grimoire_0.1.0_aarch64.dmg`.
- `cargo test`: PASS after cloud parser hardening on 2026-05-06 (`12 passed; 0 failed`).
- `npm run verify`: PASS after cloud parser hardening on 2026-05-06.
- `npm run verify`: PASS after layout/onboarding/provider UX hardening on 2026-05-06.
- Local DMG artifact exists at `src-tauri/target/release/bundle/dmg/Grimoire_0.1.0_aarch64.dmg`.
- Latest observed SHA-256: `9c0c7b074e5ddce7c2f023dfc98f86e0a913b2c1740cbc5ba5f76f4860b9f066`.
- Gatekeeper check currently fails for public distribution: `spctl` reports `rejected` and `source=no usable signature`.
- Current Tauri config bundles `app` by default, has empty `bundle.icon`, and does not yet carry release signing/notarization configuration.
- QA logs still show onboarding retest pending, many manual interaction checks pending, and Ollama/Co-Writer as partial.
- Product decision on 2026-05-06: cloud BYOK providers must ship as stable in v0.1.0 so users without local models still have an AI option.

## Current Distribution Decision

As of 2026-05-06, the project does not have an Apple Developer account. The near-term release target is therefore an unsigned community DMG published through GitHub Releases with clear Gatekeeper instructions. Developer ID signing and notarization remain the recommended future path for low-friction public installation.

## Release Decision Gates

- A local unsigned DMG is enough for developer testing.
- A public GitHub DMG should be Developer ID signed and notarized to avoid default macOS Gatekeeper rejection.
- Signing/notarization requires human-owned Apple credentials and cannot be fully completed by Codex without those secrets.
- GitHub publishing requires a target repository and GitHub authentication/secrets.

## File Map

- Modify: `src/app/App.tsx` for product completion, onboarding, project flow, and AI UX fixes.
- Modify: `src/styles/global.css` for layout, focus, contrast, responsive desktop behavior, and onboarding polish.
- Modify: `src-tauri/src/main.rs` for native commands, project open/create, export hardening, and testable helpers.
- Modify: `src-tauri/src/ai/mod.rs` and possibly split provider adapters if provider code keeps growing.
- Modify: `src-tauri/tauri.conf.json` for DMG targets, icons, bundle metadata, macOS signing settings, and CSP hardening.
- Modify: `src-tauri/Cargo.toml` for license, repository, and package metadata.
- Modify: `package.json` for build/test/package/release scripts.
- Create: `LICENSE`.
- Create: `CONTRIBUTING.md`.
- Create: `SECURITY.md`.
- Create: `PRIVACY.md`.
- Create: `.github/workflows/ci.yml`.
- Create: `.github/workflows/release.yml`.
- Create: `docs/release/release-checklist.md`.
- Create: `docs/release/signing-notarization.md`.
- Create: `docs/qa/release-qa.md`.

## Task 1: Repository Hygiene And Open Source Metadata

**Files:**
- Modify: `.gitignore`
- Modify: `package.json`
- Modify: `src-tauri/Cargo.toml`
- Create: `LICENSE`
- Create: `CONTRIBUTING.md`
- Create: `SECURITY.md`
- Create: `PRIVACY.md`

- [ ] **Step 1: Pick the final open-source license.**
  Recommended: MIT unless the project wants copyleft. Set the same license in `package.json`, `src-tauri/Cargo.toml`, and `LICENSE`.

- [ ] **Step 2: Remove generated/cache files from git if present.**
  Run: `git ls-files .vite dist src-tauri/target src-tauri/gen`
  Expected: only files that intentionally belong in source remain tracked.

- [ ] **Step 3: Add package metadata.**
  Add repository URL, license, description, author, and homepage fields to `package.json`.

- [ ] **Step 4: Add Cargo metadata.**
  Set `license` and `repository` in `src-tauri/Cargo.toml`.

- [ ] **Step 5: Add contributor docs.**
  `CONTRIBUTING.md` should include setup, test commands, release branch expectations, and how to report app bugs.

- [ ] **Step 6: Add security/privacy docs.**
  `SECURITY.md` should explain vulnerability reporting. `PRIVACY.md` should explain local-first storage, Ollama, BYOK cloud providers, Keychain storage, and exported data boundaries.

- [ ] **Step 7: Verify.**
  Run: `npm run build && cd src-tauri && cargo test && cargo check`
  Expected: all commands pass.

## Task 2: Product Completion Pass

**Files:**
- Modify: `src/app/App.tsx`
- Modify: `src/styles/global.css`
- Modify: `src-tauri/src/main.rs`
- Modify: `src/app/project.ts`
- Modify: `src/app/palace.ts`
- Modify: `docs/qa/m13-manual-qa.md`

- [ ] **Step 1: Finish first-run onboarding.**
  The app must show onboarding on a fresh project, allow feed/engine/wards skips, complete into the shell, persist completion, and expose replay from the top bar.

- [ ] **Step 2: Add real project open/create controls.**
  Replace the hard-coded demo-only experience with user-visible create/open project commands. Keep the demo project path available as a fallback/dev path.

- [ ] **Step 3: Finish Palace basics.**
  Add create item and at least one create container flow, plus persisted expanded/collapsed tree state. Add no-results and empty-tree states that feel intentional.

- [ ] **Step 4: Tighten Canvas persistence.**
  Verify title/body autosave, restart persistence, save failure copy, word count, and export status for real user-created content.

- [ ] **Step 5: Tighten Ollama/Co-Writer UX.**
  Make unavailable/no-model/one-model/multi-model states explicit. Show the active provider/model beside answers. Keep AI output manual-insert only.

- [ ] **Step 6: Run manual M13 QA.**
  Update every row in `docs/qa/m13-manual-qa.md` to PASS, FAIL, PARTIAL, or NOT TESTED with one-line evidence.

## Task 3: M14 Provider Completion And Safety

**Files:**
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/src/ai/mod.rs`
- Modify: `src/app/App.tsx`
- Modify: `src/app/ai.ts`
- Modify: `docs/qa/m14-cloud-provider-qa.md`
- Modify: `README.md`
- Modify: `PRIVACY.md`

- [x] **Step 1: Decide whether M14 cloud providers ship in v0.1.0.**
  Decision on 2026-05-06: ship cloud BYOK providers as stable for v0.1.0.

- [ ] **Step 2: Complete provider matrix testing.**
  Test missing-key, invalid-key, disclosure persistence, offline failure, and no-secret export behavior for each cloud provider UI state.

- [ ] **Step 3: Add tests for provider request builders.**
  Unit-test URLs, headers, and response parsing without performing real network calls.

- [ ] **Step 4: Add secret boundary tests.**
  Confirm API keys are never serialized into project exports or command responses beyond boolean presence.

- [ ] **Step 5: Update docs.**
  README and PRIVACY must state that cloud requests send selected context to the chosen provider under that provider's terms.

## Task 4: Automated Test Harness

**Files:**
- Modify: `package.json`
- Create: `docs/qa/release-qa.md`
- Optional Create: `tests/` or `e2e/` depending on selected harness

- [ ] **Step 1: Add script commands.**
  Add `check`, `test:rust`, `test`, `package:mac`, and `verify` scripts to `package.json`.

- [ ] **Step 2: Keep Rust tests fast and hermetic.**
  Avoid real Ollama, cloud APIs, or Keychain writes in CI tests.

- [ ] **Step 3: Add UI smoke tests if feasible.**
  Use a browser-level smoke test for the Vite app to verify the shell renders, onboarding appears for a fresh browser profile, and core controls are present.

- [ ] **Step 4: Add release QA doc.**
  Track automated test output, DMG build output, codesign/notarization checks, and first-install checks on a clean Mac account.

- [ ] **Step 5: Verify.**
  Run: `npm run verify`
  Expected: one command runs TypeScript build, Rust tests, Rust check, and a local Tauri verification/build step.

## Task 5: Packaging Configuration

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `package.json`
- Add/Modify: `src-tauri/icons/*`
- Create: `docs/release/signing-notarization.md`

- [ ] **Step 1: Configure DMG as a first-class target.**
  Set `bundle.targets` to include `app` and `dmg`, or keep CLI override but document it clearly.

- [ ] **Step 2: Add app icons.**
  Generate and reference a full icon set for macOS. `bundle.icon` must not remain empty.

- [ ] **Step 3: Add release scripts.**
  Add `package:mac` as `tauri build -- --bundles dmg`.

- [ ] **Step 4: Build local unsigned DMG.**
  Run: `npm run package:mac`
  Expected: DMG appears under `src-tauri/target/release/bundle/dmg/`.

- [ ] **Step 5: Validate local artifact.**
  Run: `hdiutil verify <path-to-dmg>`
  Expected: verification succeeds.

## Task 6: Signing And Notarization

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Create: `docs/release/signing-notarization.md`
- Modify: `.github/workflows/release.yml`

- [ ] **Step 1: Confirm Apple prerequisites.**
  Required human inputs: Apple Developer Program membership, Developer ID Application certificate, Team ID, notarization credentials, and a GitHub repository secret strategy.

- [ ] **Step 2: Document local signing verification.**
  Commands should include `security find-identity -v -p codesigning`, `codesign --verify --deep --strict --verbose=2`, and `spctl -a -vvv`.

- [ ] **Step 3: Configure Tauri signing for macOS.**
  Store certificate and password only in the developer keychain locally or GitHub Secrets in CI.

- [ ] **Step 4: Configure notarization.**
  Use Apple notary credentials from secrets. Notarize the exact DMG intended for release and staple the ticket where supported.

- [ ] **Step 5: Validate public-install trust.**
  On a clean Mac user account or separate Mac, download the GitHub DMG, open it, drag Grimoire to Applications, launch, and confirm Gatekeeper accepts it.

## Task 7: GitHub CI

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Add pull request CI.**
  Trigger on pull requests and pushes to `main`.

- [ ] **Step 2: Install Node and Rust.**
  Use `actions/setup-node`, Rust stable, and npm cache.

- [ ] **Step 3: Run checks.**
  Run `npm ci`, `npm run build`, `cargo test`, and `cargo check`.

- [ ] **Step 4: Upload failure logs if needed.**
  Keep artifacts minimal; do not upload secrets, local project DBs, or Keychain material.

## Task 8: GitHub Release Workflow

**Files:**
- Create: `.github/workflows/release.yml`
- Create: `docs/release/release-checklist.md`

- [ ] **Step 1: Trigger releases from version tags.**
  Use `v*` tags such as `v0.1.0`.

- [x] **Step 2: Build Apple Silicon and Intel artifacts.**
  Release workflow is configured to build unsigned `aarch64-apple-darwin` and `x86_64-apple-darwin` DMGs on separate GitHub macOS runners.

- [ ] **Step 3: Upload DMGs to GitHub Releases.**
  Use Tauri's GitHub Action or explicit artifact upload steps.

- [ ] **Step 4: Keep release draft-first.**
  Create draft releases until signing/notarization and smoke installation have passed.

- [ ] **Step 5: Add checksums.**
  Generate SHA-256 checksums for each DMG and attach them to the release.

## Task 9: Install Documentation

**Files:**
- Modify: `README.md`
- Create: `docs/release/release-checklist.md`

- [ ] **Step 1: Rewrite README from scaffold to user-facing app.**
  Lead with what Grimoire does, supported macOS versions, local-first behavior, and install instructions.

- [ ] **Step 2: Add developer setup.**
  Include `npm install`, `npm run dev`, `npm run verify`, and `npm run package:mac`.

- [ ] **Step 3: Add DMG install steps.**
  Download from GitHub Releases, open DMG, drag to Applications, launch.

- [ ] **Step 4: Add Ollama setup notes.**
  Clarify that Ollama is optional, models are user-installed, and writing/export/search work without Ollama.

- [ ] **Step 5: Add release checklist.**
  Include version bump, changelog, CI pass, DMG build, signing, notarization, clean install, and GitHub draft release review.

## Task 10: Final Release Candidate Gate

**Files:**
- Modify: `docs/qa/release-qa.md`
- Modify: `README.md`
- Modify: `docs/qa/m13-manual-qa.md`
- Modify: `docs/qa/m14-cloud-provider-qa.md`

- [ ] **Step 1: Run full automated verification.**
  Run: `npm run verify`
  Expected: all checks pass.

- [ ] **Step 2: Build signed/notarized DMG.**
  Run the release workflow or local release commands.
  Expected: DMG exists and `spctl` accepts it.

- [ ] **Step 3: Run clean install smoke test.**
  Install from the DMG into `/Applications`, launch from Finder, complete onboarding, write/edit/export, and confirm app data is created in the intended user location.

- [ ] **Step 4: Publish GitHub release.**
  Publish only after release notes, checksums, artifacts, and install instructions are verified.

- [ ] **Step 5: Archive release evidence.**
  Record artifact names, checksums, notarization status, test output summaries, and known limitations in `docs/qa/release-qa.md`.

## Minimal Human Inputs Needed

- GitHub repository owner/name and push permission.
- Open-source license choice. Resolved 2026-05-06: MIT.
- Apple Developer Program account for trusted public DMG distribution.
- Developer ID Application certificate or CI signing certificate export.
- Apple notarization credentials, preferably App Store Connect API key configured as GitHub Secrets.
- Product decision: whether v0.1.0 ships cloud BYOK as stable, experimental, or hidden. Resolved 2026-05-06: stable.

## Recommended Execution Order

1. Repository hygiene and open-source docs.
2. Automated test harness and CI.
3. Packaging config and local unsigned DMG.
4. GitHub unsigned release workflow.
5. Stable cloud BYOK hardening and provider QA.
6. Product completion pass for onboarding/project/Canvas/Ollama basics.
7. Clean install release candidate QA.
8. Signing/notarization upgrade when Apple Developer credentials become available.
