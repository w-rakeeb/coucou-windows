<div align="center">

<img src="src-tauri/icons/128x128.png" width="96" alt="Coucou icon">

# Coucou for Windows

Community source fork: [w-rakeeb/coucou-windows](https://github.com/w-rakeeb/coucou-windows), based on [Louis-CFM/coucou](https://github.com/Louis-CFM/coucou). The changes in this branch focus on Windows. See the root [README](../README.md) for the feature summary and asset-license notice.

**Mochi doesn't get a notch on a PC — so it lives at the top of your screen instead.**

Approve Claude Code permissions, watch your session work, drop a file, chat with Claude, keep an eye on your services — without leaving what you're doing.

![Windows 10/11](https://img.shields.io/badge/Windows-10%2F11-0078D4?logo=windows)
![Tauri 2](https://img.shields.io/badge/Tauri-2-FFC131?logo=tauri&logoColor=black)
![Rust](https://img.shields.io/badge/Rust-backend-000?logo=rust)
![License: MIT](https://img.shields.io/badge/license-MIT-green)

</div>

<img src="screenshots/greeting.png" width="640" alt="Mochi waving hello at launch">

---

## Install

The downloadable installer is **temporarily unavailable**. Microsoft Defender
wrongly flags the unsigned installer as malware (`Trojan:Win32/Wacatac.H!ml`, a
machine-learning false positive). A report is under review at Microsoft, and the
installer will be published again once it is cleared and code-signed.

Until then, [build it yourself](#build-it-yourself): it takes a few minutes and
installs for the current user only — no admin prompt.

## Using it

<img src="screenshots/compact.png" width="292" alt="The compact island, with the integration pills as mini Mochis">
<img src="screenshots/overview.png" width="640" alt="The overview: the focused integration on the left, the other pills on the right">
<img src="screenshots/approval.png" width="640" alt="A Claude Code permission request, with Deny and Allow">
<img src="screenshots/chat.png" width="640" alt="Chatting with Claude from the island">
<img src="screenshots/drop.png" width="640" alt="Mochi turned into a box, waiting for a file">

| What you do | What happens |
|---|---|
| Move the mouse to the very top-centre of the screen | Mochi peeks out |
| Click the small island | It opens |
| Click Mochi | It gets annoyed. Three times in a row and it goes dizzy |
| Rest the pointer on Mochi for two seconds | Hearts |
| Drag a file onto the island | Mochi turns into a box, swallows it, then offers to answer questions about it |
| `Esc` | Closes the island |
| Tray icon | Open, Settings…, Pause, Quit; optional Hide tray icon setting |

Everything else happens on its own: a Claude Code permission request opens the
island with **Deny / Allow**, a finished session shows what it did, and
your integrations sit in the coloured pills next to Mochi.

## Claude Code

<img src="screenshots/settings.png" width="562" alt="The settings window">

Open **Settings… → Claude Code → Install hooks…**. You get the exact diff of what
will change in `%USERPROFILE%\.claude\settings.json`, the path of the dated backup
that will be taken, and nothing is written until you click. Your own hooks are
never touched, and uninstalling removes only Coucou's entries.

The relay is a tiny executable, `coucou-hook.exe`, copied to
`%LOCALAPPDATA%\Coucou\bin\` at launch. It is given 300 ms to reach Coucou and
exits cleanly if the app is closed, slow or crashed — **a Claude Code session is
never blocked or slowed down by Coucou.** If nobody answers a permission request
in time, Coucou stays quiet and Claude Code asks in the terminal as usual.

It works from any terminal — Windows Terminal, PowerShell, VS Code, Git Bash.

## Codex support (Windows fork 0.1.2)

Codex CLI and local Codex desktop sessions can use the same island for session
activity, tool calls, completion, subagents, interruptions, and Allow / Deny
requests. Claude Code remains available alongside Codex. Each session has its
own task, history, and approval owner; the overview's pills scroll when needed.

Codex sessions use their actual saved chat names instead of generated workspace
folder names. Unnamed sessions show **Codex chat**. The activity counter says
**N steps**, meaning recorded entries for the current turn, not quota or task
completion progress.

The header shows the remaining **5H / Weekly** Codex allowance. Click it to see
the remaining percentages, reset countdowns and tokens used locally in each
current window; **Refresh** checks again. Local session reports are checked every
two seconds, and account limits every 30 seconds (manual account refresh is
throttled to three seconds). Missing limits show **Unavailable**.

The **5H** and **Weekly** token counters sum newly reported usage from local Codex
sessions, including cached input. Each follows its own allowance reset deadline,
returns to zero at reset, and is reconstructed from the logs after a restart.
Repeated usage reports and copied fork history do not count twice. Archived local
sessions are included. These are local token activity totals; remote/cloud
sessions whose logs are absent from this computer cannot be included. Codex's
account allowance percentages remain separate from raw token counts.

This uses the existing Codex sign-in through a short-lived hidden App Server
client. It calls only `initialize`, `account/rateLimits/read`, and `thread/read`
with `includeTurns: false`; it does not start/resume chats or submit turns. No
additional API key or login is installed. The native binary is discovered from
the desktop installation, native npm CLI package, or PATH. No credentials,
account identity, or server error contents are exposed in Coucou logs.

When a reply finishes, the completion card shows **Codex finished**, a short
plain-text preview, **Open Codex**, and **Dismiss**. Open Codex uses the desktop
app's registered `codex://threads/<id>` link to reopen the corresponding local
chat. Dismiss closes the notification; neither button sends a reply or approves
a tool. The overview's open button uses the same chat link for Codex sessions.

Click **Details** on a working session to inspect its received tool inputs and
replies. The larger panel keeps multiline commands, patch text, and input fields
readable; its history buttons select previous events and **Back** returns to the
overview. This uses real hook data, with bounded previews, rather than a simulated
code editor. For a Codex chat, the sidebar also has **Session**: this update,
last response, session total, current model and activity, and context used / size
with the percentage free. **Token breakdown** expands input, cached input, output
and reasoning figures. Cached input is already within input, and reasoning is
already within output. The selected Session tab stays open while activity arrives,
and totals remain isolated when you switch chats. Values come from Codex's own
token reports and update after model responses, without an additional API key or
model request. The compact ticker stays current after the 20-entry history rolls
over, and changing sessions resets its animation.

Open **Settings → Coding agents → Codex → Install hooks…** to review the diff and register the
relay in `~/.codex/hooks.json` (or `CODEX_HOME/hooks.json`). Existing handlers are
preserved, even inside a shared matcher group. A changed file is refused and a
dated backup is saved before replacement. `config.toml`, its existing `notify`
command, and approval policies are not modified.

**Codex requires a separate trust review.** Open `/hooks` in Codex CLI, review
the commands ending in `coucou-hook.exe" --codex <EventName>`, and trust the
Coucou entries. Start a new session afterwards. Coucou never changes Codex's
trust records or bypasses its approval policy. Hook registration alone does not
mean that Codex has enabled the integration.

Only a human's Allow / Deny click returns a decision. If Coucou is closed,
paused, unresponsive, or already displaying another request, the relay prints
nothing and Codex uses its normal approval flow. Unsupported/hosted tool paths
may not generate tool hooks; see the [Codex hook coverage documentation](https://learn.chatgpt.com/docs/hooks).

Built-in chat and file questions support Anthropic, OpenAI, and OpenRouter API
keys. Select the provider under Settings > Chat. This chat is independent
of the Codex session activity integration.

The island cannot send a prompt into an existing Codex desktop chat. Hooks report
activity and return permission decisions; bidirectional chat requires a separate
app-server client and access to the server hosting that chat. The inspected
desktop installation uses a private stdio connection, so this build does not
claim shared live desktop chat support.

The [desktop link commands](https://learn.chatgpt.com/docs/reference/commands)
support opening a chat and prefilling a new composer; they do not submit a
message to an existing chat.

Validation: `npm run test:codex`, `cargo test --workspace --lib --bins`, and
`npm run tauri -- build --no-bundle`.
After registering hooks, `npm run test:codex:launcher` checks the installed
command in PowerShell and Command Prompt with a helper path containing spaces.

## Chat and keys

**Settings → Chat** lets you select Claude, OpenAI, or OpenRouter, save that
provider's API key, and choose its model. OpenAI and OpenRouter accept editable
model IDs. Changing the provider or model starts a fresh chat. Keys use the
**Windows Credential Manager**; saved keys are never returned to the interface
or written to the preferences file. Same for every integration key.

No telemetry. The only network requests Coucou makes are to the services you
configure yourself.

## Build it yourself

You need [Rust](https://rustup.rs), [Node 20+](https://nodejs.org), and the
**MSVC build tools** (Visual Studio Build Tools with "Desktop development with
C++"), and the Microsoft Edge WebView2 Runtime. See the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/#windows).

```powershell
cd windows
npm install
npm run tauri dev      # live-reloading development build
npm run pack           # builds the installer and drops it in windows/release/
```

`npm run dev` alone serves the front end in an ordinary browser, which is enough
to work on the island's looks. It also serves `dev/upload-preview.html`, which
replays the whole file-drop choreography on a loop — the one part of the UI that
otherwise needs a real drag from Explorer to see. Neither page ships in the app.

`npm run pack` leaves two files in `windows/release/`, the same names the release
workflow publishes:

```
Coucou-Windows-X.Y.Z-setup.exe    the versioned installer
Coucou-Windows-setup.exe          the same file under the rolling name
```

Installing is optional — `target/release/coucou.exe` runs on its own. There is no
window in the taskbar and no console: the island at the top of the screen and the
Mochi in the notification area are the whole app, and Quit lives in its menu.

The 28 sounds are the macOS app's own files; they are never duplicated in this
folder. The path is declared once, in `SOUNDS_DIR` at the top of
`vite.config.ts` — when they move to `shared/sounds/`, change that one line.

The app icon and the tray icon are drawn in code, like Mochi itself:

```powershell
npm run icons          # regenerates src-tauri/icons from scripts/gen-icons.mjs
```

### Layout

```
windows/
  src/                 island front end (TypeScript, no framework)
    mochi/             Mochi and the launch greeting, in Canvas 2D
    island/            state machine, hooks, integrations
    views/             every island view
    settings/          the settings window
  src-tauri/           Rust backend: window, named pipe, Claude API, pollers
  hook/                coucou-hook.exe, the Claude Code relay
  scripts/             icon generator
```

### Log

`%LOCALAPPDATA%\Coucou\coucou.log` — hook events, permission decisions, poller
problems. It stays on your machine.

## What's different from the Mac version

- No notch, so the island lives at the top centre of the screen and retracts into
  the top edge instead of hiding in a notch.
- Permission approval works from **any** terminal; the Mac build only listens to
  VS Code sessions.
- Not in this version: sending a file by email, dragging Mochi onto a window to
  attach it as context, and jumping to a specific terminal window — "Open
  terminal" opens the working folder in VS Code when `code` is on your `PATH`.
- Cal.com shows the next bookings as a list rather than the Mac's calendar.

### Appearance and placement

Settings > Appearance includes Minimized size and Expanded size sliders (75–150%, default 100%), Pet & accent color with a Default reset, and separate minimized/expanded reset timers. Classic v1 keeps these under General. Limits and reset text stay white. The chosen color accents the main pet, overview name and dot, selected tabs, activity controls, and allowance bars.

Drag an empty part of the expanded top bar, or the minimized top edge, to move the island with Windows' native dragging. Position is captured once when the drag ends. Center on monitor restores top-center placement on the selected display. Remember position restores the final monitor/position on startup. When disabled, the live position is kept until exit and the next startup centers the island on that display; other preferences still persist.

### Window pin, lock and corner assist

Quick settings includes **Pin** (keep Coucou above other windows), **Lock place** (prevent dragging the current position), and **Assist** (a tiny correction near a monitor corner). All three also appear in General settings and persist across restarts. Pin and Assist default on; Lock defaults off. Existing choices remain saved. Lock disables monitor/Center placement controls. A locked location remains saved even when general remember-placement is off.

Assist only aligns the visible island after the mouse is released, when both edges are within three logical pixels of a monitor corner. It does not pull the island while dragging or align the middle of an edge. Display DPI and transparent window margins are accounted for.

The separate pin in the expanded header keeps that view open. Unpinning resumes the selected minimize delay; the Minimize button remains available. Full settings prefers the space below the visible island, aligned to its right edge, and otherwise chooses a side or above within the monitor's work area.

Minimize transitions interpolate the visual scale and size together while retaining a stable native window envelope. Different compact and expanded size preferences no longer cause an immediate resize jump.

### Settings designs and file attachments

**Settings → Window & behavior → Hide tray icon** hides the notification-area icon immediately while Coucou and its session monitoring keep running. In Classic v1, the toggle is under General. The choice persists at startup. Turn it off to restore the tray icon; when hidden, use the island or launch Coucou's shortcut to open the running app.

Settings opens at a full useful height and prefers a position below the island, aligned with its right edge. If that space is occupied by the monitor boundary, it chooses a side or above. Its bounds remain inside the monitor's work area. The **Settings design** selector switches between **v1 · Classic**, the original stacked interface, and **v2 · Modern**, with separate Appearance, Window & behavior, Chat, Connections and Coding agents pages. The choice is saved; v2 is the default for older preferences that omit it.

The default settings width is 660 pixels for Modern and 580 for Classic, with a 760-pixel preferred height and monitor-bound clamping. Settings labels and help are concise. Clicking the minimized allowance text expands the main island; the allowance page opens from the expanded header's limits button.

Modern Chat settings uses provider tabs, an editable model field and a separate credential area. A saved key is never displayed; Replace key opens an empty field for its replacement. Quick settings shows the selected chat provider, a separate keep-open choice, and the keep-minimized choice. Its minimize presets are disabled while pinned open, and changing the delay updates the actual timer. Hovering the island pauses the timer.

The **Drop** tab offers **Browse files…**, which opens the Windows file picker. Drag-and-drop listens to the island window's native drop events, with an HTML5 file fallback. Selected or dropped PDFs, supported images, plain text and code files are copied into the inbox before becoming chat context. Files over 25 MB and unsupported binary documents receive a clear error; Word documents can be exported as PDF. Selecting a file does not send an API message. The original remains untouched, and repeated filenames create separate copies.

Both open and close use one bounded animation curve for dimensions and visual scale. Layout zoom stays fixed during each transition, with a temporary compositor transform that settles back to crisp layout rendering. A fixed native envelope and in-window anchoring keep all views within the selected monitor, including corners; dragging clamps the visible shape on release independently of the optional corner assist. Native bounds guards preserve the island's intended size and placement outside active dragging and prevent stale, undersized Settings requests from leaving a short popup. Hidden mini pets are skipped during drawing.
