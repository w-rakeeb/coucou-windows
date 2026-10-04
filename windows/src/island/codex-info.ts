import { Bridge, IS_TAURI } from "../core/bridge";
import { State, type CodexLimitWindow } from "../core/state";

const attempts = new Map<string, number>();
const pending = new Set<string>();
let readingTelemetry = false;
export async function refreshCodexTelemetry() {
  if (!IS_TAURI || State.paused || readingTelemetry) return;
  readingTelemetry = true;
  try {
    const ids = State.tasks.filter(t => t.source === "codex" && t.sessionId).map(t => t.sessionId!);
    const info = State.codexInfo;
    const telemetry = await Bridge.codexTelemetry(ids, info?.primary ?? null, info?.secondary ?? null);
    if (telemetry) { State.codexTelemetry = telemetry; State.notify(); }
  } finally { readingTelemetry = false; }
}
export async function refreshCodexInfo(threadId: string | null = null, force = false) {
  if (!IS_TAURI || State.paused) return;
  if (threadId && !/^[\da-f]{8}-[\da-f]{4}-[\da-f]{4}-[\da-f]{4}-[\da-f]{12}$/i.test(threadId)) threadId = null;
  const key = threadId ?? "account";
  if (pending.has(key) || (!force && Date.now() - (attempts.get(key) ?? 0) < (threadId ? 300_000 : 30_000))) return;
  attempts.set(key, Date.now()); pending.add(key);
  try {
    const info = await Bridge.codexInfo(threadId, force);
    if (info) {
      if (!State.codexInfo || info.checkedAt >= State.codexInfo.checkedAt) State.codexInfo = info;
      if (info.threadId && info.title) {
        const task = State.tasks.find(task => task.source === "codex" && task.sessionId === info.threadId);
        if (task) { task.chatTitle = info.title; task.name = info.title; }
      }
    } else if (!State.codexInfo) {
      State.codexInfo = {primary:null,secondary:null,checkedAt:0,error:"Codex limits are unavailable.",threadId:null,title:null};
    }
    State.notify();
    void refreshCodexTelemetry();
  } finally { pending.delete(key); }
}
export function startCodexInfo() {
  void refreshCodexInfo();
  window.setInterval(() => {
    if (!State.paused && (State.mode !== "hidden" || State.activeChats.length)) void refreshCodexInfo();
  }, 30_000);
  let reading = false;
  window.setInterval(async () => {
    if (!IS_TAURI || State.paused || reading) return;
    const ids=State.tasks.filter(t=>t.source==="codex"&&t.sessionId).map(t=>t.sessionId!);
    reading=true;
    try {
      const live=await Bridge.codexLiveLimits(ids);
      if(live && live.checkedAt > (State.codexInfo?.checkedAt??0)) {
        const old=State.codexInfo;
        State.codexInfo={...live,primary:live.primary??old?.primary??null,secondary:live.secondary??old?.secondary??null};
        State.notify();
      } else if(State.mode!=="hidden") State.notify();
    } finally {reading=false;}
    void refreshCodexTelemetry();
  }, 2_000);
}
export function windowLabel(window: CodexLimitWindow | null, weekly = false): string {
  if (!window) return weekly ? "Weekly" : "5H";
  if (window.windowDurationMins === 10080) return "Weekly";
  if (window.windowDurationMins % 60 === 0) return `${window.windowDurationMins / 60}H`;
  return `${window.windowDurationMins}m`;
}
export function remaining(window: CodexLimitWindow | null): string {
  return window ? `${Math.round(Math.max(0, Math.min(100, 100 - window.usedPercent)))}%` : "—";
}
export function resetText(window: CodexLimitWindow | null): string {
  if (!window?.resetsAt) return "Reset time unavailable";
  const minutes = Math.ceil(Math.max(0, window.resetsAt * 1000 - Date.now()) / 60000);
  if (!minutes) return "Reset due; refresh to check";
  const days = Math.floor(minutes / 1440), hours = Math.floor(minutes % 1440 / 60), mins = minutes % 60;
  return `Resets in ${days ? `${days}d ` : ""}${hours ? `${hours}h ` : ""}${mins}m`;
}

export function shortReset(window: CodexLimitWindow | null): string {
  if (!window?.resetsAt) return "—";
  const minutes=Math.ceil(Math.max(0,window.resetsAt*1000-Date.now())/60000);
  const days=Math.floor(minutes/1440),hours=Math.floor(minutes%1440/60),mins=minutes%60;
  return days ? `${days}d${hours}h` : hours ? `${hours}h${mins}m` : `${mins}m`;
}
