import { State } from "../core/state";
import { h } from "./dom";
import { remaining, resetText, windowLabel, shortReset } from "../island/codex-info";

export function buildCompact(expand: () => void) {
  const limits = h("button", {id: "compact-limits", onclick: expand});
  const activity = h("span", {id: "compact-activity"});
  const el = h("div", {id: "compact-info"}, limits, activity);
  return {el, sync() {
    const info = State.codexInfo;
    limits.hidden = !State.settings.compactLimits;
    activity.hidden = !State.settings.compactActivity;
    el.classList.toggle("both", State.settings.compactLimits && State.settings.compactActivity);
    el.classList.toggle("resets", State.settings.compactReset);
    limits.textContent = !info ? "Codex limits …" : info.error ? "Limits unavailable" :
      `${windowLabel(info.primary)} ${remaining(info.primary)} · ${windowLabel(info.secondary, true)} ${remaining(info.secondary)}`;
    if(info && !info.error && State.settings.compactReset) limits.textContent=
      `${windowLabel(info.primary)} ${remaining(info.primary)} ↻${shortReset(info.primary)} · W ${remaining(info.secondary)} ↻${shortReset(info.secondary)}`;
    limits.title = info?.error ?? (info ? `${resetText(info.primary)} · ${resetText(info.secondary)}` : "Checking Codex allowance");
    limits.setAttribute("aria-label", `${limits.textContent} remaining. Click to expand.`);
    const tasks = State.activeChats;
    const first = tasks[0];
    const provider = first?.source === "codex" ? "Codex" : "Claude";
    activity.textContent = tasks.length > 1 ? `${tasks.length} tasks running` : !first ? "No task running" :
      first.state === "approval" ? `${provider} needs approval` : first.state === "question" ? `${provider} needs an answer` :
      first.state === "ratelimit" ? `${provider} is waiting` : first.state === "thinking" ? `${provider} is thinking` : `${provider} is working`;
    activity.title = first ? tasks.map(task => task.name).join(" · ") : "No active coding session";
  }};
}
