import { t } from "../i18n/i18n";
import { h } from "../views/dom";

export function settingsV2(groups: Record<string, HTMLElement[]>, version: string, switcher: HTMLElement) {
  const pages = [
    ["appearance", "Appearance"],
    ["window", "Window & behavior"],
    ["chat", t("Chat")],
    ["connections", "Connections"],
    ["agents", "Coding agents"],
    ["usage", t("Plan usage")],
    ["shortcuts", t("Shortcuts")],
  ];
  const title = h("h1");
  const content = h("div", { class: "settings-content" });
  const heading = h("header", { class: "page-heading" }, title);
  const main = h("main", { class: "settings-main" }, heading, content);
  const nav = h("nav", { class: "settings-nav", "aria-label": "Settings pages" });
  const sections = new Map<string, HTMLElement>();
  for (const [id, rawLabel] of pages) {
    const label = t(rawLabel);
    const panel = h("div", { class: "settings-page", "data-page": id }, ...(groups[id] ?? []));
    sections.set(id, panel); content.append(panel);
    nav.append(h("button", { "data-page-link": id, text: label, onclick: () => show(id, label) }));
  }
  function show(id: string, label: string) {
    title.textContent = label;
    for (const [key, panel] of sections) panel.hidden = key !== id;
    for (const button of nav.children) button.setAttribute("aria-current", (button as HTMLElement).dataset.pageLink === id ? "page" : "false");
    main.scrollTop = 0;
  }
  const sidebar = h("aside", { class: "settings-sidebar" }, h("div", { class: "settings-brand" }, h("strong", { text: "Coucou" }), h("span", { text: t("Settings") })), nav, h("div", { class: "sidebar-footer" }, switcher, h("span", { class: "hint", text: "Version " + version })));
  show(pages[0][0], t(pages[0][1]));
  return h("div", { class: "settings-shell" }, sidebar, main);
}
