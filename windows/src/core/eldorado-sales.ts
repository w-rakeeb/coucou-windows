type Row = Record<string, unknown>;

export function salesStats(input: Row[], days = 0, now = Date.now()) {
  const unique = new Map<string, Row>();
  for (const row of input) if (typeof row.id === "string" && row.id) unique.set(row.id, row);
  const orders = [...unique.values()].filter(row => !days || Date.parse(String(row.createdAt)) >= now - days * 86400000);
  const sales = orders.filter(row => ["Received", "Completed"].includes(String(row.state)) && row.refunded !== true);
  const totals = new Map<string, { amount: number; count: number }>();
  const categories = new Map<string, number>();
  for (const row of sales) {
    const category = String(row.category || "Other");
    categories.set(category, (categories.get(category) ?? 0) + 1);
    if (typeof row.amount !== "number" || !Number.isFinite(row.amount) || row.amount < 0 || typeof row.currency !== "string" || !/^[A-Z]{3}$/.test(row.currency)) continue;
    const group = totals.get(row.currency) ?? { amount: 0, count: 0 };
    group.amount += Math.round(row.amount * 100); group.count++;
    totals.set(row.currency, group);
  }
  return {
    orders: orders.length, sales: sales.length,
    cancelled: orders.filter(row => ["Canceled", "Cancelled"].includes(String(row.state))).length,
    refunded: orders.filter(row => row.refunded === true).length,
    active: orders.filter(row => !["Received", "Completed", "Canceled", "Cancelled"].includes(String(row.state)) && row.refunded !== true).length,
    totals: [...totals].sort(([a], [b]) => a.localeCompare(b)).map(([currency, v]) => ({ currency, amount: v.amount / 100, average: v.amount / 100 / v.count })),
    categories: [...categories].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0])).slice(0, 3),
    missingAmounts: sales.length - [...totals.values()].reduce((n, v) => n + v.count, 0),
  };
}

