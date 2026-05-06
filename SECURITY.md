# Security Policy

## Reporting Vulnerabilities

Please report security issues privately to the project maintainers before opening a public issue. If no private contact is configured for the repository yet, open a GitHub issue that asks for a private disclosure channel without including exploit details.

## Security Boundaries

Grimoire is local-first. Project content is stored on the user's Mac in local project files.

API keys for optional cloud providers must be stored in macOS Keychain, not in SQLite, exported project files, logs, or screenshots.

Cloud provider support is BYOK only. Users choose a provider, provide their own key, and accept disclosure before Palace excerpts or Canvas context are sent off-device.

## Supported Versions

Until the first stable release, only the latest GitHub release receives security fixes.
