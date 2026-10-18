<div align="center">

<img src="src-tauri/icons/128x128.png" width="96" alt="Coucou icon">

# Coucou for Windows

Community source fork: [w-rakeeb/coucou-windows](https://github.com/w-rakeeb/coucou-windows), based on [Louis-CFM/coucou](https://github.com/Louis-CFM/coucou). The changes in this branch focus on Windows. See the root [README](../README.md) for the feature summary and asset-license notice.

**Mochi doesn't get a notch on a PC — so it lives at the top of your screen instead.**

Watch Codex and other coding sessions, review supported permissions, drop a file, chat with your selected provider, and follow your services — without leaving what you're doing.

![Windows 10/11](https://img.shields.io/badge/Windows-10%2F11-0078D4?logo=windows)
![Tauri 2](https://img.shields.io/badge/Tauri-2-FFC131?logo=tauri&logoColor=black)
![Rust](https://img.shields.io/badge/Rust-backend-000?logo=rust)
![License: MIT](https://img.shields.io/badge/license-MIT-green)

</div>

<img src="screenshots/greeting.png" width="640" alt="Mochi waving hello at launch">

---

## Install

This fork publishes source. Follow [Build it yourself](#build-it-yourself) below. For the original project's Windows installer, use its [official releases](https://github.com/Louis-CFM/coucou/releases).

## Using it

<img src="screenshots/compact.png" width="292" alt="The compact island, with the integration pills as mini Mochis">
<img src="screenshots/overview.png" width="640" alt="The overview: the focused integration on the left, the other pills on the right">
<img src="screenshots/approval.png" width="640" alt="A Claude Code permission request, with Deny and Allow">
<img src="screenshots/chat.png" width="640" alt="Chatting with Claude from the island">
<img src="screenshots/drop.png" width="640" alt="Mochi turned into a box, waiting for a file">

| What you do | What happens |
|---|---|
| Move the mouse to the very top-centre of the screen | Mochi peeks out |
| Click the small island | It opens. With **Settings → General → Open on hover**, resting the pointer on it is enough, and it folds again shortly after the pointer leaves (click inside to keep it open) |
| Click Mochi | It gets annoyed. Three times in a row and it goes dizzy |
| Rest the pointer on Mochi for two seconds | Hearts |
| Drag a file onto the island | Mochi turns into a box, swallows it, then offers to answer questions about it |
| `Esc` | Closes the island |
| Put a file named like one of Mochi's sounds (`finish.wav`, `approval.mp3`, `greet.m4a`…) in the sounds folder | It replaces that sound after **Settings → General → Reload sounds**. **Open sounds folder** shows the folder: `~/.config/coucou/sounds` on Linux, `%APPDATA%\Coucou\sounds` on Windows |
| Tray icon | Open, Weekly recap, Wardrobe…, Settings…, Pause, Quit; optional Hide tray icon setting |
| `Ctrl+Alt+Space` | Opens the chat, from any app |
| `Ctrl+Alt+A` | Jumps to the waiting permission or question |
| `Ctrl+Alt+T` | Brings the session's window forward ("Open terminal") |
| `Ctrl+Alt+→` / `Ctrl+Alt+←` | Next / previous pill |
| `Ctrl+Alt+S` | Mutes or unmutes Mochi |
| `Ctrl+Alt+G` | Opens the wardrobe |
| `Ctrl+Alt+D` | Sends Mochi to the desktop, or brings him home |
| `Ctrl+Alt+N` | Opens and closes the island (off until you turn it on) |
| In the open island: `Ctrl+→` `Ctrl+←`, `Ctrl+1`–`Ctrl+9` | Switch pills |
| In the open island: `Ctrl+↓` `Ctrl+↑`, `Ctrl+O` | Walk the open GitHub list, open the highlighted row |
| In the open island: `Ctrl+E` | Opens the latest edit's diff, or closes the diff |
| In the open island: `Ctrl+Enter`, `Ctrl+K` | Send, start a new chat |
| In the open island: `Ctrl+,`, `Ctrl+P` | Settings, keep the island open |

### Keyboard shortcuts

Every global shortcut can be changed or turned off in **Settings… → Shortcuts**:
click it and press the new keys. A combination another app already holds is
flagged *In use*, and two Coucou shortcuts on the same keys are flagged *Used
twice*. While you record a new one, Coucou lets go of its own so the keys reach
the recorder.

The defaults are not the Mac's `⌃⌥` letters. On Windows, `Ctrl+Alt` is `AltGr`,
so a global `Ctrl+Alt+E` would swallow every `€` typed on a French or German
keyboard. The defaults were checked against the AltGr layer of the French,
German, Spanish, Italian, Portuguese and Brazilian (ABNT2) layouts — that is why
pill switching uses the arrows rather than `[` `]`, and mute is `S` rather than
`M` (`AltGr+M` is `µ` in German). On top of that, Coucou asks Windows what each
`Ctrl+Alt` combination types on the layouts you have installed and leaves any
that types a character unregistered, flagged *Types “ą”* in Settings: Polish,
for one, puts `ą` on `AltGr+A` and `ś` on `AltGr+S`. The recorder refuses such
a combination too.

Older Intel graphics drivers rotate the screen on `Ctrl+Alt+←` / `→`; if yours
still does, those two show up as *In use*. On Linux, Xfce and MATE keep
`Ctrl+Alt+D` to show the desktop, so there **Send Mochi to the desktop** shows
up as *In use* until you give it other keys.

Everything else happens on its own: a Claude Code permission request opens the
island with **Deny / Allow**, a finished session shows what it did, and
your integrations sit in the coloured pills next to Mochi.

## Eldorado Seller API

In **Settings → Connections**, enable Eldorado and choose **Use Codex API credentials** to import the credentials already protected by your Windows account. Alternatively, save a Client ID and Client secret in that connection. Coucou stores them in Windows Credential Manager and keeps access tokens in memory.

The Eldorado pill shows recent orders. **View** opens Orders, Notifications and Details; new items can open the relevant tab automatically. Configure popups, sound, popup duration and the check interval (15 seconds by default). The first snapshot is silent. Existing connections are preserved, and alerts wait while an agent approval or a composed chat needs attention.

**Details** loads all available order pages on demand (up to 5,000 orders), with a 60-second cache and live updates for recent orders. All / 30 Days / 7 Days filters use the order creation date. Sales include Received and Completed orders, excluding refunds and cancellations; amounts are gross before fees, grouped by currency. Incomplete history is explicitly labelled. Category rankings count sales, not revenue.

This integration reads orders and account notifications only. It does not deliver or cancel orders, mark notifications as read, or read chat conversations. Eldorado does not expose chat messages through the Seller API.

## Claude Code hooks

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
- **Claude** (Pro and Max plans): the numbers come from Claude Code's own status
  line. **Show in notch** first shows you the diff of the `statusLine` change in
  `%USERPROFILE%\.claude\settings.json`, takes a dated backup and writes only
  after your click, with the same writer as the hooks: the status line becomes `coucou-hook --statusline`, which
  passes only the limits on (300 ms at most) and runs the status line you had
  before — kept in `statusline-previous.json` next to the relay — with the same
  input, printing what it prints. On Windows that one runs through Git Bash, as
  Claude Code runs it (`CLAUDE_CODE_GIT_BASH_PATH`, then the Git for Windows that
  `git.exe` on `PATH` belongs to, then the usual install folders); it gets 10 s
  and 64 KB of output. **Uninstall relay** puts your status line back. The
  numbers arrive with Claude Code's replies. They come from the Claude Code
  command-line tool only: the Claude desktop app does not run the status line, so
  if you only use the desktop app the Claude gauge stays on "—".
- **Codex**: nothing is installed. When the pill shows (or is clicked, at most
  once a minute) Coucou starts `codex app-server` and asks it
  `account/rateLimits/read`, as Codex's `/status` does, then stops it (15 s at
  most, never while paused). Codex must be signed in with ChatGPT.

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
- "Open terminal" finds the session's window by walking up from the relay's
  process to the terminal or editor that runs it. A session in a classic
  console window (`cmd.exe` or PowerShell without Windows Terminal) has no such
  ancestor — conhost owns that window — so its folder opens in VS Code instead,
  as it does when `code` is on your `PATH` and nothing was found. Linux walks
  the same tree; what it can bring forward depends on the desktop (see Linux).
- Global keyboard shortcuts are hot keys on Windows, key grabs on Linux under
  X11, and go through the desktop's GlobalShortcuts portal under Wayland (see
  [Linux](#linux)). A waiting card is also folded with its **⌃** or `Esc` in
  the island, and reopened by clicking the island, Open in the tray or **Go to
  alert**.
- Apple Music, the one pill from the Mac catalog with nothing behind it here,
  is left out. Spotify is on Linux only (see [Linux](#linux)): Windows has
  nothing to read it from yet.
- Not in this version: sending a dropped file by email and dragging Mochi onto
  a window to attach it as context. On the Mac, email goes through Resend or
  Apple Mail's scripting; neither has a safe equivalent that attaches a file
  here, and the drop card would need a third button it doesn't have.
- Cal.com shows the next bookings as a list rather than the Mac's calendar.
- The Cursor pill carries both Cursor Agent's own hooks and Claude Code running
  in Cursor's terminal.
- Hermes: Coucou writes the plugin but does not run the `hermes` CLI, so it is
  turned on once by hand. Hermes runs natively on Linux; on Windows it is
  untested.
- Plan usage: the user's previous status line runs through Git Bash on Windows
  (`/bin/sh` on Linux and Mac); without Git Bash it is not run, rather than
  guessed at with `cmd`. The Codex CLI is looked for on `PATH` and in npm's,
  Volta's, Bun's and pnpm's folders (and nvm's on Linux).
- The chat's model picker opens inside the chat card instead of a popover, and
  it also offers OpenRouter and any OpenAI-compatible server, which the Mac
  does not. Google AI, OpenAI and OpenRouter can see an image you dropped (sent
  inline), where the Mac sends its name only.
- Live diff: the relay forwards an edit's text whole only once the edit is done
  (PostToolUse), up to 256 KB per string and 512 KB per event. A bigger edit
  shows its "Edits · file" step without counts rather than wrong ones. The
  diff's ↗ needs `code` on your `PATH`; without it, it opens the file's folder —
  never the file itself. Counts and diffs come from Claude Code's Edit,
  MultiEdit and Write, on whichever pill its session is on (VS Code, Cursor,
  Claude Desktop); other agents' edits show as plain steps.
- There is no iPhone to keep fetching the GitHub lists while the pill is off.
- Keyboard shortcuts use `Ctrl+Alt` where the Mac uses `⌃⌥`, with different
  keys (see [Keyboard shortcuts](#keyboard-shortcuts)), and `Ctrl` where the
  Mac uses `⌘` inside the island. "Bring the terminal forward" is "Open
  terminal" here. Not in this version: attaching the front window (its id is
  kept for later). The island only reads its own shortcuts while it has the
  keyboard: in the chat, or after a global shortcut opened it. `Ctrl+↓`
  `Ctrl+↑` `Ctrl+O` walk the GitHub lists, as on the Mac, and the highlighted
  row scrolls into view where the Mac's three-row list doesn't follow it; in the
  chat field `Ctrl+↑` `Ctrl+↓` keep moving the cursor. **Go to alert** brings
  up any agent's waiting card on its own pill; **Toggle the island** folds a
  waiting card rather than dropping it, like `Esc` in the island. **Send Mochi
  to the desktop** (`Ctrl+Alt+D`, the Mac's `⌃⌥D`)
  flies him to his spot, or to the bottom-right corner when that spot was on a
  display that is gone; pressed again, he flies home.
- Weekly recap:
  - Sharing happens inside the island instead of a separate panel, and Save
    writes straight into Pictures (or Downloads) instead of asking where. There
    is no system Share sheet; Copy uses the web clipboard.
  - Questions are counted when an agent asks one (`AskUserQuestion`), whether
    it is then answered in the island or in the terminal. Permissions count the
    Allow and Deny clicks on the island's card, for every agent that gets one.
  - There is no sleep/wake notification to listen to without a background
    loop, so after the machine wakes the Monday card waits for the first agent
    to start or for you to hover the island.
  - The card is 24 px taller than the Mac's: it also lists top agent, project,
    busiest day, longest session, permissions and questions, which the Mac
    leaves to the image. A turn cut short by the session ending still counts.
  - Tray → Pause doesn't stop the history (it stays on the machine anyway);
    switch it off in Settings → General.
- The wardrobe opens with a right-click on Mochi, from the tray menu, or with
  its global shortcut (`Ctrl+Alt+G` by default). In the compact island a tall hat
  is cut by the top edge of the screen, as it is by the notch on a Mac.
- Languages: chosen in Settings, independently of the system, and applied
  without a restart (the Mac's **Restart Coucou** isn't needed). Arabic turns
  the island's text right to left but not its layout: Mochi and the pills keep
  their sides.
- Mochi on the desktop dances only on Linux, to Spotify; on Windows there is
  no music integration to dance to yet. While he dances he stays awake (the
  Mac lets him doze off mid-dance). Dropping him on a window doesn't attach it
  to the chat. While he
  sleeps, the transparent square around him (120 px) takes the first mouse
  move, which wakes him and gives the rest back to the desktop.

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
- **The island** is a gtk-layer-shell overlay anchored to the top edge, over any
  top panel, on compositors that support it: COSMIC, KDE Plasma, Hyprland, Sway
  and other wlroots compositors. GNOME has no layer-shell and ignores where a
  Wayland window asks to go, so there Coucou runs through XWayland as a dock
  window: top centre, on every workspace, still there after Super+D.
  `COUCOU_X11=0` keeps the native Wayland window, `COUCOU_DOCK=0` makes it a
  utility window instead of a dock. `COUCOU_LAYER_SHELL=0` forces the regular
  window anywhere.
- **Click-through** is the window's input region, kept equal to the island
  shape, so the compositor sends every other click to what is underneath.
- **Mochi's eyes** follow the pointer only while it is over the island: Wayland
  gives no app the cursor position anywhere else. On the desktop, likewise,
  they follow it only while it is over him, and "the cursor is far away" (so
  he may fall asleep) means it hasn't been over him for a few seconds.
- **Mochi on the desktop** is a layer-shell surface on the island's display
  (KDE Plasma, COSMIC, Hyprland, Sway…), placed with margins and dragged within
  that display; on X11 it is an ordinary always-on-top window that goes
  anywhere. **GNOME on Wayland** has no layer-shell and lets no app place its
  own window, so there Mochi can't leave the island: dragging him does nothing,
  and the desktop shortcut only makes him grumble.
- **Claude Code hooks** go through `~/.local/share/coucou/bin/coucou-hook` and a
  Unix socket at `$XDG_RUNTIME_DIR/coucou.sock`. Both ends check that the other
  runs as the same user. Every other agent uses the same relay, single-quoted
  for `sh`, and its config under `~` (see Supported agents). A config that is a
  symlink (dotfiles) is written through to its target, with its permissions
  kept.
- **Global shortcuts** are X11 key grabs in an X11 session.
  On X11, AltGr is a modifier of its own and never clashes with `Ctrl+Alt`;
  combinations the desktop already uses (GNOME's `Ctrl+Alt+T` terminal and
  `Ctrl+Alt+←`/`→` workspace switching) show up as *In use*.
  Wayland has no key grabs, so there Coucou hands its shortcuts to the desktop
  through the XDG **GlobalShortcuts portal** (`xdg-desktop-portal`, with a
  backend that has it: KDE Plasma 6, GNOME 48+, Hyprland, COSMIC…). Each
  shortcut goes with its description and your keys as the preferred trigger;
  the desktop may show its own window to confirm them or to pick other keys,
  and it has the last word: a shortcut it already knows keeps the keys it was
  given there, and you change them in the desktop's own shortcut settings.
  **Settings → Shortcuts** says the shortcuts are registered with the desktop,
  shows next to each one the keys the desktop reports (*Desktop: …*), and flags
  one it left out. Changing or turning off a shortcut in Coucou binds the new
  set in a new portal session. Coucou names itself `fr.louisraille.coucou` to
  the portal where the portal allows it (`xdg-desktop-portal` 1.19+); a
  desktop that wants an app name may otherwise refuse.
  Where there is no such portal, or the desktop refuses, nothing is registered
  and **Settings → Shortcuts** lists commands to bind in your
  desktop's own keyboard settings instead (they are shown with the portal too,
  for any shortcut that doesn't work):
  `coucou --shortcut openChat` (or the AppImage's path) runs the action in the
  Coucou that is already open. The ids are `toggleIsland`, `openChat`,
  `goToAlert`, `jumpToTerminal`, `nextPill`, `prevPill`, `muteToggle`,
  `desktopToggle` and `wardrobeToggle`.
- **Keys** live in the Secret Service (GNOME Keyring, KWallet).
- **Spotify** (Settings → Integrations) is read over MPRIS, Spotify's D-Bus
  interface on the session bus: the pill shows the track and plays, pauses
  or skips on hover, the card has the cover, the progress bar (drag to
  seek), shuffle, previous, next, repeat and the volume, and Mochi dances
  while it plays — in the compact island, on the Spotify card, on the pill
  and on the desktop. Nothing is polled: one thread waits on the bus for
  Spotify's own signals while the pill is declared, and stops when it isn't.
  **Open Spotify** brings it forward or starts `spotify` from your `PATH`,
  else the Flatpak (`com.spotify.Client`) or the Snap; without one, **Get
  Spotify** opens its download page. Covers come from Spotify's image CDN
  (or a local track's file) and are fetched by Coucou itself, not the page.
  Spotify builds that don't report the position over MPRIS show the bar
  from where the track started; the Mac's Automation prompt has no
  equivalent here.
- **Plan usage**: the status line relay is `~/.local/share/coucou/bin/coucou-hook
  --statusline` and runs your previous status line with `/bin/sh -c`, like Claude
  Code. Codex is found on `$PATH`, in `~/.local/bin`, npm's global prefix, Volta,
  Bun, pnpm, or nvm (newest Node first), since a desktop launch often has a
  shorter `$PATH` than your shell.
- **Mochi's greeting** uses the full name in your account's GECOS field
  (`chfn` sets it); without one the chat stays neutral.
- **Files**: preferences in `~/.config/coucou/`, the log at
  `~/.local/share/coucou/coucou.log`, the weekly recap history beside it in
  `recap.json`. A saved recap image goes to the pictures folder named in
  `~/.config/user-dirs.dirs`, else `~/Pictures`, else `~/Downloads`.
- **Languages**: Hindi, Bengali, Chinese and Arabic need fonts that carry those
  scripts; Coucou asks for the Noto families (`fonts-noto-core` and
  `fonts-noto-cjk` on Debian and Ubuntu, `noto-fonts` and `noto-fonts-cjk` on
  Arch). **System** reads the language from the webview, which follows
  `LANGUAGE` / `LANG`.
- **Copying the recap image** needs a WebKitGTK with image clipboard support;
  where it is missing, the island says so and Save still works.
- What the Windows build leaves out, this one does too: sending a file by
  email and dragging Mochi onto a window.
- **Open terminal** walks up from the relay through `/proc` (stopping at
  systemd, logins, ssh, session managers and panels, and at any process of
  another user) to the terminal or editor the session runs in, then brings its
  window forward — the one titled after the session's folder if it has several:
  - **X11** (any desktop): EWMH — the window whose `_NET_WM_PID` is the nearest
    ancestor gets a `_NET_ACTIVE_WINDOW` request, on its workspace. A terminal
    that doesn't set `_NET_WM_PID` (some old `xterm` builds) isn't found.
  - **KDE Plasma on Wayland**: a few lines of KWin script, loaded over D-Bus
    from `$XDG_RUNTIME_DIR`, activate the window by process ID and are unloaded
    right after; Plasma 5 and 6.
  - **kitty**, anywhere: its tab is selected too when remote control listens on
    a socket (`allow_remote_control` and `listen_on` in kitty.conf).
  - **GNOME on Wayland** and other Wayland compositors let no app bring another
    app's native window forward (an XWayland one, such as VS Code's by default,
    still can be). There, and for a session under tmux, screen or ssh, whose
    terminal is no ancestor, the folder opens in VS Code as before.
- No **Claude Desktop** pill: the Claude app has no Linux build.
