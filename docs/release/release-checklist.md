# Release Checklist

Use this checklist for each GitHub release.

## Before Tagging

- [ ] `npm install` has been run with a clean lockfile.
- [ ] `npm run verify` passes.
- [ ] `npm run package:mac` creates a DMG.
- [ ] `hdiutil verify <dmg>` passes.
- [ ] `docs/qa/release-qa.md` has current evidence.
- [ ] README install instructions match the release artifact.
- [ ] Release notes call out whether the DMG is unsigned or signed/notarized.
- [ ] Known limitations are listed honestly.

## Unsigned Release Notes

Include this text when publishing without Apple notarization:

```text
This DMG is unsigned and not notarized. macOS may block first launch. To open it, go to System Settings > Privacy & Security after the first blocked launch and choose Open Anyway. Only install if you trust this project and downloaded the DMG from the official GitHub release.
```

## Tag And Release

```bash
git tag v0.1.0
git push origin v0.1.0
```

The release workflow should create a draft release with Apple Silicon and Intel DMGs plus SHA-256 checksums attached.

## Clean Install Smoke Test

- [ ] Download the DMG from the GitHub release.
- [ ] Open the DMG.
- [ ] Drag Grimoire to Applications.
- [ ] Launch from Finder.
- [ ] Complete or replay onboarding.
- [ ] Edit title/body.
- [ ] Confirm save state reaches saved.
- [ ] Export Markdown.
- [ ] Export project JSON.
- [ ] Confirm exports contain no secrets.
- [ ] If Ollama is installed, refresh models and run one Co-Writer request.

## Promote Release

- [ ] Draft release has correct version.
- [ ] DMG artifact is attached.
- [ ] SHA-256 checksum is attached or included.
- [ ] Install warning is accurate for signed/unsigned status.
- [ ] Release is published.
