// PROTOTYPE: virtual-file drag-out to Explorer (CFSTR_FILEDESCRIPTORW + CFSTR_FILECONTENTS).
use std::mem::ManuallyDrop;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use russh_sftp::client::SftpSession;
use serde::Serialize;
use tauri::Emitter;
use tokio::io::AsyncReadExt;
use windows::Win32::Foundation::*;
use windows::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL};
use windows::Win32::System::Com::Marshal::CoMarshalInterThreadInterfaceInStream;
use windows::Win32::System::Com::StructuredStorage::CoGetInterfaceAndReleaseStream;
use windows::Win32::System::Com::*;
use windows::Win32::System::DataExchange::RegisterClipboardFormatW;
use windows::Win32::System::Memory::*;
use windows::Win32::System::Ole::*;
use windows::Win32::System::SystemServices::{MK_LBUTTON, MODIFIERKEYS_FLAGS};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_ESCAPE, VK_LBUTTON};
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::{GUITHREADINFO, GetGUIThreadInfo, PostMessageW, WM_KEYDOWN, WM_KEYUP};
use windows_core::{BOOL, HRESULT, Interface, PWSTR, Ref, implement};

use crate::{SETTINGS, log};

pub struct Item {
    pub rel_path: String, // backslash-separated, relative to the drop folder
    pub remote_path: String,
    pub is_dir: bool,
    pub size: u64,
    pub mtime: Option<u32>,
}

static DRAG_SEQ: AtomicU32 = AtomicU32::new(0);
static STREAM_SEQ: AtomicU32 = AtomicU32::new(0);

pub fn init() {
    unsafe {
        // Keep the process MTA alive so streams marshalled from it stay reachable.
        let _ = CoIncrementMTAUsage();
        match OleInitialize(None) {
            Ok(()) => log("OleInitialize on main thread: ok"),
            Err(e) => log(format!("OleInitialize on main thread: {e}")),
        }
    }
}

fn filetime(unix: u32) -> FILETIME {
    let t = (unix as u64 + 11_644_473_600) * 10_000_000;
    FILETIME { dwLowDateTime: t as u32, dwHighDateTime: (t >> 32) as u32 }
}

fn cf(name: windows_core::PCWSTR) -> u16 {
    unsafe { RegisterClipboardFormatW(name) as u16 }
}

// ---------- the drag ----------

struct DragState {
    id: u32,
    done: AtomicBool,
    cancel: AtomicBool,
}

/// Runs on the main thread (from run_on_main_thread). Blocks in DoDragDrop's modal loop.
pub fn run(sftp: Arc<SftpSession>, items: Vec<Item>) {
    let id = DRAG_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
    let state = Arc::new(DragState { id, done: AtomicBool::new(false), cancel: AtomicBool::new(false) });
    let data: IDataObject = DataObject {
        drag: id,
        items: Arc::new(items),
        sftp,
        cf_descriptor: cf(CFSTR_FILEDESCRIPTORW),
        cf_contents: cf(CFSTR_FILECONTENTS),
        in_op: AtomicBool::new(false),
        stored: Mutex::new(vec![]),
        t0: Instant::now(),
    }
    .into();
    let source: IDropSource = DropSource { state: state.clone() }.into();

    let main_tid = crate::MAIN_TID.load(Ordering::Relaxed);
    let wd = state.clone();
    std::thread::spawn(move || watchdog(wd, main_tid));

    log(format!("drag {id}: DoDragDrop start"));
    let t0 = Instant::now();
    let mut effect = DROPEFFECT_NONE;
    let hr = unsafe { DoDragDrop(&data, &source, DROPEFFECT_COPY, &mut effect) };
    state.done.store(true, Ordering::Relaxed);
    let what = match hr {
        DRAGDROP_S_DROP => "DROP".to_string(),
        DRAGDROP_S_CANCEL => "CANCEL".to_string(),
        other => format!("{other:?}"),
    };
    log(format!(
        "drag {id}: DoDragDrop returned {what}, effect {}, after {:.0} ms",
        effect.0,
        t0.elapsed().as_secs_f64() * 1000.0
    ));
}

/// If the mouse button has been up for `watchdog_secs` while DoDragDrop is
/// still running, the drag is stuck: cancel it and post Escape to the
/// window that holds the mouse capture (the drag-rs#94 recovery).
fn watchdog(state: Arc<DragState>, main_tid: u32) {
    let mut up_since: Option<Instant> = None;
    let mut fired = 0;
    while !state.done.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(100));
        let down = unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) } as u16 & 0x8000 != 0;
        if down {
            up_since = None;
            continue;
        }
        let since = *up_since.get_or_insert_with(Instant::now);
        let limit = SETTINGS.watchdog_secs.load(Ordering::Relaxed);
        if limit == 0 || since.elapsed() < Duration::from_secs(limit as u64 + fired) {
            continue;
        }
        fired += 1;
        state.cancel.store(true, Ordering::Relaxed);
        let mut info = GUITHREADINFO { cbSize: size_of::<GUITHREADINFO>() as u32, ..Default::default() };
        let capture = unsafe { GetGUIThreadInfo(main_tid, &mut info) }.ok().map(|_| info.hwndCapture);
        if let Some(hwnd) = capture.filter(|h| !h.is_invalid()) {
            unsafe {
                let _ = PostMessageW(Some(hwnd), WM_KEYDOWN, WPARAM(VK_ESCAPE.0 as usize), LPARAM(0));
                let _ = PostMessageW(Some(hwnd), WM_KEYUP, WPARAM(VK_ESCAPE.0 as usize), LPARAM(0));
            }
        }
        log(format!(
            "drag {}: WATCHDOG fired (button up {:.1} s), set cancel flag, posted Escape to capture window {:?}",
            state.id,
            since.elapsed().as_secs_f64(),
            capture.map(|h| h.0)
        ));
    }
}

#[implement(IDropSource)]
struct DropSource {
    state: Arc<DragState>,
}

impl IDropSource_Impl for DropSource_Impl {
    fn QueryContinueDrag(&self, escape: BOOL, keys: MODIFIERKEYS_FLAGS) -> HRESULT {
        if self.state.cancel.load(Ordering::Relaxed) || escape.as_bool() {
            log(format!(
                "drag {}: QueryContinueDrag -> CANCEL (escape={}, watchdog={})",
                self.state.id,
                escape.as_bool(),
                self.state.cancel.load(Ordering::Relaxed)
            ));
            return DRAGDROP_S_CANCEL;
        }
        if keys.0 & MK_LBUTTON.0 == 0 {
            if SETTINGS.stuck_mode.load(Ordering::Relaxed) {
                return S_OK; // simulate a wedged drag: ignore the button release
            }
            return DRAGDROP_S_DROP;
        }
        S_OK
    }

    fn GiveFeedback(&self, _effect: DROPEFFECT) -> HRESULT {
        DRAGDROP_S_USEDEFAULTCURSORS
    }
}

// ---------- the data object ----------

#[implement(IDataObject, IDataObjectAsyncCapability)]
struct DataObject {
    drag: u32,
    items: Arc<Vec<Item>>,
    sftp: Arc<SftpSession>,
    cf_descriptor: u16,
    cf_contents: u16,
    in_op: AtomicBool,
    stored: Mutex<Vec<(u16, usize)>>, // formats Explorer hands us via SetData (HGLOBAL as usize)
    t0: Instant,
}

impl Drop for DataObject {
    fn drop(&mut self) {
        for (_, h) in self.stored.lock().unwrap().drain(..) {
            unsafe {
                let _ = GlobalFree(Some(HGLOBAL(h as *mut _)));
            }
        }
        log(format!("drag {}: data object released", self.drag));
    }
}

fn format_name(cf: u16) -> String {
    let mut buf = [0u16; 128];
    let n = unsafe { windows::Win32::System::DataExchange::GetClipboardFormatNameW(cf as u32, &mut buf) };
    if n > 0 { String::from_utf16_lossy(&buf[..n as usize]) } else { format!("cf {cf}") }
}

fn copy_hglobal(src: HGLOBAL) -> windows_core::Result<HGLOBAL> {
    unsafe {
        let size = GlobalSize(src);
        let dst = GlobalAlloc(GMEM_MOVEABLE, size)?;
        let (s, d) = (GlobalLock(src), GlobalLock(dst));
        std::ptr::copy_nonoverlapping(s as *const u8, d as *mut u8, size);
        let _ = GlobalUnlock(src);
        let _ = GlobalUnlock(dst);
        Ok(dst)
    }
}

impl DataObject_Impl {
    fn descriptor(&self) -> windows_core::Result<HGLOBAL> {
        let n = self.items.len();
        let size = size_of::<u32>() + n * size_of::<FILEDESCRIPTORW>();
        unsafe {
            let h = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, size)?;
            let group = GlobalLock(h) as *mut FILEGROUPDESCRIPTORW;
            (*group).cItems = n as u32;
            let fds = (&raw mut (*group).fgd) as *mut FILEDESCRIPTORW;
            // FILEDESCRIPTORW is packed: build each one by value, write it unaligned.
            for (i, item) in self.items.iter().enumerate() {
                let mut fd = FILEDESCRIPTORW::default();
                let name: Vec<u16> = item.rel_path.encode_utf16().collect();
                let len = name.len().min(259);
                let mut file_name = [0u16; 260];
                file_name[..len].copy_from_slice(&name[..len]);
                fd.cFileName = file_name;
                if name.len() > 259 {
                    log(format!("drag {}: name longer than 259 UTF-16 units, truncated: {}", self.drag, item.rel_path));
                }
                let mut flags = FD_ATTRIBUTES.0 | FD_UNICODE.0;
                if item.is_dir {
                    fd.dwFileAttributes = FILE_ATTRIBUTE_DIRECTORY.0;
                } else {
                    fd.dwFileAttributes = FILE_ATTRIBUTE_NORMAL.0;
                    flags |= FD_FILESIZE.0 | FD_PROGRESSUI.0;
                    fd.nFileSizeHigh = (item.size >> 32) as u32;
                    fd.nFileSizeLow = item.size as u32;
                }
                if let Some(m) = item.mtime {
                    flags |= FD_WRITESTIME.0;
                    fd.ftLastWriteTime = filetime(m);
                }
                fd.dwFlags = flags as u32;
                fds.add(i).write_unaligned(fd);
            }
            let _ = GlobalUnlock(h);
            Ok(h)
        }
    }

    fn contents(&self, index: usize) -> windows_core::Result<IStream> {
        let item = &self.items[index];
        let id = STREAM_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
        let stream = RemoteStream {
            id,
            name: item.rel_path.clone(),
            remote_path: item.remote_path.clone(),
            size: item.size,
            mtime: item.mtime,
            sftp: self.sftp.clone(),
            inner: Mutex::new(Inner::default()),
        };
        if SETTINGS.stream_on_main.load(Ordering::Relaxed) {
            log(format!("stream {id}: created on MAIN thread (Reads will run on the UI thread)"));
            return Ok(stream.into());
        }
        // Create and marshal the stream on an MTA thread, hand back a proxy.
        // Explorer's Read calls then land on RPC threads, never the UI thread.
        let (tx, rx) = std::sync::mpsc::channel::<usize>();
        std::thread::spawn(move || unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let s: IStream = stream.into();
            let marshalled = CoMarshalInterThreadInterfaceInStream(&IStream::IID, &s).unwrap();
            tx.send(marshalled.into_raw() as usize).unwrap();
            drop(s);
            CoUninitialize();
        });
        let raw = rx.recv().unwrap();
        unsafe {
            let marshalled = ManuallyDrop::new(IStream::from_raw(raw as *mut _));
            let proxy: IStream = CoGetInterfaceAndReleaseStream(&*marshalled)?;
            log(format!("stream {id}: created on its own MTA thread, proxy handed to Explorer"));
            Ok(proxy)
        }
    }
}

impl IDataObject_Impl for DataObject_Impl {
    fn GetData(&self, pformatetcin: *const FORMATETC) -> windows_core::Result<STGMEDIUM> {
        let f = unsafe { &*pformatetcin };
        let elapsed = self.t0.elapsed().as_secs_f64() * 1000.0;
        if f.cfFormat == self.cf_descriptor && f.tymed & TYMED_HGLOBAL.0 as u32 != 0 {
            log(format!("drag {}: GetData(FileGroupDescriptorW) at {elapsed:.0} ms", self.drag));
            let h = self.descriptor()?;
            return Ok(STGMEDIUM { tymed: TYMED_HGLOBAL.0 as u32, u: STGMEDIUM_0 { hGlobal: h }, pUnkForRelease: ManuallyDrop::new(None) });
        }
        if f.cfFormat == self.cf_contents && f.tymed & TYMED_ISTREAM.0 as u32 != 0 {
            let index = f.lindex.max(0) as usize;
            log(format!(
                "drag {}: GetData(FileContents, lindex {}) at {elapsed:.0} ms: {}",
                self.drag, f.lindex, self.items.get(index).map(|i| i.rel_path.as_str()).unwrap_or("?")
            ));
            if index >= self.items.len() || self.items[index].is_dir {
                return Err(DV_E_LINDEX.into());
            }
            let stream = self.contents(index)?;
            return Ok(STGMEDIUM { tymed: TYMED_ISTREAM.0 as u32, u: STGMEDIUM_0 { pstm: ManuallyDrop::new(Some(stream)) }, pUnkForRelease: ManuallyDrop::new(None) });
        }
        if let Some((_, h)) = self.stored.lock().unwrap().iter().find(|(c, _)| *c == f.cfFormat) {
            let copy = copy_hglobal(HGLOBAL(*h as *mut _))?;
            return Ok(STGMEDIUM { tymed: TYMED_HGLOBAL.0 as u32, u: STGMEDIUM_0 { hGlobal: copy }, pUnkForRelease: ManuallyDrop::new(None) });
        }
        Err(DV_E_FORMATETC.into())
    }

    fn GetDataHere(&self, _: *const FORMATETC, _: *mut STGMEDIUM) -> windows_core::Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn QueryGetData(&self, pformatetc: *const FORMATETC) -> HRESULT {
        let f = unsafe { &*pformatetc };
        let ok = (f.cfFormat == self.cf_descriptor && f.tymed & TYMED_HGLOBAL.0 as u32 != 0)
            || (f.cfFormat == self.cf_contents && f.tymed & TYMED_ISTREAM.0 as u32 != 0)
            || self.stored.lock().unwrap().iter().any(|(c, _)| *c == f.cfFormat);
        if ok { S_OK } else { DV_E_FORMATETC }
    }

    fn GetCanonicalFormatEtc(&self, _: *const FORMATETC, out: *mut FORMATETC) -> HRESULT {
        unsafe { (*out).ptd = std::ptr::null_mut() };
        E_NOTIMPL
    }

    fn SetData(&self, pformatetc: *const FORMATETC, pmedium: *const STGMEDIUM, release: BOOL) -> windows_core::Result<()> {
        let f = unsafe { &*pformatetc };
        let m = unsafe { &*pmedium };
        if m.tymed != TYMED_HGLOBAL.0 as u32 {
            return Err(E_NOTIMPL.into());
        }
        let copy = copy_hglobal(unsafe { m.u.hGlobal })?;
        if release.as_bool() {
            unsafe { ReleaseStgMedium(pmedium as *mut _) };
        }
        let name = format_name(f.cfFormat);
        if name.contains("Drop") || name.contains("Paste") {
            let value = unsafe {
                let p = GlobalLock(copy) as *const u32;
                let v = *p;
                let _ = GlobalUnlock(copy);
                v
            };
            log(format!("drag {}: SetData({name}) = {value}", self.drag));
        }
        let mut stored = self.stored.lock().unwrap();
        if let Some(pos) = stored.iter().position(|(c, _)| *c == f.cfFormat) {
            let (_, old) = stored.remove(pos);
            unsafe {
                let _ = GlobalFree(Some(HGLOBAL(old as *mut _)));
            }
        }
        stored.push((f.cfFormat, copy.0 as usize));
        Ok(())
    }

    fn EnumFormatEtc(&self, direction: u32) -> windows_core::Result<IEnumFORMATETC> {
        if direction != DATADIR_GET.0 as u32 {
            return Err(E_NOTIMPL.into());
        }
        let formats = [
            FORMATETC { cfFormat: self.cf_descriptor, ptd: std::ptr::null_mut(), dwAspect: DVASPECT_CONTENT.0, lindex: -1, tymed: TYMED_HGLOBAL.0 as u32 },
            FORMATETC { cfFormat: self.cf_contents, ptd: std::ptr::null_mut(), dwAspect: DVASPECT_CONTENT.0, lindex: -1, tymed: TYMED_ISTREAM.0 as u32 },
        ];
        unsafe { SHCreateStdEnumFmtEtc(&formats) }
    }

    fn DAdvise(&self, _: *const FORMATETC, _: u32, _: Ref<IAdviseSink>) -> windows_core::Result<u32> {
        Err(OLE_E_ADVISENOTSUPPORTED.into())
    }

    fn DUnadvise(&self, _: u32) -> windows_core::Result<()> {
        Err(OLE_E_ADVISENOTSUPPORTED.into())
    }

    fn EnumDAdvise(&self) -> windows_core::Result<IEnumSTATDATA> {
        Err(OLE_E_ADVISENOTSUPPORTED.into())
    }
}

impl IDataObjectAsyncCapability_Impl for DataObject_Impl {
    fn SetAsyncMode(&self, f: BOOL) -> windows_core::Result<()> {
        log(format!("drag {}: SetAsyncMode({})", self.drag, f.as_bool()));
        Ok(())
    }

    fn GetAsyncMode(&self) -> windows_core::Result<BOOL> {
        log(format!("drag {}: GetAsyncMode -> TRUE", self.drag));
        Ok(TRUE)
    }

    fn StartOperation(&self, _: Ref<IBindCtx>) -> windows_core::Result<()> {
        self.in_op.store(true, Ordering::Relaxed);
        log(format!("drag {}: StartOperation (Explorer extracts in the background)", self.drag));
        Ok(())
    }

    fn InOperation(&self) -> windows_core::Result<BOOL> {
        Ok(self.in_op.load(Ordering::Relaxed).into())
    }

    fn EndOperation(&self, hr: HRESULT, _: Ref<IBindCtx>, effects: u32) -> windows_core::Result<()> {
        self.in_op.store(false, Ordering::Relaxed);
        log(format!(
            "drag {}: EndOperation hr={hr:?} effects={effects} after {:.0} ms",
            self.drag,
            self.t0.elapsed().as_secs_f64() * 1000.0
        ));
        Ok(())
    }
}

// ---------- the stream ----------

#[derive(Serialize, Clone)]
struct StreamEvent {
    id: u32,
    name: String,
    bytes: u64,
    total: u64,
    state: &'static str,
    mbps: f64,
}

#[derive(Default)]
struct Inner {
    rx: Option<tokio::sync::mpsc::Receiver<Result<Vec<u8>, String>>>,
    buf: Vec<u8>,
    buf_pos: usize,
    pos: u64,
    eof: bool,
    failed: bool,
    started: Option<Instant>,
    last_emit: Option<Instant>,
    reads: u64,
}

#[implement(IStream)]
struct RemoteStream {
    id: u32,
    name: String,
    remote_path: String,
    size: u64,
    mtime: Option<u32>,
    sftp: Arc<SftpSession>,
    inner: Mutex<Inner>,
}

impl RemoteStream {
    fn emit(&self, inner: &Inner, state: &'static str) {
        let secs = inner.started.map(|s| s.elapsed().as_secs_f64()).unwrap_or(0.0);
        let mbps = if secs > 0.0 { inner.pos as f64 / 1e6 / secs } else { 0.0 };
        let _ = crate::app().emit(
            "stream",
            StreamEvent { id: self.id, name: self.name.clone(), bytes: inner.pos, total: self.size, state, mbps },
        );
    }

    /// Start the SFTP read lazily, on the first Read, not in GetData.
    fn start(&self) -> tokio::sync::mpsc::Receiver<Result<Vec<u8>, String>> {
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<Vec<u8>, String>>(8);
        let (sftp, path, id) = (self.sftp.clone(), self.remote_path.clone(), self.id);
        tauri::async_runtime::spawn(async move {
            let mut file = match sftp.open(path).await {
                Ok(f) => f,
                Err(e) => {
                    let _ = tx.send(Err(format!("open failed: {e}"))).await;
                    return;
                }
            };
            let t0 = Instant::now();
            let mut sent: u64 = 0;
            loop {
                let kbps = SETTINGS.throttle_kbps.load(Ordering::Relaxed) as u64;
                let chunk = if kbps == 0 { 256 * 1024 } else { (kbps * 1024 / 8).clamp(4096, 256 * 1024) as usize };
                let mut buf = vec![0u8; chunk];
                let n = match file.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(e) => {
                        let _ = tx.send(Err(format!("SFTP read failed: {e}"))).await;
                        return;
                    }
                };
                buf.truncate(n);
                sent += n as u64;
                let fail_after = SETTINGS.fail_after_mb.load(Ordering::Relaxed) as u64 * 1_000_000;
                if fail_after > 0 && sent >= fail_after {
                    let _ = tx.send(Err(format!("simulated failure after {sent} bytes"))).await;
                    return;
                }
                if tx.send(Ok(buf)).await.is_err() {
                    crate::log(format!("stream {id}: reader stopped, stream was released"));
                    return;
                }
                if kbps > 0 {
                    let due = Duration::from_secs_f64(sent as f64 / (kbps as f64 * 1024.0));
                    if let Some(wait) = due.checked_sub(t0.elapsed()) {
                        tokio::time::sleep(wait).await;
                    }
                }
            }
        });
        rx
    }
}

impl Drop for RemoteStream {
    fn drop(&mut self) {
        let inner = self.inner.lock().unwrap();
        let state = if inner.failed { "failed" } else if inner.eof { "done" } else if inner.started.is_none() { "unread" } else { "released early" };
        log(format!(
            "stream {}: released ({state}) at {} of {} bytes after {} Read calls",
            self.id, inner.pos, self.size, inner.reads
        ));
        self.emit(&inner, state);
    }
}

impl ISequentialStream_Impl for RemoteStream_Impl {
    fn Read(&self, pv: *mut core::ffi::c_void, cb: u32, pcbread: *mut u32) -> HRESULT {
        let mut inner = self.inner.lock().unwrap();
        inner.reads += 1;
        if inner.failed {
            return STG_E_READFAULT;
        }
        if inner.rx.is_none() && !inner.eof {
            log(format!("stream {}: first Read({cb}) on thread {}, starting SFTP read", self.id, crate::thread_label()));
            inner.started = Some(Instant::now());
            inner.rx = Some(self.start());
        }
        let out = unsafe { std::slice::from_raw_parts_mut(pv as *mut u8, cb as usize) };
        let mut copied = 0usize;
        while copied < out.len() {
            if inner.buf_pos < inner.buf.len() {
                let n = (inner.buf.len() - inner.buf_pos).min(out.len() - copied);
                out[copied..copied + n].copy_from_slice(&inner.buf[inner.buf_pos..inner.buf_pos + n]);
                inner.buf_pos += n;
                copied += n;
                continue;
            }
            if inner.eof {
                break;
            }
            match inner.rx.as_mut().unwrap().blocking_recv() {
                Some(Ok(chunk)) => {
                    inner.buf = chunk;
                    inner.buf_pos = 0;
                }
                Some(Err(e)) => {
                    log(format!("stream {}: {e} -> returning STG_E_READFAULT to Explorer", self.id));
                    inner.failed = true;
                    inner.pos += copied as u64;
                    self.emit(&inner, "failed");
                    unsafe { *pcbread = 0 };
                    return STG_E_READFAULT;
                }
                None => {
                    inner.eof = true;
                    log(format!(
                        "stream {}: EOF at {} bytes, {:.1} s",
                        self.id,
                        inner.pos + copied as u64,
                        inner.started.unwrap().elapsed().as_secs_f64()
                    ));
                }
            }
        }
        inner.pos += copied as u64;
        if !pcbread.is_null() {
            unsafe { *pcbread = copied as u32 };
        }
        let due = inner.last_emit.is_none_or(|t| t.elapsed() > Duration::from_millis(200));
        if due || inner.eof {
            inner.last_emit = Some(Instant::now());
            let state = if inner.eof { "done" } else { "reading" };
            self.emit(&inner, state);
        }
        if copied < cb as usize { S_FALSE } else { S_OK }
    }

    fn Write(&self, _: *const core::ffi::c_void, _: u32, _: *mut u32) -> HRESULT {
        STG_E_ACCESSDENIED
    }
}

impl IStream_Impl for RemoteStream_Impl {
    fn Seek(&self, mv: i64, origin: STREAM_SEEK, newpos: *mut u64) -> windows_core::Result<()> {
        let inner = self.inner.lock().unwrap();
        let ok = (origin == STREAM_SEEK_CUR && mv == 0) || (origin == STREAM_SEEK_SET && mv as u64 == inner.pos);
        if !ok {
            log(format!("stream {}: unsupported Seek({mv}, {}) at pos {}", self.id, origin.0, inner.pos));
            return Err(STG_E_INVALIDFUNCTION.into());
        }
        if !newpos.is_null() {
            unsafe { *newpos = inner.pos };
        }
        Ok(())
    }

    fn SetSize(&self, _: u64) -> windows_core::Result<()> {
        Err(STG_E_ACCESSDENIED.into())
    }

    fn CopyTo(&self, _: Ref<IStream>, _: u64, _: *mut u64, _: *mut u64) -> windows_core::Result<()> {
        log(format!("stream {}: CopyTo called (not implemented)", self.id));
        Err(E_NOTIMPL.into())
    }

    fn Commit(&self, _: &STGC) -> windows_core::Result<()> {
        Ok(())
    }

    fn Revert(&self) -> windows_core::Result<()> {
        Ok(())
    }

    fn LockRegion(&self, _: u64, _: u64, _: &LOCKTYPE) -> windows_core::Result<()> {
        Err(STG_E_INVALIDFUNCTION.into())
    }

    fn UnlockRegion(&self, _: u64, _: u64, _: u32) -> windows_core::Result<()> {
        Err(STG_E_INVALIDFUNCTION.into())
    }

    fn Stat(&self, pstatstg: *mut STATSTG, flag: &STATFLAG) -> windows_core::Result<()> {
        let mut st = STATSTG {
            r#type: STGTY_STREAM.0 as u32,
            cbSize: self.size,
            grfMode: STGM_READ,
            ..Default::default()
        };
        if let Some(m) = self.mtime {
            st.mtime = filetime(m);
        }
        if *flag != STATFLAG_NONAME {
            let leaf = self.name.rsplit('\\').next().unwrap_or(&self.name);
            let wide: Vec<u16> = leaf.encode_utf16().chain(Some(0)).collect();
            unsafe {
                let p = CoTaskMemAlloc(wide.len() * 2) as *mut u16;
                std::ptr::copy_nonoverlapping(wide.as_ptr(), p, wide.len());
                st.pwcsName = PWSTR(p);
            }
        }
        unsafe { *pstatstg = st };
        Ok(())
    }

    fn Clone(&self) -> windows_core::Result<IStream> {
        Err(E_NOTIMPL.into())
    }
}
