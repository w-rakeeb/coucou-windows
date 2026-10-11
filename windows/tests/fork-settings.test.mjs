import assert from "node:assert/strict";
import { test } from "node:test";
import { installFakeDom } from "./fakedom.mjs";
import { settingsV2 } from "../src/settings/interface-v2.ts";
import { h } from "../src/views/dom.ts";

test("Modern settings keeps the existing pages and gives Android pairing its own page", () => {
  installFakeDom();
  const appearance = h("section", { text: "Custom sizes and accent" });
  const phone = h("section", { text: "Phone permissions and pairing" });
  const window = h("section", { text: "Custom placement and keep-open" });
  const shell = settingsV2({ appearance: [appearance], window: [window], phone: [phone] }, "0.4.0", h("select"));
  const buttons = shell.find("BUTTON");
  assert.deepEqual(buttons.map(b => b.textContent), ["Appearance", "Window & behavior", "Chat", "Connections", "Coding agents", "Android phone", "Plan usage", "Shortcuts"]);
  assert.equal(phone.parentNode.hidden, true);
  buttons.find(b => b.textContent === "Android phone").fire("click");
  assert.equal(phone.parentNode.hidden, false);
  assert.equal(appearance.parentNode.hidden, true);
  buttons.find(b => b.textContent === "Window & behavior").fire("click");
  assert.equal(window.parentNode.hidden, false);
  assert.equal(phone.parentNode.hidden, true);
});
