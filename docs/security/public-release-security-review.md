# Public Release Security Review

Date: 2026-05-07
Scope: Grimoire public GitHub release tree after removing private planning artifacts and image mockups.

## Summary

No committed API keys, passwords, access tokens, private keys, model binaries, or local project databases were found by the release scan. The removed files were private planning/mockup artifacts rather than detected credential material.

This is a lightweight open-source release review, not a formal OWASP ASVS certification or third-party penetration test.

## Checks Run

- Current-tree secret pattern scan for API keys, passwords, tokens, private keys, GitHub tokens, Google API key patterns, Slack tokens, and AWS access key patterns.
- Git history object-name scan for accidentally included private artifacts.
- Git history text scan for common credential patterns.
- Large-object review to identify bulky or accidental binary artifacts.
- Code audit of BYOK cloud-provider storage and export boundaries.

## Findings

- Removed from public release tree and reachable public history:
  - Root master brief / product / architecture / QA markdown files: `00-master-brief.md` through `08-qa-checklist.md`.
  - `grimoire-image-mockups/`.
  - `grimoire-image-mockups.zip`.
- No obvious real credential values were detected in the current tree or scanned history.
- BYOK API keys are handled through macOS Keychain and not intentionally stored in SQLite project data.
- Project JSON and Markdown exports are designed to exclude API keys, provider secrets, hidden prompts, model binaries, and raw provider responses.
- The app currently has no telemetry path documented for release.

## OWASP Alignment Notes

Relevant OWASP ASVS themes for this desktop-first app:

- Data protection: project exports should exclude secrets and raw provider responses.
- Secrets management: user API keys should live in the operating-system credential store, not in repository files, logs, or portable project exports.
- Error handling and logging: provider errors should not echo secret values.
- Configuration hygiene: release artifacts should avoid local databases, build caches, mockups, private planning documents, and model binaries.
- Dependency hygiene: automated package/Rust checks should be run before release.

References:

- OWASP ASVS: https://owasp.org/www-project-application-security-verification-standard/
- OWASP Top Ten: https://owasp.org/www-project-top-ten/
- OWASP Secrets Management Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Secrets_Management_Cheat_Sheet.html

## Residual Risks

- GitHub may temporarily retain unreachable cached objects after a history rewrite. If an actual secret is ever suspected, rotate it immediately even if scans do not detect it.
- This repository has not had a third-party security assessment.
- The unsigned / unnotarized macOS DMG remains a release-trust limitation until an Apple Developer account is available.
