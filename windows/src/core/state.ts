// App state — mirror of AppState.swift (the parts the island needs).

import type { BotEmoteName, BotStateName, IslandMode, IslandViewName } from "./layout";
import type { EyeShape } from "../mochi/engine";

export type AgentSource = "claudeCode" | "codex" | "n8n";
export function isCodingAgent(task: AgentTask | null | undefined): boolean {
  return task?.source === "claudeCode" || task?.source === "codex";
}
export function providerName(task: AgentTask): string {
  return task.source === "codex" ? "Codex" : task.source === "claudeCode" ? "Claude Code" : "n8n";
}
export type PillBadge = "approval" | "finished" | "error";

export interface AgentTask {
  id: string;
  name: string;
  color: string;
  state: BotStateName;
  stepIndex: number;
  steps: string[];
  source: AgentSource;
  isIntegration: boolean;
  emote?: BotEmoteName | null;
  miniEye?: EyeShape | null;
  pillBadge?: PillBadge | null;
  sessionCwd?: string | null;
  sessionId?: string;
  chatTitle?: string;
  turnId?: string;
  stepRevision?: number;
  activity?: { label: string; detail: string; input?: Record<string, unknown> }[];
}

export interface ApprovalInfo {
  requestId: string;
  sessionId: string;
  tool: string;
  command: string;
  taskId: string;
}

export interface ChatMessage {
  id: number;
  role: "user" | "assistant";
  content: string;
}

export type PromptContext =
  | { kind: "window"; appName: string; title: string; url?: string }
  | { kind: "file"; name: string; path?: string };

export interface ResultItem {
  label: string;
  detail: string;
  url?: string;
}

export interface SearchResult {
  title: string;
  items: ResultItem[];
  note?: string;
}

const task = (
  id: string, name: string, color: string, source: AgentSource,
): AgentTask => ({
  id, name, color, state: "idle", stepIndex: 0, steps: [], source, isIntegration: true,
});

/** AgentTask.integrationAgents — same ids, names and colours as macOS. */
export const INTEGRATION_AGENTS: AgentTask[] = [
  task("integration_claude", "Claude Code", "#F5F6F8", "claudeCode"),
  task("integration_codex", "Codex", "#A8D8CC", "codex"),
  task("integration_resend", "Resend", "#22C55E", "n8n"),
  task("integration_n8n", "n8n", "#F29B38", "n8n"),
  task("integration_vercel", "Vercel", "#7C5CFF", "n8n"),
  task("integration_github", "GitHub", "#F4505E", "n8n"),
  task("integration_notion", "Notion", "#8C8C8C", "n8n"),
  task("integration_calcom", "Cal.com", "#C9956A", "n8n"),
  task("integration_stripe", "Stripe", "#0570DE", "n8n"),
];

export const TOGGLEABLE_INTEGRATION_IDS = [
  "integration_resend", "integration_n8n", "integration_vercel", "integration_github",
  "integration_notion", "integration_calcom", "integration_stripe",
];

/** What an integration poller last reported. */
export interface IntegrationInfo {
  data: Record<string, unknown>;
  error: string | null;
  loaded: boolean;
  configured: boolean;
}

export interface Settings {
  settingsInterface: "v1" | "v2";
  chatProvider: "anthropic" | "openai" | "openrouter";
  openaiModel: string;
  openrouterModel: string;
  alwaysOnTop: boolean;
  positionLocked: boolean;
  edgeSnap: boolean;
  petColor: string;
  compactScale: number;
  expandedScale: number;
  rememberPlacement: boolean;
  expandedReset: boolean;
  compactLimits: boolean;
  compactActivity: boolean;
  compactReset: boolean;
  keepExpanded: boolean;
  keepMinimized: boolean;
  minimizeHideInterval: number;
  freePlacement: boolean;
  positionX: number;
  positionY: number;
  soundEnabled: boolean;
  soundVolume: number;
  autoCloseInterval: number;
  absenceInterval: number;
  activeIntegrations: string[];
  screen: string;
  autostart: boolean;
  hideTrayIcon: boolean;
  hooksInstalled: boolean;
  codexHooksInstalled: boolean;
  /** Claude model used by the chat. */
  model: string;
}

export const DEFAULT_SETTINGS: Settings = {
  settingsInterface: "v2",
  chatProvider: "anthropic",
  openaiModel: "gpt-4.1-mini",
  openrouterModel: "openai/gpt-4.1-mini",
  alwaysOnTop: true,
  positionLocked: false,
  edgeSnap: true,
  petColor: "",
  compactScale: 1,
  expandedScale: 1,
  rememberPlacement: true,
  expandedReset: true,
  compactLimits: true,
  compactActivity: true,
  compactReset: false,
  keepExpanded: false,
  keepMinimized: false,
  minimizeHideInterval: 60,
  freePlacement: false,
  positionX: 0.5,
  positionY: 0,
  soundEnabled: true,
  soundVolume: 0.12,
  autoCloseInterval: 15,
  absenceInterval: 180,
  activeIntegrations: [
    "integration_resend", "integration_n8n", "integration_vercel", "integration_github",
  ],
  screen: "primary",
  autostart: false,
  hideTrayIcon: false,
  hooksInstalled: false,
  codexHooksInstalled: false,
  model: "claude-opus-5",
};

type Listener = () => void;

export interface CodexLimitWindow { usedPercent: number; windowDurationMins: number; resetsAt: number | null }
export interface CodexInfo { primary: CodexLimitWindow | null; secondary: CodexLimitWindow | null; checkedAt: number; error: string | null; threadId: string | null; title: string | null }
export interface CodexTokens { inputTokens: number; cachedInputTokens: number; outputTokens: number; reasoningOutputTokens: number; totalTokens: number }
export interface CodexSession {
  threadId: string; model: string | null; reasoningEffort: string | null; branch: string | null;
  total: CodexTokens | null; last: CodexTokens | null; lastUpdateTokens: number | null;
  contextUsed: number | null; modelContextWindow: number | null; updatedAt: number | null;
}
export interface CodexWindowUsage extends CodexTokens { responses: number; sessions: number; resetsAt: number; expired: boolean }
export interface CodexTelemetry { primary: CodexWindowUsage | null; secondary: CodexWindowUsage | null; sessions: Record<string, CodexSession>; checkedAt: number }

class AppState {
  mode: IslandMode = "hidden";
  renderScale: number | null = null;
  view: IslandViewName = "overview";

  tasks: AgentTask[] = [];
  focusId: string | null = null;

  stateOverride: BotStateName | null = null;

  /** Cursor in logical screen pixels, origin top-left (like AppState.mousePosition). */
  mouse = { x: 0, y: 0 };
  /** Cursor relative to the island's top-left corner. */
  mouseInIsland = { x: 0, y: 0 };

  isPinned = false;
  paused = false;

  uploadProgress = 0;
  uploadDuration = 2.4;
  fileDragOver = false;

  promptContext: PromptContext | null = null;
  droppedFile: { name: string; path: string } | null = null;
  noteMessage: string | null = null;
  searchResult: SearchResult | null = null;
  chatHistory: ChatMessage[] = [];
  pendingApproval: ApprovalInfo | null = null;
  codexInfo: CodexInfo | null = null;
  codexTelemetry: CodexTelemetry | null = null;

  integrations: Record<string, IntegrationInfo> = {};

  lastActivity = performance.now();

  settings: Settings = { ...DEFAULT_SETTINGS };

  private listeners = new Set<Listener>();

  subscribe(fn: Listener): () => void {
    this.listeners.add(fn);
    return () => this.listeners.delete(fn);
  }

  /** Marks the UI dirty; the island re-renders on the next frame. */
  notify() {
    for (const fn of this.listeners) fn();
  }

  get focusTask(): AgentTask | null {
    return this.tasks.find((t) => t.id === this.focusId) ?? this.tasks[0] ?? null;
  }

  get effectiveState(): BotStateName {
    return this.stateOverride ?? this.focusTask?.state ?? "idle";
  }

  get otherTasks(): AgentTask[] {
    return this.tasks.filter((t) => t.id !== this.focusId).sort((a, b) => {
      const priority = (t: AgentTask) => t.pillBadge === "approval" ? 0 : isCodingAgent(t) && t.state !== "idle" ? 1 : 2;
      return priority(a) - priority(b);
    });
  }

  get activeChats(): AgentTask[] {
    return this.tasks.filter(task => isCodingAgent(task) && !!task.sessionId &&
      ["working", "thinking", "searching", "approval", "question", "ratelimit"].includes(task.state))
      .sort((a, b) => Number(b.state === "approval") - Number(a.state === "approval"));
  }

  /** Each session owns its state, even when Claude and Codex work concurrently. */
  sessionTask(source: "claudeCode" | "codex", sessionId: string, cwd: string): AgentTask {
    const baseId = source === "codex" ? "integration_codex" : "integration_claude";
    const found = this.tasks.find((t) => t.source === source && t.sessionId === sessionId);
    if (found) { if (cwd) found.sessionCwd = cwd; return found; }
    let base = this.tasks.find((t) => t.id === baseId);
    if (!base) {
      base = { ...INTEGRATION_AGENTS.find((t) => t.id === baseId)!, steps: [] };
      this.tasks.push(base);
    }
    const session = base.sessionId
        ? { ...base, id: `${baseId}:${sessionId}`, chatTitle: undefined, steps: [], activity: [], stepRevision: 0, stepIndex: 0, state: "idle" as const, pillBadge: null }
      : base;
    session.sessionId = sessionId;
    session.sessionCwd = cwd;
    session.turnId = undefined;
    if (session !== base) this.tasks.push(session);
    return session;
  }

  endSession(id: string) {
    const current = this.tasks.find((t) => t.id === id);
    if (!current) return;
    if (id === "integration_claude" || id === "integration_codex") {
        current.state = "idle"; current.steps = []; current.stepIndex = 0;
        current.activity = []; current.stepRevision = 0;
      current.name = providerName(current); current.pillBadge = null;
      current.sessionId = undefined; current.turnId = undefined; current.sessionCwd = null;
      current.chatTitle = undefined;
    } else {
      this.tasks = this.tasks.filter((t) => t.id !== id);
      if (this.focusId === id) this.focusId = current.source === "codex" ? "integration_codex" : "integration_claude";
    }
    this.notify();
  }

  setFocus(id: string) {
    const t = this.tasks.find((x) => x.id === id);
    if (!t) return;
    this.focusId = id;
    t.pillBadge = null;
    this.notify();
  }

  updateTask(id: string, state: BotStateName) {
    const t = this.tasks.find((x) => x.id === id);
    if (!t) return;
    t.state = state;
    this.notify();
  }

  appendStep(id: string, step: string) {
    const t = this.tasks.find((x) => x.id === id);
    if (!t) return;
    t.steps.push(step);
    t.stepRevision = (t.stepRevision ?? 0) + 1;
    if (t.steps.length > 20) t.steps.shift();
    t.stepIndex = t.steps.length - 1;
    this.notify();
  }

  setPillBadge(id: string, badge: PillBadge | null) {
    const t = this.tasks.find((x) => x.id === id);
    if (!t) return;
    t.pillBadge = badge;
    this.notify();
  }

  /** loadIntegrationTasks() — VS Code always on, the rest opt-in (max 4). */
  loadIntegrationTasks() {
    for (const proto of INTEGRATION_AGENTS) {
      const shouldLoad =
        isCodingAgent(proto) || this.settings.activeIntegrations.includes(proto.id);
      const idx = this.tasks.findIndex((t) => t.id === proto.id);
      if (shouldLoad && idx < 0) this.tasks.push({ ...proto, steps: [] });
      if (!shouldLoad && idx >= 0) this.tasks.splice(idx, 1);
    }
    // Keep the declared order so pills never shuffle.
    const order = INTEGRATION_AGENTS.map((t) => t.id);
    const rank = (t: AgentTask) => order.indexOf(isCodingAgent(t) ? t.source === "codex" ? "integration_codex" : "integration_claude" : t.id);
    this.tasks.sort((a, b) => rank(a) - rank(b));
    if (!this.focusId) this.focusId = "integration_claude";
    this.notify();
  }

  toggleIntegration(id: string) {
    if (id === "integration_claude" || id === "integration_codex") return;
    const active = this.settings.activeIntegrations;
    if (active.includes(id)) {
      this.settings.activeIntegrations = active.filter((x) => x !== id);
      if (this.focusId === id) this.focusId = "integration_claude";
    } else {
      if (active.length >= 4) return;
      this.settings.activeIntegrations = [...active, id];
    }
    this.loadIntegrationTasks();
  }

  defaultView(): IslandViewName {
    return this.tasks.length === 0 ? "empty" : "overview";
  }
}

export const State = new AppState();
