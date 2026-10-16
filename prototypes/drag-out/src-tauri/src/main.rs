// PROTOTYPE: drag-out test build. Throwaway code that answers one question:
// does dragging remote files out to Explorer through our own IDataObject work
// well enough for v1? No error handling beyond what keeps it runnable.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod dragout;

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use russh::client;
use russh::keys::{PrivateKeyWithHashAlg, PublicKeyOrCertificate, load_secret_key};
use russh_sftp::client::{RawSftpSession, SftpSession, error::Error as SftpError};
use russh_sftp::protocol::StatusCode;
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

// ---------- shared state, settings and logging ----------

pub struct Settings {
    pub throttle_kbps: AtomicU32,
    pub fail_after_mb: AtomicU32,
    pub stuck_mode: AtomicBool,
    pub watchdog_secs: AtomicU32,
    pub stream_on_main: AtomicBool,
}

pub static SETTINGS: Settings = Settings {
    throttle_kbps: AtomicU32::new(0),
    fail_after_mb: AtomicU32::new(0),
    stuck_mode: AtomicBool::new(false),
    watchdog_secs: AtomicU32::new(3),
    stream_on_main: AtomicBool::new(false),
};

static APP: OnceLock<AppHandle> = OnceLock::new();
static START: OnceLock<Instant> = OnceLock::new();
pub static MAIN_TID: AtomicU32 = AtomicU32::new(0);

pub fn app() -> &'static AppHandle {
    APP.get().unwrap()
}

pub fn thread_label() -> String {
    let tid = unsafe { windows::Win32::System::Threading::GetCurrentThreadId() };
    if tid == MAIN_TID.load(Ordering::Relaxed) { "main".into() } else { format!("t{tid}") }
}

#[derive(Serialize, Clone)]
struct LogLine {
    t: f64,
    thread: String,
    msg: String,
}

pub fn log(msg: impl Into<String>) {
    let msg = msg.into();
    let t = START.get().unwrap().elapsed().as_secs_f64() * 1000.0;
    let thread = thread_label();
    eprintln!("[{t:>9.1} {thread:>6}] {msg}");
    let _ = app().emit("log", LogLine { t, thread, msg });
}

struct Conn {
    handle: client::Handle<Client>,
    sftp: Arc<SftpSession>,
    raw: Arc<RawSftpSession>,
}

#[derive(Default)]
struct AppState {
    conn: Mutex<Option<Conn>>,
}

struct Client;

impl client::Handler for Client {
    type Error = anyhow::Error;

    async fn check_server_key(&mut self, _: &PublicKeyOrCertificate) -> Result<bool, Self::Error> {
        Ok(true) // prototype: trust any host key
    }
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

// ---------- commands ----------

#[tauri::command]
async fn connect(
    state: State<'_, AppState>,
    host: String,
    port: u16,
    user: String,
    key_path: String,
) -> Result<String, String> {
    let t0 = Instant::now();
    let config = client::Config {
        keepalive_interval: Some(std::time::Duration::from_secs(15)),
        ..Default::default()
    };
    let mut handle = client::connect(Arc::new(config), (host.as_str(), port), Client)
        .await
        .map_err(err)?;
    let key = load_secret_key(&key_path, None).map_err(err)?;
    let hash = handle.best_supported_rsa_hash().await.map_err(err)?.flatten();
    let auth = handle
        .authenticate_publickey(user, PrivateKeyWithHashAlg::new(Arc::new(key), hash))
        .await
        .map_err(err)?;
    if !auth.success() {
        return Err("public key rejected".into());
    }

    // Two SFTP channels on one connection: high-level for file reads,
    // raw for streaming directory listings batch by batch.
    let ch = handle.channel_open_session().await.map_err(err)?;
    ch.request_subsystem(true, "sftp").await.map_err(err)?;
    let sftp = SftpSession::new(ch.into_stream()).await.map_err(err)?;
    let ch2 = handle.channel_open_session().await.map_err(err)?;
    ch2.request_subsystem(true, "sftp").await.map_err(err)?;
    let raw = RawSftpSession::new(ch2.into_stream());
    raw.init().await.map_err(err)?;

    let home = sftp.canonicalize(".").await.map_err(err)?;
    log(format!("connected in {:.0} ms, home {home}", t0.elapsed().as_secs_f64() * 1000.0));
    *state.conn.lock().await = Some(Conn { handle, sftp: Arc::new(sftp), raw: Arc::new(raw) });
    Ok(home)
}

#[tauri::command]
async fn disconnect(state: State<'_, AppState>) -> Result<(), String> {
    if let Some(conn) = state.conn.lock().await.take() {
        let _ = conn.handle.disconnect(russh::Disconnect::ByApplication, "bye", "en").await;
        log("disconnected (any running drag-out reads should now fail)");
    }
    Ok(())
}

#[derive(Serialize, Clone)]
pub struct Entry {
    name: String,
    is_dir: bool,
    size: u64,
    mtime: u32,
}

#[derive(Serialize, Clone)]
#[serde(tag = "kind")]
enum ListEvent {
    Batch { entries: Vec<Entry> },
    Done { total: usize, ms_first: f64, ms_total: f64, readdir_calls: usize },
}

#[tauri::command]
async fn list_dir(
    state: State<'_, AppState>,
    path: String,
    on_event: Channel<ListEvent>,
) -> Result<(), String> {
    let raw = state.conn.lock().await.as_ref().ok_or("not connected")?.raw.clone();
    let t0 = Instant::now();
    let handle = raw.opendir(path.clone()).await.map_err(err)?.handle;
    let mut batch = Vec::with_capacity(1000);
    let (mut total, mut calls, mut ms_first) = (0usize, 0usize, None);
    loop {
        match raw.readdir(handle.as_str()).await {
            Ok(name) => {
                calls += 1;
                for f in name.files {
                    if f.filename == "." || f.filename == ".." {
                        continue;
                    }
                    batch.push(Entry {
                        is_dir: f.attrs.is_dir(),
                        size: f.attrs.size.unwrap_or(0),
                        mtime: f.attrs.mtime.unwrap_or(0),
                        name: f.filename,
                    });
                }
                if batch.len() >= 1000 {
                    total += batch.len();
                    ms_first.get_or_insert(t0.elapsed().as_secs_f64() * 1000.0);
                    let _ = on_event.send(ListEvent::Batch { entries: std::mem::take(&mut batch) });
                }
            }
            Err(SftpError::Status(s)) if s.status_code == StatusCode::Eof => break,
            Err(e) => return Err(err(e)),
        }
    }
    let _ = raw.close(handle).await;
    total += batch.len();
    ms_first.get_or_insert(t0.elapsed().as_secs_f64() * 1000.0);
    let _ = on_event.send(ListEvent::Batch { entries: batch });
    let ms_total = t0.elapsed().as_secs_f64() * 1000.0;
    log(format!("listed {path}: {total} entries, {calls} READDIR calls, {ms_total:.0} ms"));
    let _ = on_event.send(ListEvent::Done { total, ms_first: ms_first.unwrap(), ms_total, readdir_calls: calls });
    Ok(())
}

#[derive(Deserialize)]
struct SettingsIn {
    throttle_kbps: u32,
    fail_after_mb: u32,
    stuck_mode: bool,
    watchdog_secs: u32,
    stream_on_main: bool,
}

#[tauri::command]
fn set_settings(s: SettingsIn) {
    SETTINGS.throttle_kbps.store(s.throttle_kbps, Ordering::Relaxed);
    SETTINGS.fail_after_mb.store(s.fail_after_mb, Ordering::Relaxed);
    SETTINGS.stuck_mode.store(s.stuck_mode, Ordering::Relaxed);
    SETTINGS.watchdog_secs.store(s.watchdog_secs, Ordering::Relaxed);
    SETTINGS.stream_on_main.store(s.stream_on_main, Ordering::Relaxed);
}

/// Round trip to the main (UI/STA) thread, to see whether it's blocked.
#[tauri::command]
async fn main_ping(app: AppHandle) -> Result<f64, String> {
    let t0 = Instant::now();
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        let _ = tx.send(());
    })
    .map_err(err)?;
    rx.await.map_err(err)?;
    Ok(t0.elapsed().as_secs_f64() * 1000.0)
}

fn join(dir: &str, name: &str) -> String {
    if dir.ends_with('/') { format!("{dir}{name}") } else { format!("{dir}/{name}") }
}

/// List the whole tree under `remote` (pre-order) into drag items.
async fn walk(raw: &RawSftpSession, remote: String, rel: String, out: &mut Vec<dragout::Item>) -> Result<(), String> {
    let handle = raw.opendir(remote.clone()).await.map_err(err)?.handle;
    let mut children = vec![];
    loop {
        match raw.readdir(handle.as_str()).await {
            Ok(name) => children.extend(name.files.into_iter().filter(|f| f.filename != "." && f.filename != "..")),
            Err(SftpError::Status(s)) if s.status_code == StatusCode::Eof => break,
            Err(e) => return Err(err(e)),
        }
    }
    let _ = raw.close(handle).await;
    for f in children {
        let child_remote = join(&remote, &f.filename);
        let child_rel = format!("{rel}\\{}", f.filename);
        let is_dir = f.attrs.is_dir();
        out.push(dragout::Item {
            rel_path: child_rel.clone(),
            remote_path: child_remote.clone(),
            is_dir,
            size: f.attrs.size.unwrap_or(0),
            mtime: f.attrs.mtime,
        });
        if is_dir {
            Box::pin(walk(raw, child_remote, child_rel, out)).await?;
        }
    }
    Ok(())
}

#[tauri::command]
async fn start_drag(
    app: AppHandle,
    state: State<'_, AppState>,
    dir: String,
    names: Vec<String>,
) -> Result<(), String> {
    let t0 = Instant::now();
    log(format!("start_drag: {} selected", names.len()));
    let (sftp, raw) = {
        let guard = state.conn.lock().await;
        let c = guard.as_ref().ok_or("not connected")?;
        (c.sftp.clone(), c.raw.clone())
    };
    let mut items = vec![];
    for name in names {
        let remote = join(&dir, &name);
        let meta = sftp.metadata(remote.clone()).await.map_err(err)?;
        let is_dir = meta.is_dir();
        items.push(dragout::Item {
            rel_path: name.clone(),
            remote_path: remote.clone(),
            is_dir,
            size: meta.size.unwrap_or(0),
            mtime: meta.mtime,
        });
        if is_dir {
            walk(&raw, remote, name, &mut items).await?;
        }
    }
    let files = items.iter().filter(|i| !i.is_dir).count();
    let bytes: u64 = items.iter().map(|i| i.size).sum();
    log(format!(
        "pre-drag listing: {} entries ({} files, {} folders, {:.1} MB) in {:.0} ms",
        items.len(),
        files,
        items.len() - files,
        bytes as f64 / 1e6,
        t0.elapsed().as_secs_f64() * 1000.0
    ));
    app.run_on_main_thread(move || dragout::run(sftp, items)).map_err(err)?;
    Ok(())
}

fn main() {
    START.get_or_init(Instant::now);
    tauri::Builder::default()
        .manage(AppState::default())
        .setup(|app| {
            APP.set(app.handle().clone()).ok();
            MAIN_TID.store(unsafe { windows::Win32::System::Threading::GetCurrentThreadId() }, Ordering::Relaxed);
            dragout::init();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![connect, disconnect, list_dir, set_settings, main_ping, start_drag])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
