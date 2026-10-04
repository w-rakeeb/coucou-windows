# Contributing to Coucou

Thanks for wanting to help Mochi grow up! 🫶

## Getting started

For this fork's Windows changes, use the `windows-enhancements` branch and work in `windows/`:

```powershell
npm ci
npm run build
npm run test:codex
cargo test --workspace --lib --bins
npm run tauri -- build --no-bundle
```

Keep provider sessions and approval owners separate, preserve existing hooks, and store API keys in Windows Credential Manager. Include behavior checks for session routing, token totals, or native placement changes. Source-only contributions retain the original credits and [asset license](LICENSE-ASSETS.md).

The original macOS workflow is below.

```bash
brew install xcodegen
cd NotchBuddy && xcodegen && open NotchBuddy.xcodeproj
```

Never edit `NotchBuddy.xcodeproj` by hand: change `project.yml` and run `xcodegen`.

## Good first contributions

- A new integration (a poller + a pill + a detail card). Look at `StripePoller.swift` for a compact example.
- A new emote or sound for Mochi.
- Bug fixes — please describe how to reproduce.

## macOS guidelines

- Swift 6, SwiftUI + AppKit, **no third-party dependencies** unless there's really no other way.
- Secrets go in the Keychain, never on disk or in git.
- No telemetry, no network calls except to services the user configured.
- Never block Claude Code: if the app doesn't answer, the hook must exit right away.
- Never write `~/.claude/settings.json` without a backup and the user's confirmation.
- Keep it light: 0 % CPU when the island is hidden.

## Pull requests

- One topic per PR, with a short GIF or screenshot for anything visual.
- Build must pass with no new warnings.
