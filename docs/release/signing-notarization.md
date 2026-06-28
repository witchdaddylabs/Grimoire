# Signing And Notarization

> _Scoped to the macOS unsigned-DMG release path. The Windows release path (NSIS `.exe`) is tracked in [get-it-working-plan](../get-it-working-plan.md)._

Grimoire can be released in two macOS distribution modes.

## Unsigned Community DMG

This path does not require an Apple Developer account.

Build:

```bash
npm run package:mac
```

Expected artifact:

```text
src-tauri/target/release/bundle/dmg/Grimoire_0.1.0_aarch64.dmg
```

Validate the DMG:

```bash
hdiutil verify src-tauri/target/release/bundle/dmg/Grimoire_0.1.0_aarch64.dmg
```

Expected public-install behavior:

- macOS Gatekeeper may block first launch because the app is unsigned and not notarized.
- Users may need to open System Settings > Privacy & Security and choose Open Anyway.
- This should be described clearly in GitHub release notes.

Current verification command:

```bash
spctl -a -vvv -t open --context context:primary-signature src-tauri/target/release/bundle/dmg/Grimoire_0.1.0_aarch64.dmg
```

Unsigned expected result:

```text
rejected
source=no usable signature
```

## Signed And Notarized DMG

This path requires an Apple Developer Program account.

Required human-owned assets:

- Developer ID Application certificate
- Apple Team ID
- Notarization credentials, preferably App Store Connect API key credentials
- GitHub repository secrets for CI signing, if releases are built in GitHub Actions

Useful local checks:

```bash
security find-identity -v -p codesigning
codesign --verify --deep --strict --verbose=2 /Applications/Grimoire.app
spctl -a -vvv /Applications/Grimoire.app
```

Public release target:

- DMG is signed.
- App is Developer ID signed.
- Notarization succeeds.
- Stapling succeeds where supported.
- `spctl` accepts the downloaded release artifact on a clean Mac user account.

## Current Project Position

As of 2026-05-06, Grimoire can build an unsigned DMG locally. It is suitable for developer testing and adventurous open-source users, but not yet a low-friction mainstream macOS install.
