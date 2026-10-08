import { t } from "../i18n/i18n";
// Claude Code and Codex lifecycle events → isolated session state.
import { Bridge, onEvent } from "../core/bridge";
import { Sound } from "../core/sound";
import { State, isCodingAgent, providerName } from "../core/state";
import type { Island } from "./island";
import { buildFileDiff, fileName, makeDiffStep } from "../core/diff";
import { refreshCodexInfo } from "./codex-info";

const finishTimers = new Map<string, number>();
let pendingTimeout: number | null = null;

export interface HookPayload {
  provider?: string;
  hook_event_name?: string;
  request_id?: string;
  session_id?: string;
  turn_id?: string;
  coucou_diff_truncated?: boolean;
  cwd?: string;
  message?: string;
  prompt?: string;
  tool_name?: string;
  tool_input?: Record<string, unknown>;
}

const TOOL_LABELS: Record<string, string> = {
  Bash: "Run", PowerShell: "Run", Read: "Read", Write: "Write", Edit: "Edit",
  apply_patch: "Edit", Glob: "Find", Grep: "Search", WebSearch: "Web search",
  WebFetch: "Fetch", TodoWrite: t("Tasks"), Task: t("Agent"), Agent: t("Agent"), LS: "List",
  MultiEdit: "Edit", NotebookEdit: t("Notebook"),
};

function lastPathComponent(path: string): string {
  return path.replace(/[\\/]+$/, "").split(/[\\/]/).pop() ?? "";
}

function target(input: Record<string, unknown>): string {
  for (const field of ["command", "file_path", "path", "url", "query", "pattern", "prompt", "code", "patch", "input"]) {
    const value = input[field];
    if (typeof value === "string" && value.trim()) return value.trim();
  }
  // MCP calls may use completely different field names. Display the actual
  // arguments instead of offering blind approval of just the tool's name.
  return Object.keys(input).length ? JSON.stringify(input, null, 2) : "";
}

function activityDetail(input: Record<string, unknown>, fallback: string): string {
  const text = Object.entries(input).map(([key, value]) =>
    `${key}\n${typeof value === "string" ? value : JSON.stringify(value, null, 2)}`,
  ).join("\n\n") || fallback;
  return text.length > 6000 ? `${text.slice(0, 6000)}\n\n… Preview limited to 6000 characters.` : text;
}

function clearFinish(id: string) {
  const timer = finishTimers.get(id);
  if (timer != null) window.clearTimeout(timer);
  finishTimers.delete(id);
}

export function clearApproval(island: Island, requestId: string) {
  const approval = State.pendingApproval;
  if (!approval || approval.requestId !== requestId) return;
  if (pendingTimeout != null) window.clearTimeout(pendingTimeout);
  pendingTimeout = null;
  State.endApproval();
  island.dropPin();
  State.updateTask(approval.taskId ?? approval.pillId, "working");
  State.setPillBadge(approval.taskId ?? approval.pillId, null);
  if (State.view === "approval") island.setView(State.defaultView());
  State.notify();
}

export function registerHookHandlers(island: Island) {
  void onEvent<HookPayload>("hook", (payload) => handleHook(island, payload));
  void onEvent<{ requestId: string }>("approval-ended", ({ requestId }) => clearApproval(island, requestId));
}

export function handleHook(island: Island, payload: HookPayload) {
  if (State.paused) {
    if (payload.request_id) void Bridge.approvalDecline(payload.request_id);
    return;
  }
  const name = payload.hook_event_name ?? "";
  const source = payload.provider === "codex" ? "codex" : "claudeCode";
  const cwd = payload.cwd ?? "";
  const session = State.sessionTask(source, payload.session_id || "legacy", cwd);
  const id = session.id;
  if (name === "Stop" && payload.turn_id && session.turnId && payload.turn_id !== session.turnId) return;
  const pending = State.pendingApproval;
  if (pending?.pillId === id && ["Stop", "StopFailure", "UserPromptSubmit", "SessionEnd", "Interrupt"].includes(name)) {
    void Bridge.approvalDecline(pending.requestId);
    clearApproval(island, pending.requestId);
  }
  const project = lastPathComponent(cwd || session.sessionCwd || "");
  session.name = source === "codex" ? session.chatTitle ?? "Codex chat" : project || providerName(session);
  if (source === "codex") void refreshCodexInfo(session.sessionId ?? null);

  if (["SessionStart", "UserPromptSubmit", "PreToolUse", "PostToolUse", "PermissionRequest"].includes(name)) {
    clearFinish(id);
    session.pillBadge = null;
    if (payload.turn_id) session.turnId = payload.turn_id;
    if (!State.pendingApproval && isCodingAgent(State.focusTask) && State.focusTask?.state === "idle") {
      State.setFocus(id);
    }
  }
  const focused = State.focusId === id;
  const surface = (view: Parameters<Island["alert"]>[0], alert: boolean) => {
    if (State.pendingApproval && State.pendingApproval.taskId !== id) return;
    if (State.mode === "expanded") { if (alert) island.setView(view); }
    else if (alert) island.alert(view);
    else if (State.mode === "hidden") island.reveal();
  };

  switch (name) {
    case "SessionStart":
      surface("overview", false);
      Sound.play("work");
      break;
    case "UserPromptSubmit":
      session.steps = []; session.activity = []; session.stepIndex = 0; session.finalLine = null;
      State.updateTask(id, "thinking");
      if (payload.prompt ?? payload.message) State.appendStep(id, (payload.prompt ?? payload.message)!.slice(0, 120));
      surface("overview", false);
      break;
    case "PreToolUse": {
      State.updateTask(id, "working");
      const tool = payload.tool_name ?? "Tool";
      session.activity ??= [];
      session.activity.push({ label: TOOL_LABELS[tool] ?? tool, detail: activityDetail(payload.tool_input ?? {}, tool), input: payload.tool_input });
      if (session.activity.length > 20) session.activity.shift();
      const detail = target(payload.tool_input ?? {}).replace(/\s+/g, " ").slice(0, 100);
      State.appendStep(id, `${TOOL_LABELS[tool] ?? tool}${detail ? ` · ${detail}` : ""}`);
      surface("overview", false);
      break;
    }
    case "PostToolUse":
      State.updateTask(id, "working");
      if (!payload.coucou_diff_truncated) {
        const diff = buildFileDiff(payload.tool_name ?? "", payload.tool_input ?? {});
        if (diff) { const key = State.appendSessionDiff(id, diff); State.appendStep(id, makeDiffStep(fileName(diff.path), diff.added, diff.removed, key)); }
      }
      break;
    case "PostToolUseFailure":
      State.updateTask(id, "working"); State.appendStep(id, "Tool failed");
      break;
    case "Notification": {
      const message = payload.message ?? "";
      if (message.toLowerCase().includes("rate limit")) {
        State.updateTask(id, "ratelimit"); Sound.play("rate");
      } else if (message.endsWith("?")) {
        State.updateTask(id, "question"); State.appendStep(id, message);
      }
      break;
    }
    case "Stop": {
      // A delayed Stop from an earlier turn cannot finish a newer turn.
      if (payload.turn_id && session.turnId && payload.turn_id !== session.turnId) break;
      clearFinish(id);
      State.updateTask(id, "finished");
      if (payload.message) {
        session.activity ??= [];
        session.activity.push({ label: "Reply", detail: payload.message.slice(0, 6000) });
        if (session.activity.length > 20) session.activity.shift();
      }
      if (payload.message) { session.finalLine = payload.message.replace(/\s+/g, " ").trim().slice(0, 6000); State.appendStep(id, payload.message.slice(0, 160)); }
      Sound.play("finish");
      if (focused && !State.pendingApproval) surface("finished", true);
      else { State.setPillBadge(id, "finished"); island.reveal(); }
      finishTimers.set(id, window.setTimeout(() => {
        finishTimers.delete(id);
        State.updateTask(id, "idle"); State.setPillBadge(id, null);
      }, 5200));
      break;
    }
    case "StopFailure":
      clearFinish(id); State.updateTask(id, "error"); Sound.play("error");
      if (focused) surface("error", true); else State.setPillBadge(id, "error");
      break;
    case "Interrupt":
      clearFinish(id);
      if (State.pendingApproval?.taskId === id) {
        void Bridge.approvalDecline(State.pendingApproval.requestId);
        clearApproval(island, State.pendingApproval.requestId);
      }
      State.updateTask(id, "idle"); State.setPillBadge(id, null); State.appendStep(id, "Interrupted");
      break;
    case "SessionEnd":
      clearFinish(id);
      if (State.pendingApproval?.taskId === id) {
        void Bridge.approvalDecline(State.pendingApproval.requestId);
        clearApproval(island, State.pendingApproval.requestId);
      }
      State.clearSessionDiffs(id); State.endSession(id);
      break;
    case "SubagentStart": State.appendStep(id, t("+ subagent")); break;
    case "SubagentStop": State.appendStep(id, "Subagent done"); break;
    case "PermissionRequest": {
      const requestId = payload.request_id ?? "";
      if (!requestId) break;
      if (State.pendingApproval && State.pendingApproval.requestId !== requestId) {
        void Bridge.approvalDecline(requestId); break;
      }
      if (pendingTimeout != null) window.clearTimeout(pendingTimeout);
      const tool = payload.tool_name ?? "Tool";
      const input = target(payload.tool_input ?? {});
      State.beginApproval({ requestId, sessionId: session.sessionId ?? "", taskId: id, pillId: id, tool, command: `${tool}${input ? ` · ${input}` : ""}` });
      State.updateTask(id, "approval");
      State.setFocus(id);
      State.isPinned = true;
      Sound.play("approval");
      island.alert("approval");
      // Acknowledge only after the correct session's card has been shown.
      void Bridge.approvalAck(requestId);
      pendingTimeout = window.setTimeout(() => clearApproval(island, requestId), 110_000);
      break;
    }
  }
  State.notify();
}
