// Island views — DOM ports of IslandViewContent.swift. Paddings, font sizes,
// colours and wording are copied from the Swift views so both platforms read
// identically.

import { h, svg, clear, dot } from "./dom";
import { ICONS } from "./icons";
import { Ticker } from "./ticker";
import { renderActivity } from "./activity-code";
import { renderSession, tokenNumber, exactTokens } from "./session";
import { State, isCodingAgent, providerName, type AgentTask } from "../core/state";
import { washRGBA, type IslandViewName, type Wash } from "../core/layout";
import { createMiniBot, pruneMiniBots } from "../mochi/minibots";
import { buildPrompt } from "./chat";
import { buildChoose, buildUpload, buildUploading } from "./upload";
import { refreshCodexInfo, refreshCodexTelemetry, remaining, resetText, windowLabel, shortReset } from "../island/codex-info";
import { renderIntegrationCard, type IntegrationCardHooks } from "./integrations";

export interface ViewActions {
  setView(v: IslandViewName): void;
  collapse(): void;
  setFocus(id: string): void;
  openTerminal(): void;
  /** The ↗ button: opens whatever the focused pill points at. */
  openTarget(): void;
  openUrl(url: string): void;
  decide(d: "allow" | "deny"): void;
  toggleSound(): void;
  toggleTopmost(): void;
  toggleKeepOpen(): void;
  toggleKeepMinimized(): void;
  browseFile(): void;
  togglePositionLock(): void;
  toggleEdgeSnap(): void;
  setVolume(v: number): void;
  setAutoClose(seconds: number): void;
  openSettingsWindow(): void;
  blip(): void;
}

export interface ViewHost {
  el: HTMLElement;
  sync(): void;
  /** Called when the view becomes active, for views with a text field. */
  focus?(): void;
  /** Called every frame while the view is on screen. */
  tick?(nowMs: number): void;
  animating?(): boolean;
}

// ── Shared pieces ─────────────────────────────────────────────────────────────

function card(wash: Wash, ...children: (Node | string)[]): HTMLElement {
  const el = h("div", { class: wash ? "card wash" : "card" }, ...children);
  if (wash) el.style.setProperty("--wash", washRGBA(wash));
  return el;
}

function btn(
  label: string,
  kind: "primary" | "secondary",
  onClick: () => void,
  kbd?: string,
): HTMLElement {
  return h(
    "button",
    { class: `btn ${kind}`, onclick: onClick },
    h("span", { text: label }),
    kbd ? h("span", { class: "kbd", text: kbd }) : null,
  );
}

/** AgentWho — coloured dot + task name + grey label. */
function agentWho(task: AgentTask | null, label: string): HTMLElement {
  const row = h("div", { class: "who-row" });
  if (task) {
    row.append(dot(/^#[0-9a-f]{6}$/i.test(State.settings.petColor) ? State.settings.petColor : task.color, 8), h("span", { class: "n", text: task.name }));
  }
  row.append(h("span", { text: label }));
  return row;
}

function stack(padLeft: number, padRight: number, ...children: Node[]): HTMLElement {
  const el = h("div", { class: "stack" }, ...children);
  el.style.padding = `4px ${padRight}px 4px ${padLeft}px`;
  return el;
}

// ── Header ────────────────────────────────────────────────────────────────────

export function buildHeader(actions: ViewActions): ViewHost {
  const tabHome = h("button", { class: "tab", title: "Overview", onclick: () => go("overview") }, svg(ICONS.house, 13));
  const tabChat = h("button", { class: "tab", title: "Ask", onclick: () => go("prompt") }, svg(ICONS.bubble, 13));
  const tabDrop = h("button", { class: "tab", title: "Drop", onclick: () => go("upload") }, svg(ICONS.plus, 13));

  const gearBtn = h("button", { title: "Settings", onclick: () => go("settings") }, svg(ICONS.gear, 14));
  const soundBtn = h("button", { title: "Mute", onclick: () => actions.toggleSound() }, svg(ICONS.speakerOn, 14));
  const minimizeBtn = h("button", { title: "Minimize", "aria-label": "Minimize", onclick: () => actions.collapse() }, svg("M4 12h16", 14, { stroke: 2 }));
  const keepOpenBtn = h("button", { title: "Keep expanded view open", "aria-label": "Keep expanded view open", onclick: () => actions.toggleKeepOpen() }, svg(ICONS.pin, 13));
  const limitsBtn = h("button", { class: "codex-limits", title: "Codex allowance remaining", onclick: () => { go("usage"); void refreshCodexInfo(State.focusTask?.source === "codex" ? State.focusTask.sessionId ?? null : null, true); } });

  function go(v: IslandViewName) {
    actions.blip();
    actions.setView(v);
  }

  const el = h(
    "div",
    { id: "header" },
    h("div", { class: "tabs" }, tabHome, tabChat, tabDrop),
    h("div", { class: "header-actions" }, limitsBtn, keepOpenBtn, gearBtn, soundBtn, minimizeBtn),
  );

  return {
    el,
    sync() {
      const v = State.view;
      const quota = State.codexInfo;
      keepOpenBtn.classList.toggle("on", State.settings.keepExpanded);
      keepOpenBtn.setAttribute("aria-pressed", String(State.settings.keepExpanded));
      keepOpenBtn.title = State.settings.keepExpanded ? "Pinned open · click to use the closing timer" : "Keep expanded view open";
      limitsBtn.textContent = quota?.error ? "Codex limits unavailable" : quota
        ? `${windowLabel(quota.primary)} ${remaining(quota.primary)} · ${windowLabel(quota.secondary, true)} ${remaining(quota.secondary)} left`
        : "Codex limits …";
      if (quota && !quota.error && State.settings.expandedReset) limitsBtn.textContent = `${windowLabel(quota.primary)} ${remaining(quota.primary)} ↻${shortReset(quota.primary)} · Weekly ${remaining(quota.secondary)} ↻${shortReset(quota.secondary)}`;
      limitsBtn.title = quota?.error ?? "Account-wide Codex allowance remaining. Click for reset times.";
      tabHome.classList.toggle("on", v === "overview" || v === "empty");
      tabChat.classList.toggle("on", v === "prompt");
      tabDrop.classList.toggle("on", v === "upload");
      gearBtn.classList.toggle("on", v === "settings");
      clear(gearBtn);
      gearBtn.append(svg(v === "settings" ? ICONS.gearFill : ICONS.gear, 14));
      clear(soundBtn);
      soundBtn.append(svg(State.settings.soundEnabled ? ICONS.speakerOn : ICONS.speakerOff, 14));
      el.style.opacity = v === "confused" ? "0" : "1";
    },
  };
}

// ── Overview ──────────────────────────────────────────────────────────────────

function buildOverview(actions: ViewActions): ViewHost {
  const ticker = new Ticker();
  const who = h("div", { class: "who" });
  const tickerBody = h("div", { class: "card-body" }, who, ticker.el);
  const leftBody = h("div", { class: "left-body" });
  const jump = h(
    "button",
    { class: "icon-btn jump", title: "Open", onclick: () => actions.openTarget() },
    svg(ICONS.arrowUpRight, 8),
  );
  const details = h("button", { class: "activity-open", text: "Details", title: "View session activity details", onclick: () => actions.setView("activity") });
  const left = card(null, leftBody, jump, details);
  const pills = h("div", { class: "pills" });
  const right = card(null, pills);

  const el = h("div", { class: "view overview" },
    h("div", { class: "left" }, left),
    h("div", { class: "right" }, right),
  );

  let pillIds = "";
  let detailOpen = false;
  let lastFocus: string | null = null;
  let mode: "ticker" | "card" | null = null;
  let cardKey = "";

  const hooks: IntegrationCardHooks = {
    get detailOpen() {
      return detailOpen;
    },
    openDetail() {
      detailOpen = true;
      cardKey = "";
      State.notify();
    },
    closeDetail() {
      detailOpen = false;
      cardKey = "";
      State.notify();
    },
    openSettings: () => actions.openSettingsWindow(),
  };

  return {
    el,
    tick(nowMs: number) {
      if (mode === "ticker") ticker.tick(nowMs);
    },
    animating: () => mode === "ticker" && ticker.animating,
    sync() {
      const task = State.focusTask;
      if (task?.id !== lastFocus) {
        lastFocus = task?.id ?? null;
        detailOpen = false;
        cardKey = "";
        mode = null;
      }

      // VS Code with a live Claude Code session keeps the ticker; every other
      // pill shows its own card, exactly like IntegrationCardView.
      const sessionActive =
        !!task && isCodingAgent(task) && (task.state !== "idle" || task.steps.length > 0);
      details.hidden = !sessionActive;

      if (task && sessionActive) {
        if (mode !== "ticker") {
          clear(leftBody);
          leftBody.append(tickerBody);
          mode = "ticker";
          cardKey = "";
        }
        clear(who);
        who.append(
          dot(task.color, 7),
          h("span", { class: "name", text: task.name, title: task.name }),
          h("span", { class: "tool", text: providerName(task) }),
        );
        if (task.steps.length > 1) {
          who.append(h("span", {
            class: "count",
            text: `${task.steps.length} steps`, title: "Activity recorded for this turn",
          }));
        }
        ticker.sync(task);
      } else if (task) {
        const info = State.integrations[task.id];
        const key = [
          task.id, detailOpen, task.state, task.steps.join("|"),
          info?.loaded, info?.error, info?.configured,
          JSON.stringify(info?.data ?? {}),
        ].join("~");
        if (key !== cardKey) {
          cardKey = key;
          mode = "card";
          clear(leftBody);
          leftBody.append(renderIntegrationCard(task, hooks));
        }
      }

      jump.style.display = detailOpen ? "none" : "";

      const others = State.otherTasks;
      const pillKey = others.map((t) => `${t.id}:${t.name}:${t.pillBadge ?? ""}`).join("|");
      if (pillKey !== pillIds) {
        pillIds = pillKey;
        clear(pills);
        for (const t of others) pills.append(buildPill(t, actions));
        pruneMiniBots();
      }
    },
  };
}

function buildPill(task: AgentTask, actions: ViewActions): HTMLElement {
  const label = isCodingAgent(task) ? task.sessionId ? task.name : providerName(task) : task.name;
  const canvas = createMiniBot(task, 24);
  const pill = h(
    "div",
    { class: "pill", title: `${isCodingAgent(task) ? `${providerName(task)} · ` : ""}${label}${task.sessionId ? ` · ${task.sessionId.slice(0, 8)}` : ""}`, onclick: () => actions.setFocus(task.id) },
    canvas,
    h("span", { class: "lbl", text: label }),
  );
  pill.style.borderColor = `${task.color}24`;
  pill.addEventListener("mouseenter", () => {
    pill.style.background = `${task.color}2e`;
    pill.style.borderColor = `${task.color}8c`;
    pill.style.boxShadow = `0 2px 10px ${task.color}59`;
    (pill.querySelector(".lbl") as HTMLElement).style.color = lighten(task.color, 0.3);
  });
  pill.addEventListener("mouseleave", () => {
    pill.style.background = "";
    pill.style.borderColor = `${task.color}24`;
    pill.style.boxShadow = "";
    (pill.querySelector(".lbl") as HTMLElement).style.color = "";
  });

  if (task.pillBadge) {
    const colors = { approval: "#F5A524", finished: "#22C55E", error: "#F4505E" } as const;
    const icons = { approval: ICONS.bang, finished: ICONS.check, error: ICONS.xmark } as const;
    const inner = h("i", { style: `background:${colors[task.pillBadge]}` }, svg(icons[task.pillBadge], 6, { stroke: task.pillBadge === "finished" ? 3 : 0 }));
    const badge = h("div", { class: "pill-badge" }, inner);
    badge.style.boxShadow = `0 0 4px ${colors[task.pillBadge]}99`;
    pill.append(badge);
  }
  return pill;
}

function lighten(hex: string, amount: number): string {
  const v = parseInt(hex.replace("#", ""), 16);
  const c = [(v >> 16) & 255, (v >> 8) & 255, v & 255].map((x) =>
    Math.min(255, Math.round(x + amount * 255)),
  );
  return `rgb(${c[0]},${c[1]},${c[2]})`;
}

// ── Empty ─────────────────────────────────────────────────────────────────────

function buildEmpty(actions: ViewActions): ViewHost {
  const body = h(
    "div",
    { class: "stack", style: "padding:0 18px 0 118px;flex-direction:row;align-items:center;gap:16px" },
    h(
      "div",
      { style: "display:flex;flex-direction:column;gap:5px" },
      h("div", { class: "title", text: "Nothing running right now." }),
      h("div", { class: "sub", text: "Drop a file or window, or ask me anything." }),
    ),
    h("div", { class: "grow" }),
    btn("Ask Mochi", "primary", () => actions.setView("prompt")),
  );
  return { el: h("div", { class: "view" }, card(null, body)), sync() {} };
}

// ── Approval ──────────────────────────────────────────────────────────────────

function buildApproval(actions: ViewActions): ViewHost {
  const who = h("div");
  const code = h("div", { class: "code" });
  const row = h("div", { class: "actions" });
  const el = h("div", { class: "view" }, card("amber", stack(116, 16, who, code, row)));
  let rowKey = "";
  return {
    el,
    sync() {
      clear(who);
      const owner = State.tasks.find((t) => t.id === State.pendingApproval?.taskId) ?? State.focusTask;
      who.append(agentWho(owner, `${owner ? providerName(owner) : "Agent"} needs permission`));
      // The whole point of approving here rather than in the terminal: this line
      // is the command, the file path or the URL being authorised, not just the
      // name of the tool asking.
      code.textContent = State.pendingApproval?.command || State.pendingApproval?.tool || "…";
      code.title = code.textContent;
      // Two buttons, built once. Rebuilding them between a mouse-down and a
      // mouse-up would swallow the click, and there is nothing left to vary:
      // "Always" is gone until the remembered-rules list exists to back it.
      if (rowKey === "built") return;
      rowKey = "built";
      clear(row);
      row.append(
        btn("Deny", "secondary", () => actions.decide("deny"), "N"),
        btn("Allow", "primary", () => actions.decide("allow"), "Y"),
      );
    },
  };
}

// ── Question ──────────────────────────────────────────────────────────────────

function buildQuestion(): ViewHost {
  const who = h("div");
  const title = h("div", { class: "title" });
  const row = h("div", { class: "actions" });
  const el = h("div", { class: "view" }, card("cyan", stack(116, 16, who, title, row)));
  return {
    el,
    sync() {
      clear(who);
      const task = State.focusTask;
      const provider = task ? providerName(task) : "Agent";
      who.append(agentWho(task, `${provider} is asking a question`));
      title.textContent = task?.steps.at(-1) ?? `${provider} needs an answer.`;
      clear(row);
      row.append(h("div", { class: "sub", text: task?.source === "codex" ? "Answer in the Codex app." : "Answer in your terminal." }));
    },
  };
}

// ── Error ─────────────────────────────────────────────────────────────────────

function buildError(actions: ViewActions): ViewHost {
  const who = h("div");
  const title = h("div", { class: "title", text: "Workflow stopped." });
  const detail = h("div", { class: "detail" });
  const row = h("div", { class: "actions" },
    btn("Retry", "primary", () => actions.setView(State.defaultView())),
    btn("Open in n8n", "secondary", () => actions.openUrl("")),
  );
  const el = h("div", { class: "view" }, card("red", stack(116, 16, who, title, detail, row)));
  return {
    el,
    sync() {
      const task = State.focusTask;
      clear(who);
      who.append(agentWho(task, task ? providerName(task) : "Agent"));
      title.textContent = task?.source === "n8n" ? "Workflow stopped." : "Session stopped on an error.";
      detail.textContent = task?.steps.at(-1) ?? "No detail available.";
    },
  };
}

// ── Finished ──────────────────────────────────────────────────────────────────

function buildActivity(actions: ViewActions): ViewHost {
  const project = h("div", { class: "activity-project" });
  const tools = h("div", { class: "activity-tools" });
  const heading = h("div", { class: "activity-heading" });
  const content = h("div", { class: "activity-content" });
  const back = h("button", { class: "activity-back", text: "Back", onclick: () => actions.setView("overview") });
  const el = h("div", { class: "view" }, card(null,
    h("div", { class: "activity-card" },
      h("div", { class: "activity-sidebar" }, project, tools),
      h("div", { class: "activity-pane" }, h("div", { class: "activity-toolbar" }, heading, back), content))));
  let selected: number | "session" = -1;
  let key = "";
  let taskKey = "";
  let sessionKey = "";
  let renderedEntry: AgentTask["activity"] extends (infer T)[] | undefined ? T | undefined : never;
  const host: ViewHost = { el, sync() {
    const task = State.focusTask;
    const entries = task?.activity ?? [];
    const nextKey = `${task?.id}:${task?.turnId}:${entries.length}:${entries.at(-1)?.detail}`;
    if (taskKey !== task?.id) {
      taskKey = task?.id ?? ""; selected = entries.length ? entries.length - 1 : task?.source === "codex" ? "session" : -1;
      sessionKey = ""; renderedEntry = undefined; clear(content);
    }
    if (key !== nextKey) { key = nextKey; if (selected !== "session") selected = entries.length - 1; }
    project.textContent = task?.name ?? "Session";
    project.title = `${task?.name ?? "Session"} · ${task ? providerName(task) : "Agent"} · ${task?.sessionCwd ?? ""}`;
    clear(tools);
    if (task?.source === "codex") tools.append(h("button", {
      class: `activity-tool session-tab${selected === "session" ? " selected" : ""}`, text: "Session",
      title: "Session tokens and context", onclick: () => { selected = "session"; sessionKey = ""; host.sync(); void refreshCodexTelemetry(); },
    }));
    entries.forEach((entry, index) => tools.append(h("button", {
      class: `activity-tool${index === selected ? " selected" : ""}`,
      text: entry.label, title: entry.detail, onclick: () => { selected = index; sessionKey = ""; renderedEntry = undefined; host.sync(); },
    })));
    const selectedButton = tools.querySelector<HTMLElement>(".selected");
    if (selectedButton) {
      const selectedBounds = selectedButton.getBoundingClientRect();
      const listBounds = tools.getBoundingClientRect();
      tools.scrollTop += Math.min(0, selectedBounds.top - listBounds.top)
        + Math.max(0, selectedBounds.bottom - listBounds.bottom);
    }
    if (selected === "session") {
      heading.textContent = "Session";
      const session = State.codexTelemetry?.sessions[task?.sessionId ?? ""] ?? null;
      const nextSessionKey = JSON.stringify([task?.id, task?.state, session]);
      if (sessionKey !== nextSessionKey) { sessionKey = nextSessionKey; renderSession(content, task, session); }
      return;
    }
    const entry = entries[selected];
    heading.textContent = entry?.label ?? "Session activity";
    if (renderedEntry !== entry || !content.childNodes.length) {
      renderedEntry = entry;
      renderActivity(content, entry);
      content.scrollTop = 0;
    }
  } };
  return host;
}

function buildUsage(actions: ViewActions): ViewHost {
  const rows = h("div", { class: "usage-rows" });
  const status = h("div", { class: "usage-status", title: "Allowance percentages are account-wide. Token totals cover sessions stored on this PC and reset with each allowance window." });
  const refresh = h("button", { class: "activity-back", text: "Refresh", onclick: () => { void refreshCodexInfo(null, true); void refreshCodexTelemetry(); } });
  const back = h("button", { class: "activity-back", text: "Back", onclick: () => actions.setView("overview") });
  const el = h("div", { class: "view" }, card(null, h("div", { class: "usage-body" },
    h("div", { class: "usage-heading" }, h("span", {text:"Codex allowance"}), refresh, back), rows, status)));
  return {el, sync() {
    const quota = State.codexInfo;
    clear(rows);
    for (const [window, weekly] of [[quota?.primary ?? null, false], [quota?.secondary ?? null, true]] as const) {
      const fill = h("span", { class:"usage-fill" });
      fill.style.width = window ? `${Math.max(0, 100 - window.usedPercent)}%` : "0%";
      const usage = weekly ? State.codexTelemetry?.secondary : State.codexTelemetry?.primary;
      const expired = !!window?.resetsAt && Date.now() >= window.resetsAt * 1000;
      const matches = !!usage && !!window?.resetsAt && Math.abs(usage.resetsAt - window.resetsAt) <= 90;
      const total = expired ? 0 : matches ? usage!.totalTokens : null;
      const tokens = h("div", { class: "usage-tokens", text: total == null ? window?.resetsAt ? "Checking local tokens…" : "Token window unavailable" : `${tokenNumber(total)} tokens used locally`,
        title: total == null ? "Tokens are reconstructed from local Codex logs." : `${exactTokens(total)}\nInput: ${exactTokens(expired ? 0 : usage?.inputTokens ?? 0)}\nCached input: ${exactTokens(expired ? 0 : usage?.cachedInputTokens ?? 0)}\nOutput: ${exactTokens(expired ? 0 : usage?.outputTokens ?? 0)}\nCached input is already included in the total.` });
      if (matches && !expired) tokens.append(h("span", { text: `${usage!.sessions} ${usage!.sessions === 1 ? "session" : "sessions"}` }));
      rows.append(h("div", { class:"usage-row" },
        h("div", { class:"usage-label" }, h("span", {text:windowLabel(window,weekly)}), h("span", {text:window ? `${remaining(window)} left` : "Unavailable"})),
        h("div", { class:"usage-track" }, fill), tokens, h("div", {class:"usage-reset", text:resetText(window)})));
    }
    status.textContent = quota?.error ?? (quota ? "Local sessions · Cached input included · Counters reset with each window" : "Checking your Codex account…");
  }};
}

function buildFinished(actions: ViewActions): ViewHost {
  const who = h("div");
  const title = h("div", { class: "title finished-summary" });
  const open = btn("Open terminal", "primary", () => actions.openTerminal());
  const row = h("div", { class: "actions" },
    open,
    btn("Dismiss", "secondary", () => actions.collapse()),
  );
  const body = card("green", stack(116, 16, who, title, row));
  body.classList.add("finished-card");
  const el = h("div", { class: "view" }, body);
  return {
    el,
    sync() {
      clear(who);
      const task = State.focusTask;
      who.append(agentWho(task, `${task ? providerName(task) : "Agent"} finished`));
      open.querySelector("span")!.textContent = task?.source === "codex" ? "Open Codex" : "Open terminal";
      open.title = task?.source === "codex" ? "Open this chat in the Codex app" : "Open the working folder";
      title.textContent = (task?.steps.at(-1) ?? "Session finished")
        .replace(/\[([^\]]+)\]\([^)]+\)/g, "$1")
        .replace(/^\s*(?:#{1,6}|[-*])\s+/gm, "")
        .replace(/\*\*|__|`/g, "")
        .replace(/\s+/g, " ").trim();
    },
  };
}

// ── Confused ──────────────────────────────────────────────────────────────────

function buildConfused(): ViewHost {
  const body = h(
    "div",
    { class: "stack", style: "padding:0 18px 0 128px" },
    h("div", { class: "title", text: "Too many hits at once." }),
    h("div", { class: "sub", text: "Give me a sec — back to work in three seconds." }),
  );
  return { el: h("div", { class: "view" }, card("pink", body)), sync() {} };
}

// ── Note ──────────────────────────────────────────────────────────────────────

function buildNote(): ViewHost {
  const title = h("div", { class: "title" });
  const el = h("div", { class: "view" }, card(null, h("div", { class: "stack", style: "padding:0 18px 0 98px" }, title)));
  return {
    el,
    sync() {
      title.textContent = State.noteMessage ?? "";
    },
  };
}

// ── In-island settings ────────────────────────────────────────────────────────

function buildSettings(actions: ViewActions): ViewHost {
  const soundSwitch = h("button", { class: "switch", onclick: () => actions.toggleSound() });
  const pin = h("button", { class: "quick-toggle", "aria-label": "Pin always on top", title: "Keep Coucou above other windows", onclick: () => actions.toggleTopmost() }, svg(ICONS.pin, 12), "Pin");
  const lock = h("button", { class: "quick-toggle", "aria-label": "Lock position", title: "Lock the current position", onclick: () => actions.togglePositionLock() }, svg(ICONS.lock, 12), "Lock position");
  const snap = h("button", { class: "quick-toggle", "aria-label": "Gentle corner assist", title: "Align on release only within 3 pixels of a monitor corner", onclick: () => actions.toggleEdgeSnap() }, svg(ICONS.magnet, 12), "Assist");
  const volume = h("input", {
    type: "range", min: "0", max: "0.2", step: "0.005",
    oninput: (e: Event) => actions.setVolume(Number((e.target as HTMLInputElement).value)),
  }) as HTMLInputElement;
  const autoLabel = h("span", {});
  const segButtons = [10, 15, 30].map((s) =>
    h("button", { onclick: () => actions.setAutoClose(s) }, `${s}s`),
  );
  const providerBadge = h("span", { class: "status-badge" });
  const visible = h("button",{class:"quick-toggle", "aria-label":"Keep minimized visible", onclick:()=>actions.toggleKeepMinimized()}, "Keep minimized");
  const keep = h("button",{onclick:()=>actions.toggleKeepOpen()},"Keep open");

  const rows = h(
    "div",
    { class: "settings-rows" },
    h("div", { class: "settings-row" }, soundSwitch, h("span", { text: "Sound" }), volume, h("div", { class: "grow" }), pin, lock, snap),
    h(
      "div",
      { class: "settings-row" },
      svg(ICONS.timer, 12),
      autoLabel,
      h("div", { class: "seg" }, ...segButtons, keep),
    ),
    h(
      "div",
      { class: "settings-row", style: "gap:14px" },
      providerBadge,
      visible,
      h("div", { class: "grow" }),
      h("button", {
        class: "link-btn",
        style: "color:#8e939c;font-size:11.5px",
        text: "Settings…",
        onclick: () => actions.openSettingsWindow(),
      }),
    ),
  );

  const el = h("div", { class: "view" },
    card(null, h("div", { class: "stack", style: "padding:14px 16px 14px 84px" }, rows)));

  return {
    el,
    sync() {
      const s = State.settings;
      soundSwitch.classList.toggle("on", s.soundEnabled);
      for (const [button, on] of [[pin, s.alwaysOnTop], [lock, s.positionLocked], [snap, s.edgeSnap]] as const) {
        button.classList.toggle("on", on);
        button.setAttribute("aria-pressed", String(on));
      }
      volume.value = String(s.soundVolume);
      volume.style.opacity = s.soundEnabled ? "1" : "0.4";
      autoLabel.textContent = s.keepExpanded ? "Keep open" : State.isPinned ? "Waiting for your response" : `Minimize after ${Math.round(s.autoCloseInterval)}s`;
      segButtons.forEach((b, i) => { b.classList.toggle("on", !s.keepExpanded && s.autoCloseInterval === [10,15,30][i]); (b as HTMLButtonElement).disabled=s.keepExpanded || State.isPinned; });
      keep.classList.toggle("on",s.keepExpanded);keep.setAttribute("aria-pressed",String(s.keepExpanded));
      visible.classList.toggle("on",s.keepMinimized);visible.setAttribute("aria-pressed",String(s.keepMinimized));visible.title=s.keepMinimized?"Minimized view stays visible":"Minimized view hides after "+Math.round(s.minimizeHideInterval)+"s";
      providerBadge.textContent = "Chat · " + ({anthropic:"Claude",openai:"OpenAI",openrouter:"OpenRouter"}[s.chatProvider] ?? "Claude");

    },
  };
}

// ── Placeholders filled in later stages ───────────────────────────────────────

function buildPlaceholder(title: string, sub: string): ViewHost {
  const body = h(
    "div",
    { class: "stack", style: "padding:0 18px 0 118px" },
    h("div", { class: "title", text: title }),
    h("div", { class: "sub", text: sub }),
  );
  return { el: h("div", { class: "view" }, card(null, body)), sync() {} };
}

// ── Registry ──────────────────────────────────────────────────────────────────

export function buildViews(
  actions: ViewActions,
  onChatHeightChange: () => void,
): Map<IslandViewName, ViewHost> {
  const map = new Map<IslandViewName, ViewHost>();
  map.set("overview", buildOverview(actions));
  map.set("activity", buildActivity(actions));
  map.set("usage", buildUsage(actions));
  map.set("empty", buildEmpty(actions));
  map.set("approval", buildApproval(actions));
  map.set("question", buildQuestion());
  map.set("error", buildError(actions));
  map.set("finished", buildFinished(actions));
  map.set("confused", buildConfused());
  map.set("note", buildNote());
  map.set("settings", buildSettings(actions));
  map.set("prompt", buildPrompt(onChatHeightChange));
  map.set("upload", buildUpload(actions));
  map.set("uploading", buildUploading());
  map.set("choose", buildChoose(actions));
  // Not in the Windows v1: sending a file by email, window attach + web result.
  map.set("mail", buildPlaceholder("Sending by email isn't in this version.", ""));
  map.set("searching", buildPlaceholder("Claude is searching…", ""));
  map.set("result", buildPlaceholder("Result", ""));
  return map;
}
