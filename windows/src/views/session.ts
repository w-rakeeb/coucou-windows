import { h, clear } from "./dom";
import type { AgentTask, CodexSession } from "../core/state";

const exact = new Intl.NumberFormat(undefined, { maximumFractionDigits: 0 });
export function tokenNumber(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) return "—";
  for (const [size, unit] of [[1e9, "B"], [1e6, "M"], [1e3, "K"]] as const) {
    if (value >= size) return `${(value / size).toLocaleString(undefined, { maximumFractionDigits: 1 })}${unit}`;
  }
  return exact.format(value);
}
export function exactTokens(value: number): string { return `${exact.format(value)} tokens`; }
function metric(label: string, value: number | null | undefined): HTMLElement {
  return h("div", { class: "session-metric" }, h("span", { text: label }),
    h("strong", { text: tokenNumber(value), title: value == null ? "Not reported yet" : exactTokens(value) }));
}
function activity(task: AgentTask | null): string {
  return ({ working: "Running a task", thinking: "Thinking", searching: "Searching",
    approval: "Needs approval", question: "Waiting for an answer", error: "Stopped on an error",
    ratelimit: "Waiting for allowance", sleeping: "Paused" } as Record<string, string>)[task?.state ?? ""] ?? "Waiting for input";
}
export function renderSession(container: HTMLElement, task: AgentTask | null, session: CodexSession | null) {
  const scroll = container.scrollTop;
  const breakdownOpen = container.querySelector<HTMLDetailsElement>(".session-breakdown")?.open ?? false;
  clear(container);
  if (!session) {
    container.append(h("div", { class: "session-empty", text: "Waiting for this session’s Codex token report…" }),
      h("div", { class: "session-note", text: "Usage appears after a model response. Other sessions keep their own totals." }));
    return;
  }
  const model = session.model?.replace(/^gpt-/i, "GPT-") ?? "Model unavailable";
  const effort = session.reasoningEffort ? session.reasoningEffort[0].toUpperCase() + session.reasoningEffort.slice(1) : null;
  const head = h("div", { class: "session-head" },
    h("div", { class: "session-model", text: [model, effort].filter(Boolean).join(" · ") }),
    h("div", { class: "session-activity", text: activity(task) }));
  if (session.branch) head.append(h("div", { class: "session-branch", text: session.branch }));
  const used = session.contextUsed, capacity = session.modelContextWindow;
  const percent = used != null && capacity ? Math.min(100, Math.max(0, used / capacity * 100)) : null;
  const fill = h("span", { class: "session-context-fill", style: `width:${percent ?? 0}%` });
  const context = h("div", { class: "session-context" },
    h("div", { class: "session-context-heading" },
      h("span", { text: "Context" }), h("span", { text: percent == null ? "Capacity unavailable" : `${Math.round(100 - percent)}% free` })),
    h("div", { class: "session-context-value", text: `${tokenNumber(used)} / ${tokenNumber(capacity)}`,
      title: used != null && capacity ? `${exactTokens(used)} of ${exactTokens(capacity)}` : "Codex has not reported a context capacity." }),
    h("div", { class: "session-context-track", role: "progressbar", "aria-label": "Session context used",
      "aria-valuemin": 0, "aria-valuemax": 100, "aria-valuenow": percent == null ? undefined : Math.round(percent) }, fill));
  const details = h("details", { class: "session-breakdown" }, h("summary", { text: "Token breakdown" }),
    h("div", { class: "session-breakdown-grid" }, metric("Input", session.total?.inputTokens),
      metric("Cached input", session.total?.cachedInputTokens), metric("Output", session.total?.outputTokens),
      metric("Reasoning", session.total?.reasoningOutputTokens)),
    h("div", { class: "session-note", text: "Cached input is included in input. Reasoning is included in output." }));
  details.open = breakdownOpen;
  container.append(h("div", { class: "session-summary" }, head,
    h("div", { class: "session-metrics" }, metric("This update", session.lastUpdateTokens),
      metric("Last response", session.last?.totalTokens), metric("Session total", session.total?.totalTokens)),
    context, details,
    h("div", { class: "session-note", text: "Live local session data · Updated after each model response", title: session.threadId })));
  container.scrollTop = scroll;
}
