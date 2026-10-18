//! Read-only Eldorado Seller API. Credentials never cross the webview boundary.
use crate::{
    integrations::{IntegrationEvent, IntegrationUpdate},
    secrets,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    sync::LazyLock,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};

pub const ID: &str = "integration_eldorado";
const BASE: &str = "https://www.eldorado.gg";
const ORDERS: &str = "/api/orders/me/seller/orders?pageSize=50";
const NOTIFICATIONS: &str = "/api/notifications/me?notificationReadStatuses=IsUnread&notificationReadStatuses=IsRead&pageSize=50";
const MAX_JSON: usize = 2 * 1024 * 1024;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    configured: bool,
    codex_available: bool,
}
fn configured() -> bool {
    secrets::present("eldorado-client-id") && secrets::present("eldorado-client-secret")
}
fn codex_path() -> Option<std::path::PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(|p| std::path::PathBuf::from(p).join("Codex/Eldorado/credentials.json"))
}
#[tauri::command]
pub fn eldorado_status() -> Status {
    Status {
        configured: configured(),
        codex_available: cfg!(windows) && codex_path().is_some_and(|p| p.is_file()),
    }
}

#[cfg(windows)]
fn unprotect(raw: &str) -> Result<String, String> {
    use windows::Win32::{
        Foundation::{LocalFree, HLOCAL},
        Security::Cryptography::{
            CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        },
    };
    if raw.len() > 65536 || raw.len() % 2 != 0 || !raw.is_ascii() {
        return Err("Saved credentials have an invalid format.".into());
    }
    let mut bytes = (0..raw.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&raw[i..i + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "Saved credentials have an invalid format.")?;
    let input = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_mut_ptr(),
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptUnprotectData(
            &input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
        .map_err(|_| "Windows could not unlock the saved Codex credentials.")?;
        let data = std::slice::from_raw_parts_mut(output.pbData, output.cbData as usize);
        let words = data
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect::<Vec<_>>();
        let secret = String::from_utf16(&words)
            .map_err(|_| "Saved credentials have invalid text.".to_string());
        data.fill(0);
        let _ = LocalFree(Some(HLOCAL(output.pbData.cast())));
        secret
    }
}

/// Explicitly copies the existing account's DPAPI-protected credentials to this app's keystore.
#[tauri::command]
pub async fn eldorado_import_codex() -> Result<Status, String> {
    #[cfg(windows)]
    {
        let path = codex_path().ok_or("Codex credentials were not found.")?;
        let bytes = std::fs::read(path).map_err(|_| "Codex credentials were not found.")?;
        if bytes.len() > 131072 {
            return Err("Saved credentials are too large.".into());
        }
        let text = String::from_utf8(bytes).map_err(|_| "Saved credentials have invalid text.")?;
        let stored: Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))
            .map_err(|_| "Saved credentials have an invalid format.")?;
        let client = stored["clientId"]
            .as_str()
            .filter(|v| !v.is_empty() && v.len() < 4096)
            .ok_or("Saved client ID is missing.")?;
        let mut secret = unprotect(
            stored["protectedSecret"]
                .as_str()
                .ok_or("Saved secret is missing.")?,
        )?;
        if secret.is_empty() {
            return Err("Saved secret is empty.".into());
        }
        // Write the secret first: no credentials are ever returned to JavaScript.
        secrets::set("eldorado-client-secret", &secret)
            .map_err(|_| "Could not save the secret in Windows Credential Manager.")?;
        secrets::set("eldorado-client-id", client)
            .map_err(|_| "Could not save the client ID in Windows Credential Manager.")?;
        unsafe {
            secret.as_bytes_mut().fill(0);
        }
        *RUNTIME.lock().await = Runtime::default();
        *HISTORY.lock().await = None;
        Ok(eldorado_status())
    }
    #[cfg(not(windows))]
    {
        Err("Importing Codex credentials is supported on Windows.".into())
    }
}

#[derive(Default)]
struct Runtime {
    token: String,
    expires: Option<Instant>,
    blocked: bool,
    next_check: Option<Instant>,
    rate_limited_until: Option<Instant>,
    seen_orders: HashSet<String>,
    seen_notifications: HashSet<String>,
    initialized: bool,
}
static RUNTIME: LazyLock<tokio::sync::Mutex<Runtime>> =
    LazyLock::new(|| tokio::sync::Mutex::new(Runtime::default()));
pub fn credentials_changed() {
    tauri::async_runtime::spawn(async {
        *RUNTIME.lock().await = Runtime::default();
        *HISTORY.lock().await = None;
    });
}
fn text(v: &Value, key: &str, max: usize) -> String {
    v[key]
        .as_str()
        .unwrap_or("")
        .chars()
        .filter(|c| !c.is_control())
        .take(max)
        .collect()
}
fn rows(v: &Value) -> &[Value] {
    v["results"].as_array().map(Vec::as_slice).unwrap_or(&[])
}
fn plain(raw: &str) -> String {
    let mut inside = false;
    raw.chars()
        .filter(|c| {
            if *c == '<' {
                inside = true;
                false
            } else if *c == '>' {
                inside = false;
                false
            } else {
                !inside && !c.is_control()
            }
        })
        .take(300)
        .collect()
}
/// Only the fields this interface needs. Delivery credentials, descriptions and private IDs stay out.
fn order(v: &Value) -> Option<Value> {
    let id = text(v, "id", 100);
    if id.is_empty() {
        return None;
    }
    Some(
        json!({"id":id,"title":text(&v["orderOfferDetails"],"offerTitle",180),"buyer":text(v,"buyerUsername",80),
        "quantity":v["purchaseQuantity"].as_f64().unwrap_or(1.0),"amount":v["totalPrice"]["amount"].as_f64(),
        "currency":text(&v["totalPrice"],"currency",8),"state":text(&v["state"],"state",60),"createdAt":text(v,"createdDate",50),
        "category":text(&v["orderOfferDetails"],"gameCategoryTitle",80),"refunded":v["hasBeenRefundedPostCompletion"].as_bool().unwrap_or(false)}),
    )
}
fn notification(v: &Value) -> Option<Value> {
    let custom = !v["customNotification"].is_null();
    let n = if custom {
        &v["customNotification"]
    } else {
        &v["notification"]
    };
    let id = text(n, "id", 100);
    if id.is_empty() {
        return None;
    }
    Some(
        json!({"id":format!("{}:{id}",if custom{"custom"}else{"event"}),"event":if custom{"AccountNotification".into()}else{text(n,"event",100)},
        "title":if custom{text(n,"subject",180)}else{text(&n["details"],"title",180)},
        "text":if custom{plain(n["text"].as_str().unwrap_or(""))}else{String::new()},
        "orderId":if custom{String::new()}else{text(&n["details"],"detailsId",100)},
        "unread":n[if custom{"readStatus"}else{"notificationReadStatus"}].as_str()==Some("IsUnread"),
        "createdAt":text(n,"notificationDate",50)}),
    )
}

#[derive(Debug)]
struct Failure {
    status: u16,
}
impl Failure {
    fn message(&self) -> String {
        match self.status {
            401 | 403 => {
                "Eldorado rejected the connection. Check the saved credentials, then reconnect."
            }
            429 => "Eldorado is rate limiting requests. Retrying in five minutes.",
            _ => "Could not reach Eldorado. The last successful results are kept.",
        }
        .into()
    }
}
fn http() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap_or_default()
}
async fn json_response(response: reqwest::Response) -> Result<Value, Failure> {
    if !response.status().is_success() {
        return Err(Failure {
            status: response.status().as_u16(),
        });
    }
    let bytes = crate::net::read_capped(response, MAX_JSON)
        .await
        .map_err(|_| Failure { status: 0 })?;
    serde_json::from_slice(&bytes).map_err(|_| Failure { status: 0 })
}
async fn authenticate(runtime: &mut Runtime, client: &reqwest::Client) -> Result<(), Failure> {
    if !runtime.token.is_empty() && runtime.expires.is_some_and(|e| e > Instant::now()) {
        return Ok(());
    }
    let id = secrets::get("eldorado-client-id").ok_or(Failure { status: 401 })?;
    let secret = secrets::get("eldorado-client-secret").ok_or(Failure { status: 401 })?;
    let response = client
        .post(format!("{BASE}/api/authentication/seller/token"))
        .json(&json!({"clientId":id,"clientSecret":secret}))
        .send()
        .await
        .map_err(|_| Failure { status: 0 })?;
    let auth = json_response(response).await?;
    runtime.token = auth["accessToken"]
        .as_str()
        .filter(|s| s.len() < 32768)
        .ok_or(Failure { status: 401 })?
        .to_string();
    if runtime.token.is_empty() {
        return Err(Failure { status: 401 });
    }
    runtime.expires = Some(
        Instant::now()
            + Duration::from_secs(auth["expiresIn"].as_u64().unwrap_or(900).clamp(60, 900) - 30),
    );
    Ok(())
}
async fn get(client: &reqwest::Client, path: &str, token: &str) -> Result<Value, Failure> {
    // Fixed allowlist: no arbitrary endpoints, redirects, account writes or message reads.
    if ![ORDERS, NOTIFICATIONS].contains(&path) {
        return Err(Failure { status: 0 });
    }
    json_response(
        client
            .get(format!("{BASE}{path}"))
            .bearer_auth(token)
            .send()
            .await
            .map_err(|_| Failure { status: 0 })?,
    )
    .await
}
fn new_items(seen: &mut HashSet<String>, items: &[Value], initialized: bool) -> Vec<Value> {
    let fresh = items
        .iter()
        .filter(|v| !seen.contains(v["id"].as_str().unwrap_or("")))
        .cloned()
        .collect::<Vec<_>>();
    seen.extend(
        items
            .iter()
            .filter_map(|v| v["id"].as_str().map(str::to_owned)),
    );
    if seen.len() > 2000 {
        *seen = items
            .iter()
            .filter_map(|v| v["id"].as_str().map(str::to_owned))
            .collect();
    }
    if initialized {
        fresh
    } else {
        vec![]
    }
}
fn update(
    runtime: &mut Runtime,
    orders: Value,
    notifications: Value,
) -> (Value, Option<IntegrationEvent>) {
    let orders = rows(&orders).iter().filter_map(order).collect::<Vec<_>>();
    let notifications = rows(&notifications)
        .iter()
        .filter_map(notification)
        .collect::<Vec<_>>();
    let old_orders = runtime.seen_orders.clone();
    let new_orders = new_items(&mut runtime.seen_orders, &orders, runtime.initialized);
    let new_notifications = new_items(
        &mut runtime.seen_notifications,
        &notifications,
        runtime.initialized,
    )
    .into_iter()
    .filter(|n| {
        // An OrderCreated notice for an already-known or freshly reported order is not a second alert.
        !(n["event"] == "OrderCreated"
            && (old_orders.contains(n["orderId"].as_str().unwrap_or(""))
                || runtime
                    .seen_orders
                    .contains(n["orderId"].as_str().unwrap_or(""))))
    })
    .collect::<Vec<_>>();
    runtime.initialized = true;
    let event = if !new_orders.is_empty() {
        Some(IntegrationEvent {
            success: true,
            label: if new_orders.len() == 1 {
                "New Eldorado Order".into()
            } else {
                format!("{} New Eldorado Orders", new_orders.len())
            },
            detail: Some(new_orders[0]["title"].as_str().unwrap_or("").to_string()),
        })
    } else if !new_notifications.is_empty() {
        Some(IntegrationEvent {
            success: true,
            label: if new_notifications.len() == 1 {
                "New Eldorado Notification".into()
            } else {
                format!("{} New Eldorado Notifications", new_notifications.len())
            },
            detail: Some(
                new_notifications[0]["title"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
            ),
        })
    } else {
        None
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    (
        json!({"orders":orders,"notifications":notifications,"checkedAt":now,"newOrders":new_orders.len(),"newNotifications":new_notifications.len()}),
        event,
    )
}

// On-demand history stays independent of the live alert baseline.
static HISTORY: LazyLock<tokio::sync::Mutex<Option<(Instant, Value)>>> =
    LazyLock::new(|| tokio::sync::Mutex::new(None));

#[tauri::command]
pub async fn eldorado_details(app: AppHandle) -> Result<Value, String> {
    let enabled = || {
        app.try_state::<crate::Shared>().is_some_and(|s| {
            s.settings
                .lock()
                .unwrap()
                .active_integrations
                .iter()
                .any(|id| id == ID)
        }) && !crate::integrations::PAUSED.load(std::sync::atomic::Ordering::Relaxed)
    };
    if !enabled() || !configured() {
        return Err("Connect Eldorado to view sales details.".into());
    }
    let mut cache = HISTORY.lock().await;
    if let Some((at, data)) = &*cache {
        if at.elapsed() < Duration::from_secs(60) {
            return Ok(data.clone());
        }
    }
    let client = http();
    let token = {
        let mut runtime = RUNTIME.lock().await;
        if runtime.blocked
            || runtime.rate_limited_until.is_some_and(|t| t > Instant::now())
        {
            return Err("Eldorado is unavailable. Try Refresh later.".into());
        }
        authenticate(&mut runtime, &client)
            .await
            .map_err(|e| e.message())?;
        runtime.token.clone()
    };
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    let mut cursors = HashSet::new();
    let mut cursor = String::new();
    let mut complete = false;
    let mut warning = String::new();
    // Finite pagination budget; partial history is explicitly labelled in the view.
    for _ in 0..100 {
        if !enabled() {
            return Err("Eldorado monitoring is paused or disconnected.".into());
        }
        let mut request = client
            .get(format!("{BASE}/api/orders/me/seller/orders"))
            .bearer_auth(&token)
            .query(&[("pageSize", "50")]);
        if !cursor.is_empty() {
            request = request.query(&[("cursorValue", cursor.as_str()), ("pageDirection", "Next")]);
        }
        let result = match request.send().await {
            Ok(response) => json_response(response).await,
            Err(_) => Err(Failure { status: 0 }),
        };
        let page = match result {
            Ok(page) if page["results"].is_array() => page,
            result => {
                let failure = result.err().unwrap_or(Failure { status: 0 });
                let mut runtime = RUNTIME.lock().await;
                if matches!(failure.status, 401 | 403) {
                    runtime.blocked = true;
                }
                if failure.status == 429 {
                    runtime.next_check = Some(Instant::now() + Duration::from_secs(300));
                    runtime.rate_limited_until = runtime.next_check;
                }
                warning = failure.message();
                if items.is_empty() {
                    return Err(warning);
                }
                break;
            }
        };
        for item in rows(&page).iter().filter_map(order) {
            if seen.insert(item["id"].as_str().unwrap_or("").to_string()) {
                items.push(item);
            }
        }
        cursor = text(&page, "nextPageCursor", 1024);
        if cursor.is_empty() {
            complete = true;
            break;
        }
        if !cursors.insert(cursor.clone()) {
            warning = "History pagination repeated results.".into();
            break;
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
    if !complete && warning.is_empty() {
        warning = "Showing the latest 5,000 orders.".into();
    }
    let data = json!({"orders":items,"complete":complete,"warning":warning});
    *cache = Some((Instant::now(), data.clone()));
    Ok(data)
}

pub async fn refresh(app: AppHandle, manual: bool) {
    let Some(shared) = app.try_state::<crate::Shared>() else {
        return;
    };
    let settings = shared.settings.lock().unwrap().clone();
    if !settings.active_integrations.iter().any(|s| s == ID) {
        if let Ok(mut runtime) = RUNTIME.try_lock() {
            *runtime = Runtime::default();
        }
        return;
    }
    if crate::integrations::PAUSED.load(std::sync::atomic::Ordering::Relaxed) || !configured() {
        return;
    }
    let Ok(mut runtime) = RUNTIME.try_lock() else {
        return;
    };
    if manual && runtime.blocked {
        runtime.blocked = false;
        runtime.next_check = None;
    }
    if runtime.blocked || runtime.next_check.is_some_and(|t| t > Instant::now()) {
        return;
    }
    let seconds = if settings.eldorado_poll_interval.is_finite() {
        settings.eldorado_poll_interval.clamp(15.0, 300.0) as u64
    } else {
        15
    };
    runtime.next_check =
        Some(Instant::now() + Duration::from_secs(if manual { 5 } else { seconds }));
    let client = http();
    let mut result = Err(Failure { status: 0 });
    for attempt in 0..2 {
        result = async {
            authenticate(&mut runtime, &client).await?;
            let orders = get(&client, ORDERS, &runtime.token).await?;
            let notifications = get(&client, NOTIFICATIONS, &runtime.token).await?;
            Ok::<_, Failure>((orders, notifications))
        }
        .await;
        if matches!(&result,Err(e) if e.status==401) && attempt == 0 {
            runtime.token.clear();
            runtime.expires = None;
        } else {
            break;
        }
    }
    if crate::integrations::PAUSED.load(std::sync::atomic::Ordering::Relaxed)
        || !shared
            .settings
            .lock()
            .unwrap()
            .active_integrations
            .iter()
            .any(|s| s == ID)
    {
        return;
    }
    let message = match result {
        Ok((orders, notifications)) => {
            let (data, event) = update(&mut runtime, orders, notifications);
            IntegrationUpdate {
                id: ID,
                data,
                error: None,
                event,
            }
        }
        Err(e) => {
            if e.status == 429 {
                runtime.rate_limited_until = Some(Instant::now() + Duration::from_secs(300));
            }
            if matches!(e.status, 401 | 403) {
                runtime.blocked = true;
            } else {
                runtime.next_check = Some(
                    Instant::now() + Duration::from_secs(if e.status == 429 { 300 } else { 30 }),
                );
            }
            IntegrationUpdate {
                id: ID,
                data: json!({}),
                error: Some(e.message()),
                event: None,
            }
        }
    };
    // Turning off or pausing while the request was in flight also suppresses the update.
    if !crate::integrations::PAUSED.load(std::sync::atomic::Ordering::Relaxed)
        && shared
            .settings
            .lock()
            .unwrap()
            .active_integrations
            .iter()
            .any(|s| s == ID)
    {
        let _ = app.emit_to(crate::island::WINDOW_LABEL, "integration", message);
    }
}
pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(2));
        loop {
            ticker.tick().await;
            refresh(app.clone(), false).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn orders(id: &str) -> Value {
        json!({"results":[{"id":id,"orderOfferDetails":{"offerTitle":"Test order","description":"private delivery info"},"buyerUsername":"Buyer","sellerId":"private-id","state":{"state":"Paid"},"totalPrice":{"amount":12.5,"currency":"USD"},"createdDate":"2026-10-18T10:00:00Z"}]})
    }
    #[test]
    fn first_snapshot_is_silent_and_only_new_ids_alert() {
        let mut r = Runtime::default();
        assert!(update(&mut r, orders("one"), json!({"results":[]}))
            .1
            .is_none());
        assert!(update(&mut r, orders("one"), json!({"results":[]}))
            .1
            .is_none());
        assert!(update(&mut r, orders("two"), json!({"results":[]}))
            .1
            .is_some());
        assert!(update(&mut r, orders("two"), json!({"results":[]}))
            .1
            .is_none());
    }
    #[test]
    fn order_projection_excludes_private_fields() {
        let (data, _) = update(
            &mut Runtime::default(),
            orders("one"),
            json!({"results":[]}),
        );
        let s = data.to_string();
        assert!(!s.contains("private"));
        assert_eq!(data["orders"][0]["amount"], 12.5);
        assert_eq!(data["orders"][0]["state"], "Paid");
    }
    #[test]
    fn notifications_are_independent_and_old_order_notices_do_not_alert_twice() {
        let mut r = Runtime::default();
        update(&mut r, orders("one"), json!({"results":[]}));
        let n = json!({"results":[{"notification":{"id":"n1","event":"OrderCreated","details":{"detailsId":"one","title":"Test"}}}]});
        assert!(update(&mut r, orders("one"), n).1.is_none());
        let n = json!({"results":[{"notification":{"id":"n2","event":"OrderDisputed","details":{"detailsId":"one","title":"Test"}}}]});
        assert!(update(&mut r, orders("one"), n.clone()).1.is_some());
        assert!(update(&mut r, orders("one"), n).1.is_none());
    }
    #[test]
    fn custom_notifications_strip_markup_and_ignore_remote_redirects() {
        let n=notification(&json!({"customNotification":{"id":"n","subject":"Notice","text":"<b>Hello</b>","redirectUrl":"https://evil.example/","readStatus":"IsUnread"}})).unwrap();
        assert_eq!(n["text"], "Hello");
        assert_eq!(n["unread"], true);
        assert!(!n.to_string().contains("evil"));
    }
}
