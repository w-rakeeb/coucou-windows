import { State } from "../core/state";
import { Bridge } from "../core/bridge";
import { h, clear } from "./dom";
import type { ViewActions, ViewHost } from "./views";
import { t } from "../i18n/i18n";

export const ELDORADO = "integration_eldorado";
export const ELDORADO_ORDERS = "https://www.eldorado.gg/dashboard/orders/sold";
export const ELDORADO_NOTIFICATIONS = "https://www.eldorado.gg/dashboard/notifications";
type Row = Record<string, unknown>;
function rows(key: string): Row[] {
  const value = State.integrations[ELDORADO]?.data[key];
  return Array.isArray(value) ? value.filter(v => v && typeof v === "object").slice(0,50) : [];
}
export function eldoradoMoney(row: Row): string {
  if (typeof row.amount !== "number" || !Number.isFinite(row.amount)) return "";
  const currency = typeof row.currency === "string" ? row.currency : "USD";
  try { return new Intl.NumberFormat(undefined,{style:"currency",currency}).format(row.amount); }
  catch { return `${row.amount.toFixed(2)} ${currency}`; }
}
export function eldoradoEvent(value: unknown): string {
  return String(value ?? "Account Notification").replace(/([a-z])([A-Z])/g,"$1 $2");
}
export function eldoradoCard(open: () => void): HTMLElement {
  const info=State.integrations[ELDORADO];
  const orders=rows("orders");
  const body=h("div",{class:"int-card eldorado-summary"},
    h("div",{class:"eldorado-summary-head"},h("strong",{text:"Eldorado"}),h("button",{class:"link-btn",text:"View",onclick:open})),
  );
  if(info?.error)body.append(h("div",{class:"int-status",text:info.error}));
  else if(!info?.loaded)body.append(h("div",{class:"int-status",text:info?.configured?"Checking orders…":"Connect in Settings → Connections"}));
  else if(!orders.length)body.append(h("div",{class:"int-status",text:"No recent orders"}));
  else for(const row of orders.slice(0,3))body.append(h("div",{class:"eldorado-summary-row"},h("span",{text:String(row.title??"Order"),title:String(row.title??"")}),h("span",{text:eldoradoMoney(row)})));
  return body;
}

export function buildEldorado(actions: ViewActions): ViewHost {
  let tab="orders",lastKey="";
  let shownAlert=State.eldoradoAlertSequence;
  const list=h("div",{class:"eldorado-list"});
  const status=h("div",{class:"eldorado-status"});
  const ordersTab=h("button",{text:"Orders",onclick:()=>{tab="orders";lastKey="";sync();}});
  const noticesTab=h("button",{text:"Notifications",onclick:()=>{tab="notifications";lastKey="";sync();}});
  const close=h("button",{class:"link-btn",text:"Dismiss",onclick:actions.collapse});
  const refresh=h("button",{class:"link-btn",text:t("Refresh"),onclick:()=>void Bridge.refreshIntegration(ELDORADO)});
  const open=h("button",{class:"link-btn",text:"Open Eldorado",onclick:()=>void Bridge.openUrl(tab==="orders"?ELDORADO_ORDERS:ELDORADO_NOTIFICATIONS)});
  const el=h("div",{class:"view"},h("div",{class:"card eldorado-card"},
    h("div",{class:"eldorado-content"},
      h("div",{class:"eldorado-heading"},h("strong",{text:"Eldorado"}),refresh,open,close),
      h("div",{class:"eldorado-tabs"},ordersTab,noticesTab),status,list)));
  function sync(){
    if (State.view === "eldorado" && shownAlert !== State.eldoradoAlertSequence) {
      shownAlert=State.eldoradoAlertSequence;tab=State.eldoradoAlertTab;lastKey="";
    }
    const info=State.integrations[ELDORADO];
    const key=JSON.stringify([tab,info]);if(key===lastKey)return;lastKey=key;
    ordersTab.setAttribute("aria-pressed",String(tab==="orders"));noticesTab.setAttribute("aria-pressed",String(tab==="notifications"));
    const checked=info?.data.checkedAt;
    status.textContent=info?.error??(typeof checked==="number"?`Checked ${new Date(checked).toLocaleTimeString(undefined,{hour:"2-digit",minute:"2-digit",second:"2-digit"})} · Every ${State.settings.eldoradoPollInterval}s`:t("Connecting…"));
    clear(list);
    const entries=rows(tab);
    if(!entries.length)list.append(h("div",{class:"eldorado-empty",text:info?.loaded?`No recent ${tab}`:"Waiting for the first check"}));
    for(const row of entries){
      const top=tab==="orders"?String(row.title||"Order"):eldoradoEvent(row.event);
      const bottom=tab==="orders"?[row.buyer,row.state,row.quantity?`Qty ${row.quantity}`:""].filter(Boolean).join(" · "):String(row.title||row.text||"");
      const when=Date.parse(String(row.createdAt??""));
      const date=Number.isFinite(when)?new Date(when).toLocaleString(undefined,{month:"short",day:"numeric",hour:"2-digit",minute:"2-digit"}):"";
      list.append(h("button",{class:`eldorado-row${row.unread?" unread":""}`,onclick:()=>void Bridge.openUrl(tab==="orders"?ELDORADO_ORDERS:ELDORADO_NOTIFICATIONS)},
        h("span",{class:"eldorado-row-main"},h("strong",{text:top}),h("span",{text:bottom})),
        h("span",{class:"eldorado-row-meta"},h("strong",{text:tab==="orders"?eldoradoMoney(row):row.unread?"Unread":""}),h("span",{text:date}))));
    }
  }
  return {el,sync};
}
