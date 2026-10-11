# Windows fork changes

## 0.4.0 — October 11, 2026

Merged upstream Windows/Linux 0.3.0 and 0.4.0 through `e1b2cfd` while retaining the fork's settings, placement, scaling, visibility controls, Codex sessions, allowance and local token accounting.

- Optional hover opening, with keep-open, keep-minimized and pinned views respected.
- Custom sound files, reload controls, individual pill colors and new keyboard shortcuts.
- An Android phone page in Modern and Classic settings, with optional pairing and separate permission controls. The link is off by default; the upstream signed Android APK is still forthcoming.
- Live Codex desktop limits are shared with a paired phone when allowance sharing is enabled.
- Individual Codex colors reach every session pet; the global accent remains available.
- A new prompt or resumed tool activity replaces that chat's previous finished card.

Upstream macOS voice features remain macOS-only.

## 0.1.2 — October 5, 2026

Published the Windows customization work on the `windows-enhancements` branch.

- Codex hooks alongside Claude Code, with separate sessions and approval owners.
- Saved Codex chat titles and links to the corresponding desktop chat.
- Tool/reply history, syntax highlighting, and a Session tab for tokens and context.
- 5H/weekly allowance percentages, reset timers, and local token totals per window.
- Optional minimized allowance, reset timers, and running-task status.
- Modern and Classic settings, with narrower layouts and concise labels.
- Pet/accent colors and independent minimized/expanded size controls.
- Monitor selection, dragging, saved position, pinning, locking, and subtle corner assist.
- Independent keep-open and keep-minimized options, with timed closing modes.
- Optional hidden tray icon while the app remains active.
- Claude, OpenAI, and OpenRouter API chat with securely saved provider keys.
- Native file selection/import, attachment validation, and improved activity layout.
- Minimized limits expand the main view; expanded limits open Codex allowance.
- Windows CI for the frontend, native build, and Codex/Rust behavior checks.

The code is MIT-licensed. Original names, character, artwork, sounds, and media retain the upstream [asset license](LICENSE-ASSETS.md). This publication is a source fork and includes no packaged app release.
