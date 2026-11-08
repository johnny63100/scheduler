//! Cloud sync through Supabase, so a phone can see and add events and todos.
//!
//! The local database stays the source of truth. A background thread
//! - uploads a snapshot of nearby events and open todos to `nora_snapshot`, and
//! - applies requests the phone left in `nora_inbox` (add event, add todo, tick a todo),
//!   then tells the windows to reload.
//!
//! Only the project URL, the public anon key, the account e-mail and a refresh token are kept
//! (in the settings table); the password is used once to sign in and never stored.
//! Table definitions: `mobile/supabase.sql`.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter, Listener};

use crate::db::{self, Db, Event, Scope};

pub const KEY_URL: &str = "cloud.url";
pub const KEY_ANON: &str = "cloud.anonKey";
pub const KEY_EMAIL: &str = "cloud.email";
pub const KEY_USER: &str = "cloud.userId";
pub const KEY_REFRESH: &str = "cloud.refreshToken";
const KEY_LAST_INBOX: &str = "cloud.lastInbox";
const KEY_LAST_PUSH: &str = "cloud.lastPush";
const KEY_LAST_ERROR: &str = "cloud.lastError";

/// How often the phone's requests are picked up when nothing else happens.
const POLL_EVERY: Duration = Duration::from_secs(20);
/// A fresh snapshot goes up at least this often (so "today" and routines stay current).
const PUSH_EVERY: Duration = Duration::from_secs(10 * 60);
/// Local edits arriving in a burst are sent together.
const SETTLE: Duration = Duration::from_millis(1500);
/// Events in the snapshot: this many days back and ahead of today.
const DAYS_BACK: i64 = 14;
const DAYS_AHEAD: i64 = 120;
/// Finished todos stay visible on the phone for this many days.
const DONE_KEEP_DAYS: i64 = 3;
const DEFAULT_COLOR: u32 = 0x7c74ff;

const DATA_EVENT: &str = "nora://data";
const FROM_CLOUD: &str = "cloud";

/// Managed state: wakes the sync thread.
pub struct Cloud {
    wake: Sender<()>,
}

impl Cloud {
    pub fn wake(&self) {
        let _ = self.wake.send(());
    }
}

#[derive(Serialize)]
pub struct CloudStatus {
    pub configured: bool,
    pub url: String,
    pub email: String,
    /// RFC 3339 time of the last successful upload.
    pub last_push: Option<String>,
    pub last_error: Option<String>,
}

pub fn status(db: &Db) -> db::DbResult<CloudStatus> {
    let get = |k: &str| db.setting(k).map(|v| v.unwrap_or_default());
    Ok(CloudStatus {
        configured: !get(KEY_REFRESH)?.is_empty() && !get(KEY_URL)?.is_empty(),
        url: get(KEY_URL)?,
        email: get(KEY_EMAIL)?,
        last_push: db.setting(KEY_LAST_PUSH)?.filter(|s| !s.is_empty()),
        last_error: db.setting(KEY_LAST_ERROR)?.filter(|s| !s.is_empty()),
    })
}

pub fn sign_out(db: &Db) -> db::DbResult<()> {
    for key in [KEY_REFRESH, KEY_USER, KEY_LAST_ERROR, KEY_LAST_PUSH] {
        db.set_setting(key, "")?;
    }
    Ok(())
}

/// Signs in with e-mail and password and stores the session (blocking; call off the main thread).
pub fn sign_in(db: &Db, url: &str, anon_key: &str, email: &str, password: &str) -> Result<(), String> {
    let url = url.trim().trim_end_matches('/');
    if !url.starts_with("https://") {
        return Err("프로젝트 URL은 https:// 로 시작해야 해요.".into());
    }
    let client = client()?;
    let session = auth_request(&client, url, anon_key.trim(), "password", json!({ "email": email.trim(), "password": password }))?;
    db.set_setting(KEY_URL, url).map_err(|e| e.to_string())?;
    db.set_setting(KEY_ANON, anon_key.trim()).map_err(|e| e.to_string())?;
    db.set_setting(KEY_EMAIL, email.trim()).map_err(|e| e.to_string())?;
    db.set_setting(KEY_USER, &session.user.id).map_err(|e| e.to_string())?;
    db.set_setting(KEY_REFRESH, &session.refresh_token).map_err(|e| e.to_string())?;
    db.set_setting(KEY_LAST_ERROR, "").map_err(|e| e.to_string())?;
    Ok(())
}

/// Starts the sync thread and listens for local edits. Returns the managed state.
pub fn start(app: AppHandle, db_path: std::path::PathBuf) -> Cloud {
    let (tx, rx) = mpsc::channel();
    // Every write in a window broadcasts `nora://data`; upload soon after.
    let on_edit = tx.clone();
    app.listen_any(DATA_EVENT, move |event| {
        let from_cloud = serde_json::from_str::<Value>(event.payload())
            .ok()
            .and_then(|v| v.get("from").and_then(|f| f.as_str()).map(|f| f == FROM_CLOUD))
            .unwrap_or(false);
        if !from_cloud {
            let _ = on_edit.send(());
        }
    });
    let spawned = std::thread::Builder::new().name("cloud".into()).spawn(move || {
        let db = match db::open_conn(&db_path) {
            Ok(conn) => Db { conn },
            Err(e) => return eprintln!("cloud sync could not open database: {e}"),
        };
        run(app, db, rx);
    });
    if let Err(e) = spawned {
        eprintln!("failed to start cloud sync: {e}");
    }
    Cloud { wake: tx }
}

// ---------------------------------------------------------------------------

struct Worker {
    app: AppHandle,
    db: Db,
    client: reqwest::blocking::Client,
    /// Access token, the refresh token it came with, and when it should be renewed.
    token: Option<(String, String, Instant)>,
    dirty: bool,
    last_push: Option<Instant>,
}

fn run(app: AppHandle, db: Db, rx: Receiver<()>) {
    let client = match client() {
        Ok(c) => c,
        Err(e) => return eprintln!("cloud sync: {e}"),
    };
    let mut w = Worker { app, db, client, token: None, dirty: true, last_push: None };
    loop {
        match rx.recv_timeout(POLL_EVERY) {
            Ok(()) => {
                w.dirty = true;
                std::thread::sleep(SETTLE);
                while rx.try_recv().is_ok() {}
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        match w.cycle() {
            Ok(()) => {}
            Err(e) => {
                w.token = None;
                let _ = w.db.set_setting(KEY_LAST_ERROR, &e);
            }
        }
    }
}

struct Config {
    url: String,
    anon: String,
    user: String,
    refresh: String,
}

impl Worker {
    fn config(&self) -> Option<Config> {
        let get = |k: &str| self.db.setting(k).ok().flatten().filter(|s| !s.is_empty());
        Some(Config { url: get(KEY_URL)?, anon: get(KEY_ANON)?, user: get(KEY_USER)?, refresh: get(KEY_REFRESH)? })
    }

    fn cycle(&mut self) -> Result<(), String> {
        let Some(cfg) = self.config() else {
            self.token = None;
            return Ok(());
        };
        let access = self.access_token(&cfg)?;
        let applied = self.apply_inbox(&cfg, &access)?;
        if applied {
            let _ = self.app.emit(DATA_EVENT, json!({ "from": FROM_CLOUD }));
            self.dirty = true;
        }
        let stale = self.last_push.is_none_or(|t| t.elapsed() >= PUSH_EVERY);
        if self.dirty || stale {
            self.push_snapshot(&cfg, &access)?;
            self.dirty = false;
            self.last_push = Some(Instant::now());
            let _ = self.db.set_setting(KEY_LAST_PUSH, &chrono::Utc::now().to_rfc3339());
            let _ = self.db.set_setting(KEY_LAST_ERROR, "");
        }
        Ok(())
    }

    /// A valid access token, renewing it with the stored refresh token when needed.
    fn access_token(&mut self, cfg: &Config) -> Result<String, String> {
        if let Some((access, refresh, renew_at)) = &self.token
            && *refresh == cfg.refresh
            && Instant::now() < *renew_at
        {
            return Ok(access.clone());
        }
        let session = auth_request(&self.client, &cfg.url, &cfg.anon, "refresh_token", json!({ "refresh_token": cfg.refresh }))
            .map_err(|e| format!("로그인이 만료됐어요. 설정에서 다시 로그인해 주세요. ({e})"))?;
        // Refresh tokens are single-use: keep the new one right away.
        self.db.set_setting(KEY_REFRESH, &session.refresh_token).map_err(|e| e.to_string())?;
        let renew_at = Instant::now() + Duration::from_secs(session.expires_in.saturating_sub(120).max(60));
        self.token = Some((session.access_token.clone(), session.refresh_token, renew_at));
        Ok(session.access_token)
    }

    fn rest(&self, cfg: &Config, access: &str, method: reqwest::Method, path: &str) -> reqwest::blocking::RequestBuilder {
        self.client
            .request(method, format!("{}/rest/v1/{path}", cfg.url))
            .header("apikey", &cfg.anon)
            .bearer_auth(access)
    }

    /// Applies the phone's pending requests in order. Returns whether anything changed locally.
    fn apply_inbox(&self, cfg: &Config, access: &str) -> Result<bool, String> {
        let items: Vec<InboxItem> = check(
            self.rest(cfg, access, reqwest::Method::GET, "nora_inbox?select=id,kind,payload&applied_at=is.null&order=id.asc&limit=100")
                .send(),
        )?
        .json()
        .map_err(|e| e.to_string())?;
        // Ids up to this one were applied already (the remote mark may have failed afterwards).
        let last: i64 = self.db.setting_or(KEY_LAST_INBOX, 0);
        let mut changed = false;
        for item in items {
            let error = if item.id > last {
                let result = apply(&self.db, &item);
                self.db.set_setting(KEY_LAST_INBOX, &item.id.to_string()).map_err(|e| e.to_string())?;
                changed |= result.is_ok();
                result.err()
            } else {
                None
            };
            check(
                self.rest(cfg, access, reqwest::Method::PATCH, &format!("nora_inbox?id=eq.{}", item.id))
                    .header("Prefer", "return=minimal")
                    .json(&json!({ "applied_at": chrono::Utc::now().to_rfc3339(), "error": error }))
                    .send(),
            )?;
        }
        Ok(changed)
    }

    fn push_snapshot(&self, cfg: &Config, access: &str) -> Result<(), String> {
        let data = snapshot(&self.db).map_err(|e| e.to_string())?;
        check(
            self.rest(cfg, access, reqwest::Method::POST, "nora_snapshot?on_conflict=owner")
                .header("Prefer", "resolution=merge-duplicates,return=minimal")
                .json(&json!({ "owner": cfg.user, "data": data, "updated_at": chrono::Utc::now().to_rfc3339() }))
                .send(),
        )?;
        // Requests applied more than a week ago are no longer needed.
        // `Z` form: no `+` that would need escaping in the URL.
        let cutoff = (chrono::Utc::now() - chrono::Duration::days(7)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let _ = self.rest(cfg, access, reqwest::Method::DELETE, &format!("nora_inbox?applied_at=lt.{cutoff}")).send();
        Ok(())
    }
}

// ---- snapshot -----------------------------------------------------------------

fn snapshot(db: &Db) -> db::DbResult<Value> {
    let today = Local::now().date_naive();
    let from = today - chrono::Duration::days(DAYS_BACK);
    let to = today + chrono::Duration::days(DAYS_AHEAD);
    let events: Vec<Value> = db
        .events_between(from, to)?
        .into_iter()
        .map(|e| {
            json!({
                "id": e.id,
                "occurrence": e.occurrence,
                "title": e.title,
                "start": e.start,
                "end": e.end,
                "all_day": e.all_day,
                "color": e.color,
                "location": e.location,
                "memo": e.memo,
                "cancelled": e.cancelled,
                "repeats": e.repeat.is_some(),
            })
        })
        .collect();
    // Today's routine copies are made the first time todos are read each day.
    db.materialize_routines(today)?;
    let recent = db::now_ts() - DONE_KEEP_DAYS * 86_400;
    let todos: Vec<Value> = db
        .todos()?
        .into_iter()
        .filter(|t| !t.done || t.completed_at.is_some_and(|c| c >= recent))
        .map(|t| {
            json!({
                "id": t.id,
                "group_id": t.group_id,
                "parent_id": t.parent_id,
                "title": t.title,
                "done": t.done,
                "due": t.due,
                "due_time": t.due_time,
            })
        })
        .collect();
    let groups: Vec<Value> = db
        .todo_groups()?
        .into_iter()
        .map(|g| json!({ "id": g.id, "name": g.name, "color": g.color }))
        .collect();
    Ok(json!({
        "version": 1,
        "generated_at": chrono::Utc::now().to_rfc3339(),
        "today": today,
        "from": from,
        "to": to,
        "events": events,
        "todo_groups": groups,
        "todos": todos,
    }))
}

// ---- inbox ------------------------------------------------------------------

#[derive(Deserialize)]
struct InboxItem {
    id: i64,
    kind: String,
    payload: Value,
}

#[derive(Deserialize)]
struct NewEvent {
    title: String,
    start_date: NaiveDate,
    end_date: Option<NaiveDate>,
    start_time: Option<String>,
    end_time: Option<String>,
    #[serde(default)]
    all_day: bool,
    #[serde(default)]
    location: String,
    #[serde(default)]
    memo: String,
    color: Option<u32>,
    reminder: Option<i64>,
}

#[derive(Deserialize)]
struct NewTodo {
    title: String,
    group_id: Option<i64>,
    due: Option<NaiveDate>,
    due_time: Option<String>,
}

#[derive(Deserialize)]
struct TodoDone {
    id: i64,
    done: bool,
}

fn parse<T: for<'de> Deserialize<'de>>(v: &Value) -> Result<T, String> {
    serde_json::from_value(v.clone()).map_err(|e| format!("잘못된 요청: {e}"))
}

fn time(s: &Option<String>) -> Option<NaiveTime> {
    s.as_deref().and_then(db::normalize_time).and_then(|t| NaiveTime::parse_from_str(&t, "%H:%M").ok())
}

fn apply(db: &Db, item: &InboxItem) -> Result<(), String> {
    match item.kind.as_str() {
        "add_event" => {
            let n: NewEvent = parse(&item.payload)?;
            let title = n.title.trim();
            if title.is_empty() {
                return Err("제목이 비어 있어요".into());
            }
            let end_date = n.end_date.unwrap_or(n.start_date).max(n.start_date);
            let (start, end) = if n.all_day {
                (n.start_date.and_time(NaiveTime::MIN), end_date.and_time(NaiveTime::MIN))
            } else {
                let start: NaiveDateTime = n.start_date.and_time(time(&n.start_time).unwrap_or(NaiveTime::from_hms_opt(9, 0, 0).unwrap()));
                let end = time(&n.end_time).map(|t| end_date.and_time(t)).filter(|e| *e > start);
                (start, end.unwrap_or(start + chrono::Duration::hours(1)))
            };
            let event = Event {
                id: 0,
                title: title.into(),
                start,
                end,
                all_day: n.all_day,
                color: n.color.unwrap_or(DEFAULT_COLOR),
                location: n.location.trim().into(),
                links: vec![],
                memo: n.memo,
                repeat: None,
                exdates: vec![],
                occurrence: None,
                cancelled: false,
                tags: vec![],
                reminder: n.reminder.filter(|m| *m >= 0),
            };
            db.save_event(&event, Scope::All).map(|_| ()).map_err(|e| e.to_string())
        }
        "add_todo" => {
            let n: NewTodo = parse(&item.payload)?;
            let title = n.title.trim();
            if title.is_empty() {
                return Err("제목이 비어 있어요".into());
            }
            // A group deleted on the PC in the meantime: add it ungrouped instead of failing.
            let group = n.group_id.filter(|g| db.todo_groups().map(|gs| gs.iter().any(|x| x.id == *g)).unwrap_or(false));
            db.add_todo(group, None, title, n.due, n.due_time.as_deref()).map(|_| ()).map_err(|e| e.to_string())
        }
        "set_todo_done" => {
            let n: TodoDone = parse(&item.payload)?;
            db.set_todo_done(n.id, n.done).map_err(|e| e.to_string())
        }
        other => Err(format!("알 수 없는 요청: {other}")),
    }
}

// ---- HTTP -------------------------------------------------------------------

#[derive(Deserialize)]
struct Session {
    access_token: String,
    refresh_token: String,
    #[serde(default = "default_expiry")]
    expires_in: u64,
    user: SessionUser,
}

#[derive(Deserialize)]
struct SessionUser {
    id: String,
}

fn default_expiry() -> u64 {
    3600
}

fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent(concat!("NoraSchedule/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())
}

fn auth_request(client: &reqwest::blocking::Client, url: &str, anon: &str, grant: &str, body: Value) -> Result<Session, String> {
    let response = check(
        client
            .post(format!("{url}/auth/v1/token?grant_type={grant}"))
            .header("apikey", anon)
            .json(&body)
            .send(),
    )?;
    response.json().map_err(|e| e.to_string())
}

/// Turns transport errors and non-2xx answers into readable messages.
fn check(result: reqwest::Result<reqwest::blocking::Response>) -> Result<reqwest::blocking::Response, String> {
    let response = result.map_err(|e| format!("연결 실패: {e}"))?;
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let body = response.text().unwrap_or_default();
    let message = serde_json::from_str::<Value>(&body)
        .ok()
        .and_then(|v| {
            ["msg", "message", "error_description", "error"]
                .iter()
                .find_map(|k| v.get(*k).and_then(|m| m.as_str()).map(str::to_owned))
        })
        .unwrap_or(body);
    Err(format!("{status}: {message}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db() -> Db {
        let path = std::env::temp_dir().join(format!("nora-cloud-test-{}.sqlite3", std::process::id() as u128 * 1000 + rand_suffix()));
        let _ = std::fs::remove_file(&path);
        Db::open(&path).unwrap()
    }

    fn rand_suffix() -> u128 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos() as u128
    }

    fn item(kind: &str, payload: Value) -> InboxItem {
        InboxItem { id: 1, kind: kind.into(), payload }
    }

    #[test]
    fn phone_requests_reach_the_database() {
        let db = temp_db();
        let today = Local::now().date_naive();
        apply(&db, &item("add_event", json!({ "title": " 세션 ", "start_date": today, "start_time": "20:00", "end_time": "23:30" }))).unwrap();
        apply(&db, &item("add_event", json!({ "title": "여행", "start_date": today, "end_date": today + chrono::Duration::days(2), "all_day": true }))).unwrap();
        // End before start falls back to one hour.
        apply(&db, &item("add_event", json!({ "title": "짧게", "start_date": today, "start_time": "10:00", "end_time": "09:00" }))).unwrap();
        assert!(apply(&db, &item("add_event", json!({ "title": "  ", "start_date": today }))).is_err());

        let group = db.add_todo_group("공부", 0x4aa8ff).unwrap();
        apply(&db, &item("add_todo", json!({ "title": "컴활 기출", "group_id": group, "due": today, "due_time": "18:00" }))).unwrap();
        apply(&db, &item("add_todo", json!({ "title": "없는 그룹", "group_id": 9999 }))).unwrap();

        let snap = snapshot(&db).unwrap();
        let events = snap["events"].as_array().unwrap();
        assert!(events.iter().any(|e| e["title"] == "세션" && e["start"].as_str().unwrap().ends_with("20:00:00")));
        let short = events.iter().find(|e| e["title"] == "짧게").unwrap();
        assert!(short["end"].as_str().unwrap().ends_with("11:00:00"));
        let todos = snap["todos"].as_array().unwrap();
        let t = todos.iter().find(|t| t["title"] == "컴활 기출").unwrap();
        assert_eq!(t["group_id"], group);
        assert_eq!(t["due_time"], "18:00");
        assert!(todos.iter().find(|t| t["title"] == "없는 그룹").unwrap()["group_id"].is_null());

        let id = t["id"].as_i64().unwrap();
        apply(&db, &item("set_todo_done", json!({ "id": id, "done": true }))).unwrap();
        let snap = snapshot(&db).unwrap();
        assert_eq!(snap["todos"].as_array().unwrap().iter().find(|t| t["id"] == id).unwrap()["done"], true);
        assert!(apply(&db, &item("delete_everything", json!({}))).is_err());
    }
}
