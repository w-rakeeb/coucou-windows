import { t } from "../i18n/i18n";
import { Bridge } from "../core/bridge";
import type { Settings } from "../core/state";
import { h, clear } from "../views/dom";

export function chatApiV2(settings: Settings, present: Record<string, boolean>, save: () => Promise<void>): HTMLElement {
  const providers = [
    { id: "anthropic", name: "Claude", key: "anthropic-api-key", model: "model", placeholder: "sk-ant-…" },
    { id: "openai", name: "OpenAI", key: "openai-api-key", model: "openaiModel", placeholder: "sk-…" },
    { id: "google", name: "Google AI", key: "google-api-key", model: "googleModel", placeholder: "AIza…" },
    { id: "openrouter", name: "OpenRouter", key: "openrouter-api-key", model: "openrouterModel", placeholder: "sk-or-…" },
  ] as const;
  const tabs = h("div", { class: "provider-tabs", role: "group", "aria-label": "Chat provider" });
  const body = h("div", { class: "provider-body" });
  const section = h("section", { class: "api-v2" }, tabs, body);
  function draw() {
    for (const button of tabs.children) button.setAttribute("aria-pressed", String((button as HTMLElement).dataset.provider === settings.chatProvider));
    const spec = providers.find(p => p.id === settings.chatProvider) ?? providers[0];
    clear(body);
    const ready = !!present[spec.key];
    const model = h("input", { type: "text", class: "model-field", "aria-label": spec.name + " model", value: spec.id === "anthropic" ? settings.model : settings.chatModels[spec.id] ?? (spec.id === "openai" ? settings.openaiModel : spec.id === "openrouter" ? settings.openrouterModel : ""), spellcheck: "false" }) as HTMLInputElement;
    model.addEventListener("change", () => { const value = model.value.trim(); if (value) { if (spec.id === "anthropic") settings.model = value; else { settings.chatModels[spec.id] = value; if(spec.id === "openai") settings.openaiModel=value; if(spec.id === "openrouter") settings.openrouterModel=value; } void save(); } });
    const modelRow = h("div", { class: "field-stack" }, h("label", { text: t("Model ID"), for: "chat-model" }), model);
    model.id = "chat-model";
    const status = h("div", { class: "credential-status" }, h("span", { text: ready ? "Key saved" : "No API key" }));
    const field = h("input", { type: "password", "aria-label": spec.name + " API key", placeholder: spec.placeholder, autocomplete: "off", spellcheck: "false" }) as HTMLInputElement;
    const saveKey = h("button", { class: "primary", text: t("Save key") }) as HTMLButtonElement;
    saveKey.disabled = true;
    field.addEventListener("input", () => { saveKey.disabled = !field.value.trim(); });
    const feedback = h("div", { class: "key-feedback", role: "status", "aria-live": "polite" });
    const entry = h("div", { class: "key-entry", hidden: ready }, h("label", { text: spec.name + " API key" }), h("div", { class: "key-input-row" }, field, saveKey));
    const update = h("button", { text: "Replace key", onclick: () => { entry.hidden = !entry.hidden; if (!entry.hidden) field.focus(); } });
    const remove = h("button", { class: "quiet-danger", text: "Remove key" });
    saveKey.addEventListener("click", async () => {
      const key = field.value.trim(); if (!key) return;
      saveKey.disabled = true; feedback.textContent = t("Saving…");
      try { await Bridge.secretSet(spec.key, key); field.value = ""; present[spec.key] = true; draw(); (body.querySelector('[role="status"]') as HTMLElement).textContent = "Key saved."; }
      catch { field.value = ""; feedback.textContent = "Could not save the key."; }
    });
    remove.addEventListener("click", async () => {
      remove.disabled = true;
      try { await Bridge.secretClear(spec.key); present[spec.key] = false; draw(); }
      catch { feedback.textContent = "Could not remove the key."; remove.disabled = false; }
    });
    body.append(modelRow, h("div", { class: "credential-box" }, status, ready ? h("div", { class: "key-actions" }, update, remove) : null, entry, feedback), h("p", { class: "hint api-footnote", text: "Changing provider or model starts a new chat." }));
  }
  for (const spec of providers) tabs.append(h("button", { "data-provider": spec.id, "aria-label": "Use " + spec.name, text: spec.name, onclick: () => { settings.chatProvider = spec.id; void save(); draw(); } }));
  draw();
  return section;
}
