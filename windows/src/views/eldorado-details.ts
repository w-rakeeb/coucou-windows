import { h } from "./dom";
type Row = Record<string, unknown>;

export { salesStats } from "../core/eldorado-sales";
import { salesStats } from "../core/eldorado-sales";

export function salesDetails(orders: Row[], money: (row: Row) => string, days: number, complete: boolean, loading: boolean, warning: string, select: (days: number) => void): HTMLElement {
  const stats = salesStats(orders, days);
  const ranges = h("div", { class: "eldorado-ranges", role: "group", "aria-label": "Sales period" });
  for (const [label, value] of [["All", 0], ["30 Days", 30], ["7 Days", 7]] as const) ranges.append(h("button", { text: label, "aria-pressed": days === value, onclick: () => select(value) }));
  const coverage = loading ? "Loading history…" : `${complete ? "All available history" : "Partial history"} · ${orders.length} ${orders.length === 1 ? "order" : "orders"}`;
  const root = h("div", { class: "eldorado-details" },
    h("div", { class: "eldorado-details-top" }, ranges, h("span", { text: coverage })),
    h("div", { class: "eldorado-metrics" },
      h("div", {}, h("span", { text: "Sales" }), h("strong", { text: String(stats.sales) })),
      h("div", {}, h("span", { text: "Gross Sales" }), ...(!stats.totals.length ? [h("strong", { text: "—" })] : stats.totals.map(v => h("strong", { text: money(v) })))),
      h("div", {}, h("span", { text: "Average Order" }), ...(!stats.totals.length ? [h("strong", { text: "—" })] : stats.totals.map(v => h("strong", { text: money({ amount: v.average, currency: v.currency }) }))))),
    h("div", { class: "eldorado-counts", text: `${stats.orders} Orders · ${stats.active} Active · ${stats.cancelled} Cancelled · ${stats.refunded} Refunded` }),
    h("div", { class: "eldorado-category-heading", text: "Top Categories" }));
  if (!stats.categories.length) root.append(h("span", { class: "eldorado-status", text: "No sales in this period" }));
  for (const [name, count] of stats.categories) root.append(h("div", { class: "eldorado-category" },
    h("span", { text: name }), h("progress", { max: Math.max(1, stats.sales), value: count, "aria-label": `${name}: ${count} sales` }), h("strong", { text: String(count) })));
  root.append(h("div", { class: "eldorado-details-note", text: "Received and completed orders · Before fees · Period uses order date" }));
  if (stats.missingAmounts) root.append(h("div", { class: "eldorado-details-note", text: `${stats.missingAmounts} sales have no valid amount` }));
  if (warning) root.append(h("div", { class: "eldorado-status", text: warning }));
  return root;
}
