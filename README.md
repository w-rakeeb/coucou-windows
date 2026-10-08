# Coucou Windows fork

A floating status island for Codex and Claude Code on Windows, with session activity, usage information, and optional API chat.

## About

This is a community source fork of [Louis-CFM/coucou](https://github.com/Louis-CFM/coucou), maintained by [Ra Kib](https://github.com/w-rakeeb). It is independent of the original project.

The `windows-enhancements` branch combines our Windows changes with upstream Windows 0.2.0, through [`2a60e6b`](https://github.com/Louis-CFM/coucou/commit/2a60e6b). The original project remains credited and its licenses are retained.

## Features

| Feature | What it does |
|---|---|
| Codex and Claude Code | Displays sessions, tool activity, completion, and supported approval requests separately. |
| Session details | Shows tool inputs and replies, code highlighting, model/activity, token totals, and context usage. |
| Codex allowance | Shows remaining 5H/weekly percentages, reset timers, and local token totals for each reset window. |
| Minimized view | Optional limits, reset timers, and running-task status. Clicking the limits expands the main view. |
| Appearance | Pet/accent color and separate size sliders for minimized and expanded views. |
| Window controls | Monitor selection, dragging, saved position, always-on-top, position lock, and light corner assist. |
| Visibility | Timed minimize/hide, keep-open, keep-minimized, startup, and an optional hidden tray icon. |
| API chat | Claude, OpenAI, Google AI, OpenRouter, Ollama, LM Studio, and custom compatible servers, with a model picker and streaming replies. |
| File attachments | Native file selection and drag/drop for supported documents, images, text, and code. |
| Settings | Modern pages or Classic layout, with concise labels and narrower windows. |
| More coding agents | Upstream integrations for Cursor, Gemini CLI, Antigravity, Copilot, Muse, OpenCode, Amp, Hermes, and Claude Desktop. |
| Live edits and questions | File diffs, finished-answer ticker, supported question cards, and optional Claude/Codex plan pills. |
| Desktop pet | Drag Mochi onto the desktop and choose seasonal outfits in the wardrobe. |
| Shortcuts and recaps | Configurable shortcuts, weekly recaps and history, and GitHub activity views. |
| Languages | Ten interface languages, including Bengali, with RTL support. |

## Build and run

Requirements: Windows 10/11, Node.js 22+, stable Rust with the MSVC toolchain, Visual Studio Build Tools with **Desktop development with C++**, and Microsoft Edge WebView2 Runtime. See the [official Tauri prerequisites](https://v2.tauri.app/start/prerequisites/#windows).

```powershell
git clone --branch windows-enhancements https://github.com/w-rakeeb/coucou-windows.git
cd coucou-windows/windows
npm ci
npm run tauri -- build --no-bundle
.\target\release\coucou.exe
```

The build also creates `coucou-hook.exe` beside the app. Keep both files together if you copy the build to another folder. This repository publishes source; no packaged app release is provided here.

## Connect your coding sessions

1. Open **Settings → Coding agents**.
2. Choose **Codex** or **Claude Code**, then **Install hooks…**.
3. Review the proposed changes and apply them. Existing handlers are preserved, with a backup before replacement.
4. For Codex, review and trust the new Coucou commands through `/hooks` in Codex CLI, then start a new session.

Codex registration follows `CODEX_HOME` or the default `.codex` folder. Permission decisions require an explicit **Allow** or **Deny** click. When Coucou cannot answer, Codex retains its own approval flow. Tool-hook coverage depends on the Codex version and execution path.

Use **Details → Session** to inspect a Codex chat's token totals and context. Use the expanded header's limit button to open **Codex allowance**.

For API chat, open **Settings → Chat**, select a provider, save its API key, and choose a model available to your account. API chat is separate from session monitoring; it does not send prompts into an existing Codex desktop chat.

See [the Windows guide](windows/README.md) for configuration, behavior, and troubleshooting.

## Usage and privacy

- Allowance percentages are account-wide. Token counters cover Codex logs saved on this computer, including local archived sessions.
- Each token counter follows its own reset window. Cached input is included in input; reasoning output is included in output.
- Usage changes appear when Codex reports them. Local reports are checked every two seconds, and account limits every 30 seconds.
- API keys stay in Windows Credential Manager. Saved keys are never returned to the settings interface or written to preferences.
- Hooks and usage reads do not submit model requests. API chat and configured integrations use their selected services.
- Personal settings, conversation logs, test reports, dependency folders, and generated binaries are excluded from this publication.

## Development

```powershell
cd windows
npm ci
npm run build
npm test
npm run test:codex
cargo test --locked --workspace --lib --bins
npm run tauri -- build --no-bundle
```

[Windows fork checks](https://github.com/w-rakeeb/coucou-windows/actions/workflows/windows-ci.yml) runs the frontend build, Codex behavior checks, Rust tests, and native production build. See [Contributing](CONTRIBUTING.md) and the [Windows changelog](CHANGELOG.windows.md).

## Credits and license

Original Coucou, Mochi, macOS code, character, sounds, and artwork: [Louis Raillé / Louis-CFM](https://github.com/Louis-CFM/coucou). Windows customization in this fork: [Ra Kib / w-rakeeb](https://github.com/w-rakeeb).

Source code is covered by the original [MIT license](LICENSE). Names, character, icons, sounds, and media remain subject to [LICENSE-ASSETS.md](LICENSE-ASSETS.md), which includes restrictions on distributing branded derivatives. Those notices are retained. This source fork does not provide an app release or claim endorsement by the original author.
