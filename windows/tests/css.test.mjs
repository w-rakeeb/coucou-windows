import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import postcss from "postcss";

for (const file of ["src/style.css", "src/settings/settings.css"]) {
  test(`${file} keeps view styles at their intended scope`, () => {
    const css = postcss.parse(readFileSync(file, "utf8"));
    css.walkRules(rule => {
      assert.notEqual(rule.parent.type, "rule", `Unintended nesting: ${rule.selector}`);
    });
  });
}
