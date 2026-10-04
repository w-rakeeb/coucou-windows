// Settings window — the place where anything that writes to disk is confirmed.
// Stage 2 covers the Claude Code hooks and the general preferences; API keys and
// integrations land here too in a later stage.

import "./settings.css";
import { chatApiV2 } from "./chat-api";
import { settingsV2 } from "./interface-v2";
import { Bridge, onEvent, type HookStatus } from "../core/bridge";
import { DEFAULT_SETTINGS, type Settings } from "../core/state";
import { h, clear } from "../views/dom";

let settings: Settings = { ...DEFAULT_SETTINGS };
let version = "";

const root = document.getElementById("settings-root")!;

async function save() {
  await Bridge.saveSettings(settings);
}

// ── Reusable bits ─────────────────────────────────────────────────────────────

function toggle(on: boolean, onChange: (v: boolean) => void): HTMLElement {
  const el = h("button", { class: on ? "switch on" : "switch", "aria-pressed": String(on) });
  el.addEventListener("click", () => {
    const next = !el.classList.contains("on");
    el.classList.toggle("on", next);
    el.setAttribute("aria-pressed", String(next));
    onChange(next);
  });
  return el;
}

function statusDot(ok: boolean): HTMLElement {
  return h("i", { class: "dot", style: `background:${ok ? "#22c55e" : "#f4505e"}` });
}

function renderDiff(text: string): HTMLElement {
  const box = h("div", { class: "diff" });
  for (const line of text.split("\n")) {
    const cls = line.startsWith("+") ? "add" : line.startsWith("-") ? "del" : "ctx";
    box.append(h("div", { class: cls, text: line }));
  }
  return box;
}

// ── Claude Code section ───────────────────────────────────────────────────────

function hooksSection(status: HookStatus, codex = false): HTMLElement {
  const name = codex ? "Codex" : "Claude Code";
  const getStatus = codex ? Bridge.codexHooksStatus : Bridge.hooksStatus;
  const getPreview = codex ? Bridge.codexHooksPreview : Bridge.hooksPreview;
  const apply = codex ? Bridge.codexHooksApply : Bridge.hooksApply;
  const body = h("div", { style: "display:flex;flex-direction:column;gap:12px" });
  const section = h(
    "section",
    {},
    h("h2", {}, statusDot(status.installed), h("span", { text: name })),
    body,
  );

  const rebuild = async () => {
    const fresh = await getStatus();
    if (fresh) Object.assign(status, fresh);
    clear(body);
    draw();
    const head = section.querySelector("h2")!;
    clear(head);
    head.append(statusDot(status.installed), h("span", { text: name }));
  };

  function draw() {
    body.append(
      h("div", {
        class: "hint",
        text: status.installed
          ? "Hooks installed."
          : "Install hooks to show sessions and approvals.",
      }),
      h("div", { class: "row" },
        h("label", { text: codex ? "hooks.json" : "settings.json" }),
        h("span", { class: "path", text: status.settingsPath }),
      ),
      h("div", { class: "row" },
        h("label", { text: "Relay" }),
        h("span", { class: "path", text: status.hookPath }),
        statusDot(status.hookReady),
      ),
    );

    if (codex) body.append(h("div", {
      class: "notice warn",
      text: "Review new hooks with /hooks in Codex CLI, then start a new session.",
    }));

    if (!status.hookReady) {
      body.append(h("div", {
        class: "notice warn",
        text: "Hook relay missing. Restart Coucou.",
      }));
    }

    const actions = h("div", { class: "row" });
    const install = h("button", {
      class: "primary",
      text: status.installed ? "Reinstall hooks…" : "Install hooks…",
      onclick: () => showPreview(true),
    });
    // Writing hook commands that point at a relay which isn't there would give
    // every Claude Code session a broken hook and nothing to show for it.
    if (!status.hookReady) {
      install.disabled = true;
      install.title = "Hook relay missing";
    }
    actions.append(install);
    if (status.installed) {
      actions.append(h("button", {
        class: "danger",
        text: "Remove hooks…",
        onclick: () => showPreview(false),
      }));
    }
    body.append(actions);
  }

  async function showPreview(install: boolean) {
    let preview;
    try {
      preview = await getPreview(install);
    } catch (err) {
      // An unreadable or invalid settings.json stops here rather than being
      // treated as empty and written over.
      clear(body);
      body.append(
        h("div", { class: "notice err", text: String(err).replace(/^Error:\s*/, "") }),
        h("div", { class: "row" }, h("button", {
          text: "Back",
          onclick: () => { clear(body); draw(); },
        })),
      );
      return;
    }
    if (!preview) return;
    clear(body);
    body.append(
      h("div", {
        class: "hint",
        text: install
          ? "Review changes. Existing hooks are preserved."
          : "Only Coucou's hooks will be removed.",
      }),
      renderDiff(preview.diff),
      h("div", { class: "row" },
        h("span", { class: "path", text: `Backup → ${preview.backup}` }),
      ),
    );
    const confirm = h("button", {
      class: install ? "primary" : "danger",
      text: install ? "Back up and write" : "Back up and remove",
    });
    confirm.addEventListener("click", async () => {
      confirm.disabled = true;
      try {
        const backup = await apply(install, preview.fingerprint);
        clear(body);
        body.append(h("div", {
          class: "notice ok",
          text: `${install ? "Hooks installed." : "Hooks removed."} ${codex && install ? "Review them with /hooks, then start a new session." : "Start a new session."} Backup: ${backup}`,
        }));
        window.setTimeout(() => void rebuild(), 2600);
      } catch (err) {
        confirm.disabled = false;
        body.append(h("div", { class: "notice err", text: `Could not write: ${String(err)}` }));
      }
    });
    body.append(h("div", { class: "row" }, confirm, h("button", {
      text: "Cancel",
      onclick: () => { clear(body); draw(); },
    })));
  }

  draw();
  return section;
}

// ── Claude API section ────────────────────────────────────────────────────────

const MODELS: [string, string][] = [
  ["claude-opus-5", "Claude Opus 5"],
  ["claude-sonnet-5", "Claude Sonnet 5"],
  ["claude-haiku-4-5", "Claude Haiku 4.5"],
];

function apiSection(present: Record<string,boolean>): HTMLElement {
  const provider=h("select",{"aria-label":"Chat provider"}) as HTMLSelectElement;
  for(const [id,label] of [["anthropic","Claude"],["openai","OpenAI"],["openrouter","OpenRouter"]])provider.append(h("option",{value:id,text:label}));
  provider.value=settings.chatProvider;
  const panels=h("div",{});
  const specs=[
    {id:"anthropic",name:"Claude",key:"anthropic-api-key",placeholder:"sk-ant-...",model:"model"},
    {id:"openai",name:"OpenAI",key:"openai-api-key",placeholder:"sk-...",model:"openaiModel"},
    {id:"openrouter",name:"OpenRouter",key:"openrouter-api-key",placeholder:"sk-or-...",model:"openrouterModel"},
  ] as const;
  for(const spec of specs){
    const saved=statusDot(!!present[spec.key]);
    const state=h("span",{class:"hint",text:present[spec.key]?"Key saved.":"No API key."});
    const field=h("input",{type:"password","aria-label":spec.name+" API key",placeholder:present[spec.key]?"Stored securely":spec.placeholder,autocomplete:"off",spellcheck:"false",style:"flex:1;min-width:0"}) as HTMLInputElement;
    const saveKey=h("button",{class:"primary",text:"Save key"}),remove=h("button",{class:"danger",text:"Remove"}),feedback=h("div",{});
    const model=spec.id==="anthropic"?h("select",{"aria-label":spec.name+" model"}):h("input",{type:"text","aria-label":spec.name+" model",style:"flex:1;min-width:0",spellcheck:"false"});
    if(model instanceof HTMLSelectElement){for(const [id,label] of MODELS)model.append(h("option",{value:id,text:label}));if(!MODELS.some(([id])=>id===settings.model))model.append(h("option",{value:settings.model,text:settings.model}));}
    model.value=settings[spec.model];
    model.addEventListener("change",()=>{const value=model.value.trim();if(value){settings[spec.model]=value;void save();}else model.value=settings[spec.model];});
    async function refresh(){const ready=await Bridge.secretPresent(spec.key)??false;present[spec.key]=ready;saved.style.background=ready?"#22c55e":"#f4505e";state.textContent=ready?"Key saved.":"No API key.";field.placeholder=ready?"Stored securely":spec.placeholder;remove.style.display=ready?"":"none";}
    saveKey.addEventListener("click",async()=>{const value=field.value.trim();if(!value)return;clear(feedback);try{await Bridge.secretSet(spec.key,value);field.value="";feedback.append(h("div",{class:"notice ok",text:"Key saved."}));await refresh();}catch{feedback.append(h("div",{class:"notice err",text:"Could not save the key."}));}});
    remove.addEventListener("click",async()=>{clear(feedback);try{await Bridge.secretClear(spec.key);field.value="";await refresh();}catch{feedback.append(h("div",{class:"notice err",text:"Could not remove the key."}));}});
    remove.style.display=present[spec.key]?"":"none";
    const panel=h("div",{"data-chat-provider":spec.id},h("div",{class:"row"},saved,state),h("div",{class:"row"},h("label",{text:"API key"}),field,saveKey,remove),h("div",{class:"row"},h("label",{text:spec.id==="anthropic"?"Model":"Model ID"}),model),feedback);
    panel.hidden=spec.id!==settings.chatProvider;panels.append(panel);
  }
  provider.addEventListener("change",()=>{settings.chatProvider=provider.value as Settings["chatProvider"];for(const panel of panels.children)(panel as HTMLElement).hidden=(panel as HTMLElement).dataset.chatProvider!==provider.value;void save();});
  return h("section",{},h("h2",{text:"Chat API"}),h("div",{class:"hint",text:"Changing provider or model starts a new chat."}),h("div",{class:"row"},h("label",{text:"Provider"}),provider),panels);
}

// ── Integrations section ──────────────────────────────────────────────────────

interface IntegrationDef {
  id: string;
  name: string;
  color: string;
  /** Credential Manager keys, in the order they are shown. */
  fields: { key: string; label: string; placeholder: string; secret: boolean }[];
}

const INTEGRATIONS: IntegrationDef[] = [
  { id: "integration_stripe", name: "Stripe", color: "#0570DE",
    fields: [{ key: "stripe-api-key", label: "Secret key", placeholder: "sk_live_…", secret: true }] },
  { id: "integration_github", name: "GitHub", color: "#F4505E",
    fields: [{ key: "github-token", label: "Token", placeholder: "ghp_…", secret: true }] },
  { id: "integration_vercel", name: "Vercel", color: "#7C5CFF",
    fields: [{ key: "vercel-token", label: "Token", placeholder: "…", secret: true }] },
  { id: "integration_n8n", name: "n8n", color: "#F29B38",
    fields: [
      { key: "n8n-url", label: "Instance URL", placeholder: "https://n8n.example.com", secret: false },
      { key: "n8n-api-key", label: "API key", placeholder: "…", secret: true },
    ] },
  { id: "integration_resend", name: "Resend", color: "#22C55E",
    fields: [{ key: "resend-api-key", label: "API key", placeholder: "re_…", secret: true }] },
  { id: "integration_notion", name: "Notion", color: "#8C8C8C",
    fields: [{ key: "notion-api-key", label: "Integration token", placeholder: "ntn_…", secret: true }] },
  { id: "integration_calcom", name: "Cal.com", color: "#C9956A",
    fields: [{ key: "calcom-api-key", label: "API key", placeholder: "cal_…", secret: true }] },
];

const MAX_ACTIVE = 4;

function integrationsSection(present: Record<string, boolean>): HTMLElement {
  const note = h("div", { class: "hint" });
  const list = h("div", { style: "display:flex;flex-direction:column;gap:14px" });

  function updateNote() {
    const used = settings.activeIntegrations.length;
    note.textContent = `Services: ${used}/${MAX_ACTIVE} selected.`;
  }

  for (const def of INTEGRATIONS) {
    const active = settings.activeIntegrations.includes(def.id);
    const sw = h("button", { class: active ? "switch on" : "switch" });
    sw.addEventListener("click", () => {
      const on = settings.activeIntegrations.includes(def.id);
      if (on) {
        settings.activeIntegrations = settings.activeIntegrations.filter((x) => x !== def.id);
      } else {
        if (settings.activeIntegrations.length >= MAX_ACTIVE) return;
        settings.activeIntegrations = [...settings.activeIntegrations, def.id];
      }
      sw.classList.toggle("on", !on);
      updateNote();
      void save();
    });

    const rows = h("div", { style: "display:flex;flex-direction:column;gap:6px;flex:1 1 auto;min-width:0" });
    for (const field of def.fields) {
      const input = h("input", {
        type: field.secret ? "password" : "text",
        placeholder: present[field.key] ? "Saved" : field.placeholder,
        autocomplete: "off",
        spellcheck: "false",
        style: "flex:1 1 auto;min-width:0",
      }) as HTMLInputElement;
      const saveBtn = h("button", { text: "Save" });
      const dotEl = statusDot(present[field.key] ?? false);
      saveBtn.addEventListener("click", async () => {
        const value = input.value.trim();
        try {
          await Bridge.secretSet(field.key, value);
          present[field.key] = value.length > 0;
          input.value = "";
          input.placeholder = value ? "Saved" : field.placeholder;
          dotEl.style.background = value ? "#22c55e" : "#f4505e";
        } catch {
          dotEl.style.background = "#f5a524";
        }
      });
      rows.append(
        h("div", { class: "row" },
          h("label", { style: "min-width:104px", text: field.label }),
          input, saveBtn, dotEl,
        ),
      );
    }

    list.append(
      h("div", { class: "integration-item" },
        h("div", { class: "integration-heading" },
          sw,
          h("i", { class: "dot", style: `background:${def.color}` }),
          h("span", { style: "font-size:12.5px", text: def.name }),
        ),
        rows,
      ),
    );
  }

  updateNote();
  return h("section", {}, h("h2", {}, h("span", { text: "Integrations" })), note, list);
}

// ── General section ───────────────────────────────────────────────────────────

function generalSection(monitors: {id:string;label:string;width:number;height:number}[]): HTMLElement {
  const petColor = h("input", { type: "color", "aria-label": "Pet color", value: settings.petColor || "#a8d8cc" }) as HTMLInputElement;
  petColor.addEventListener("input", () => { settings.petColor = petColor.value; void save(); });
  const resetColor = h("button", { text: "Default", title: "Restore the original agent colors", onclick: () => { settings.petColor = ""; petColor.value = "#a8d8cc"; void save(); } });
  const volume = h("input", {
    type: "range", min: "0", max: "0.2", step: "0.005",
    value: String(settings.soundVolume),
  }) as HTMLInputElement;
  volume.addEventListener("input", () => {
    settings.soundVolume = Number(volume.value);
    void save();
  });

  const autoClose = h("input", {
    type: "number", min: "5", max: "120", step: "1",
    title: "Delay after leaving the island",
    value: String(Math.round(settings.autoCloseInterval)),
    style: "width:72px",
  }) as HTMLInputElement;
  autoClose.addEventListener("change", () => {
    settings.autoCloseInterval = Math.max(5, Math.min(120, Number(autoClose.value) || 15));
    autoClose.value = String(settings.autoCloseInterval);
    void save();
  });

  const screen = h("select", { "aria-label": "Monitor" }) as HTMLSelectElement;
  screen.append(
    h("option", { value: "primary", text: "Primary monitor" }),
    h("option", { value: "cursor", text: "Follow cursor" }),
  );
  for(const monitor of monitors)screen.append(h("option",{value:monitor.id,text:monitor.label}));
  screen.value = settings.screen;
  screen.addEventListener("change", () => {
    settings.screen = screen.value as Settings["screen"];
    void save();
  });

  const hideDelay=h("input",{type:"number",min:"5",max:"86400",value:String(settings.minimizeHideInterval),title:"Delay after leaving the minimized view",style:"width:80px"}) as HTMLInputElement;
  hideDelay.addEventListener("change",()=>{settings.minimizeHideInterval=Math.max(5,Math.min(86400,Number(hideDelay.value)||60));hideDelay.value=String(settings.minimizeHideInterval);void save();});
  const fullMode=h("select",{"aria-label":"Expanded view behavior"}) as HTMLSelectElement;
  fullMode.append(h("option",{value:"timed",text:"Minimize automatically"}),h("option",{value:"keep",text:"Keep open"}));
  fullMode.value=settings.keepExpanded?"keep":"timed";
  fullMode.addEventListener("change",()=>{settings.keepExpanded=fullMode.value==="keep";autoClose.disabled=settings.keepExpanded;void save();});
  autoClose.disabled=settings.keepExpanded;
  const smallMode=h("select",{"aria-label":"Minimized view behavior"}) as HTMLSelectElement;
  smallMode.append(h("option",{value:"timed",text:"Hide automatically"}),h("option",{value:"keep",text:"Keep visible"}));
  smallMode.value=settings.keepMinimized?"keep":"timed";
  smallMode.addEventListener("change",()=>{settings.keepMinimized=smallMode.value==="keep";hideDelay.disabled=settings.keepMinimized;void save();});
  hideDelay.disabled=settings.keepMinimized;
  function sizeSlider(key: "compactScale" | "expandedScale", label: string): HTMLElement {
    const slider = h("input", {type:"range",min:"75",max:"150",step:"5",value:String(Math.round(settings[key]*100)),"aria-label":label}) as HTMLInputElement;
    const value = h("span", {class:"hint",text:slider.value+"%"});
    slider.addEventListener("input",()=>{settings[key]=Number(slider.value)/100;value.textContent=slider.value+"%";});
    slider.addEventListener("change",()=>{void save();});
    return h("div",{class:"row"},h("label",{text:label}),slider,value);
  }
  const center = h("button",{"data-placement-control":"true",text:"Center on monitor",onclick:()=>{if(settings.positionLocked)return;settings.freePlacement=false;settings.positionX=.5;settings.positionY=0;void save();}}) as HTMLButtonElement;
  center.disabled = screen.disabled = settings.positionLocked;
  function windowToggle(key: "alwaysOnTop" | "positionLocked" | "edgeSnap", label: string): HTMLElement {
    const control=toggle(settings[key],v=>{settings[key]=v;center.disabled=screen.disabled=settings.positionLocked;void save();});
    control.setAttribute("data-window-setting",key);control.setAttribute("aria-label",label);
    return h("div",{class:"row"},h("label",{text:label}),control);
  }


  return h(
    "section",
    {},
    h("h2", {}, h("span", { text: "General" })),
    h("div", { class: "row" }, h("label", { text: "Pet & accent" }), petColor, resetColor),
    sizeSlider("compactScale", "Minimized size"),
    sizeSlider("expandedScale", "Expanded size"),
    h("div",{class:"row"},h("label",{text:"Expanded reset timer"}),toggle(settings.expandedReset,v=>{settings.expandedReset=v;void save();})),
    h("div", { class: "row" },
      h("label", { text: "Minimized limits" }),
      toggle(settings.compactLimits, (v) => { settings.compactLimits = v; void save(); }),
    ),
    h("div", { class: "row" },
      h("label", { text: "Minimized activity" }),
      toggle(settings.compactActivity, (v) => { settings.compactActivity = v; void save(); }),
    ),
    h("div", { class: "row" },
      h("label", { text: "Minimized reset timer" }),
      toggle(settings.compactReset, (v) => { settings.compactReset=v;void save(); }),
    ),
    h("div", { class: "row" },
      h("label", { text: "Sound" }),
      toggle(settings.soundEnabled, (v) => { settings.soundEnabled = v; void save(); }),
      volume,
    ),
    h("div", { class: "row" },
      h("label", { text: "Expanded view" }),
      fullMode,
    ),
    h("div", { class: "row" },
      h("label", { text: "Minimize delay" }),
      autoClose,
      h("span", { class: "hint", text: "seconds" }),
    ),
    h("div",{class:"row"},h("label",{text:"Minimized view"}),smallMode),
    h("div",{class:"row"},h("label",{text:"Hide delay"}),hideDelay,h("span",{class:"hint",text:"seconds"})),
    h("div", { class: "row" },
      h("label", { text: "Monitor" }),
      screen,
    ),
    windowToggle("alwaysOnTop", "Always on top"),
    windowToggle("positionLocked", "Lock position"),
    windowToggle("edgeSnap", "Corner assist"),
    h("div",{class:"row"},h("label",{text:"Placement"}),center),
    h("div",{class:"hint",text:"Drag the top bar or minimized edge to move."}),
    h("div",{class:"row"},h("label",{text:"Remember position"}),toggle(settings.rememberPlacement,v=>{settings.rememberPlacement=v;void save();})),
    h("div",{class:"hint",text:"Off: start centered on the selected monitor."}),
    h("div", { class: "row" },
      h("label", { text: "Start with Windows" }),
      toggle(settings.autostart, (v) => { settings.autostart = v; void save(); }),
    ),
    h("div", { class: "row" },
      h("label", { text: "Hide tray icon" }),
      toggle(settings.hideTrayIcon, (v) => { settings.hideTrayIcon = v; void save(); }),
    ),
  );
}

// ── Boot ──────────────────────────────────────────────────────────────────────

async function main() {
  const boot = await Bridge.boot();
  if (boot) {
    settings = { ...settings, ...boot.settings };
    version = boot.version;
  }
  const monitors=await Bridge.monitorChoices()??[];
  const status = (await Bridge.hooksStatus()) ?? {
    installed: false, settingsPath: "", hookPath: "", hookReady: false,
  };
  const codexStatus = (await Bridge.codexHooksStatus()) ?? {
    installed: false, settingsPath: "", hookPath: "", hookReady: false,
  };

  const apiKeys:Record<string,boolean>={};
  for(const key of ["anthropic-api-key","openai-api-key","openrouter-api-key"])apiKeys[key]=await Bridge.secretPresent(key)??false;

  const keys = [
    "stripe-api-key", "github-token", "vercel-token",
    "n8n-url", "n8n-api-key", "resend-api-key", "notion-api-key", "calcom-api-key",
  ];
  const present: Record<string, boolean> = {};
  for (const k of keys) present[k] = (await Bridge.secretPresent(k)) ?? false;

  function render() {
    clear(root);
    document.body.dataset.interface = settings.settingsInterface;
    const select = h("select", { "aria-label": "Settings interface" }) as HTMLSelectElement;
    select.append(h("option", {value:"v2",text:"v2 · Modern"}),h("option", {value:"v1",text:"v1 · Classic"}));
    select.value = settings.settingsInterface;
    select.addEventListener("change", () => { settings.settingsInterface=select.value as Settings["settingsInterface"];render();void save(); });
    const switcher=h("div",{class:"interface-switcher"},h("label",{text:"Settings design"}),select);
    const general = generalSection(monitors);
    if(settings.settingsInterface === "v1") {
      root.append(h("div",{class:"classic-heading"},h("h1",{},h("span",{text:"Coucou"}),h("span",{class:"version",text:version})),switcher),hooksSection(status),hooksSection(codexStatus,true),apiSection(apiKeys),integrationsSection(present),general);
    } else {
      const appearance=h("section",{}), behavior=h("section",{});
      const appearanceLabels=new Set(["Pet & accent","Minimized size","Expanded size","Expanded reset timer","Minimized limits","Minimized activity","Minimized reset timer","Sound"]);
      for(const child of Array.from(general.children).slice(1)) {
        const label=child.querySelector("label")?.textContent;
        (label && appearanceLabels.has(label) ? appearance : behavior).append(child);
      }
      root.append(settingsV2({appearance:[appearance],window:[behavior],chat:[chatApiV2(settings,apiKeys,save)],connections:[integrationsSection(present)],agents:[hooksSection(codexStatus,true),hooksSection(status)]},version,switcher));
    }
    const fields:Record<string,string>={"Expanded view":"keepExpanded","Minimized view":"keepMinimized","Minimize delay":"autoCloseInterval","Hide delay":"minimizeHideInterval","Pet & accent":"petColor","Minimized size":"compactScale","Expanded size":"expandedScale","Sound":"soundEnabled","Expanded reset timer":"expandedReset","Minimized limits":"compactLimits","Minimized activity":"compactActivity","Minimized reset timer":"compactReset","Remember position":"rememberPlacement","Start with Windows":"autostart","Hide tray icon":"hideTrayIcon"};
    for(const row of root.querySelectorAll<HTMLElement>(".row")) {
      const key=fields[row.querySelector("label")?.textContent??""];
      if(!key)continue;
      const control=row.querySelector<HTMLElement>(".switch,select,input");if(control){control.dataset.setting=key;control.setAttribute("aria-label",row.querySelector("label")!.textContent!);}
    }
  }
  render();

  void onEvent<Settings>("settings-changed", (s) => {
    const previousInterface=settings.settingsInterface;
    Object.assign(settings,s);
    if(previousInterface!==settings.settingsInterface){render();return;}
    for(const control of root.querySelectorAll<HTMLElement>("[data-setting]")){
      const key=control.dataset.setting as keyof Settings,value=settings[key];
      if(control.classList.contains("switch")){control.classList.toggle("on",!!value);control.setAttribute("aria-pressed",String(value));}
      else if(control instanceof HTMLInputElement || control instanceof HTMLSelectElement){
        if(document.activeElement!==control)control.value=key==="keepExpanded"||key==="keepMinimized"?(value?"keep":"timed"):key==="compactScale"||key==="expandedScale"?String(Number(value)*100):String(value);
        if(key==="autoCloseInterval")control.disabled=settings.keepExpanded;
        if(key==="minimizeHideInterval")control.disabled=settings.keepMinimized;
      }
    }
    const monitor = root.querySelector<HTMLSelectElement>('select[aria-label="Monitor"]');
    if (monitor) {monitor.value = settings.screen;monitor.disabled=settings.positionLocked;}
    const center=root.querySelector<HTMLButtonElement>('[data-placement-control]');
    if(center)center.disabled=settings.positionLocked;
    for(const key of ["alwaysOnTop","positionLocked","edgeSnap"] as const){
      const control=root.querySelector<HTMLElement>(`[data-window-setting="${key}"]`);
      control?.classList.toggle("on",settings[key]);control?.setAttribute("aria-pressed",String(settings[key]));
    }
  });
}

void main();
