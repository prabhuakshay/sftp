// PROTOTYPE: drag-out test build UI. Throwaway.
import { useEffect, useMemo, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke, Channel } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useVirtualizer } from "@tanstack/react-virtual";
import "./styles.css";

const saved = (() => {
  try { return JSON.parse(localStorage.getItem("dragout") || "{}"); } catch { return {}; }
})();

const fmtSize = (n) => (n >= 1e9 ? (n / 1e9).toFixed(2) + " GB" : n >= 1e6 ? (n / 1e6).toFixed(1) + " MB" : n >= 1e3 ? (n / 1e3).toFixed(1) + " kB" : n + " B");
const fmtTime = (s) => (s ? new Date(s * 1000).toISOString().slice(0, 19).replace("T", " ") : "");
const parent = (p) => (p === "/" ? "/" : p.replace(/\/[^/]+\/?$/, "") || "/");
const join = (d, n) => (d.endsWith("/") ? d + n : d + "/" + n);

function App() {
  const [conn, setConn] = useState({ host: "127.0.0.1", port: 2222, user: "", keyPath: "", ...saved.conn });
  const [settings, setSettings] = useState({ throttle_kbps: 0, fail_after_mb: 0, stuck_mode: false, watchdog_secs: 3, stream_on_main: false, ...saved.settings });
  const [startMode, setStartMode] = useState(saved.startMode || "dragstart");
  const [connected, setConnected] = useState(false);
  const [path, setPath] = useState("");
  const [entries, setEntries] = useState([]);
  const [selected, setSelected] = useState(new Set());
  const [listStats, setListStats] = useState(null);
  const [logLines, setLogLines] = useState([]);
  const [streams, setStreams] = useState({});
  const [ping, setPing] = useState({ now: 0, max: 0 });
  const [frameGap, setFrameGap] = useState(0);

  useEffect(() => {
    try { localStorage.setItem("dragout", JSON.stringify({ conn, settings, startMode })); } catch {}
  }, [conn, settings, startMode]);

  useEffect(() => { invoke("set_settings", { s: settings }); }, [settings]);

  useEffect(() => {
    const un1 = listen("log", (e) => setLogLines((l) => [...l.slice(-400), e.payload]));
    const un2 = listen("stream", (e) => setStreams((s) => ({ ...s, [e.payload.id]: e.payload })));
    return () => { un1.then((f) => f()); un2.then((f) => f()); };
  }, []);

  // Responsiveness meters: main-thread round trip, and JS frame gaps.
  useEffect(() => {
    let alive = true, samples = [];
    const tick = async () => {
      while (alive) {
        const ms = await invoke("main_ping");
        const now = performance.now();
        samples = [...samples.filter((s) => now - s.t < 5000), { t: now, ms }];
        setPing({ now: ms, max: Math.max(...samples.map((s) => s.ms)) });
        await new Promise((r) => setTimeout(r, 100));
      }
    };
    tick();
    let last = performance.now(), gaps = [], raf;
    const frame = (t) => {
      gaps = [...gaps.filter((g) => t - g.t < 3000), { t, gap: t - last }];
      last = t;
      raf = requestAnimationFrame(frame);
    };
    raf = requestAnimationFrame(frame);
    const show = setInterval(() => setFrameGap(Math.max(...gaps.map((g) => g.gap))), 250);
    return () => { alive = false; cancelAnimationFrame(raf); clearInterval(show); };
  }, []);

  const doConnect = async () => {
    try {
      const home = await invoke("connect", { host: conn.host, port: Number(conn.port), user: conn.user, keyPath: conn.keyPath });
      setConnected(true);
      open(home + "/dragtest");
    } catch (e) { alert(e); }
  };

  const open = async (p) => {
    setPath(p);
    setSelected(new Set());
    setEntries([]);
    const all = [];
    const t0 = performance.now();
    let tFirstPaint = null;
    const ch = new Channel();
    await new Promise((resolve, reject) => {
      ch.onmessage = (m) => {
        if (m.kind === "Batch") {
          all.push(...m.entries);
          setEntries(all.slice());
          if (tFirstPaint === null) requestAnimationFrame(() => (tFirstPaint = performance.now() - t0));
        } else {
          const tArrived = performance.now() - t0;
          const ts = performance.now();
          const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: "base" });
          all.sort((a, b) => (b.is_dir - a.is_dir) || collator.compare(a.name, b.name));
          const sortMs = performance.now() - ts;
          setEntries(all.slice());
          requestAnimationFrame(() => {
            setListStats({ ...m, tArrived, sortMs, tFirstPaint, tRendered: performance.now() - t0 });
            resolve();
          });
        }
      };
      invoke("list_dir", { path: p, onEvent: ch }).catch(reject);
    }).catch((e) => alert(e));
  };

  const dragNames = (row) => (selected.has(row.name) ? [...selected] : [row.name]);
  const startDrag = (row) => invoke("start_drag", { dir: path, names: dragNames(row) }).catch((e) => alert(e));

  return (
    <div className="app">
      <div className="banner">PROTOTYPE: drag-out test build. Throwaway; see README.md.</div>
      <div className="bar">
        <input value={conn.host} onChange={(e) => setConn({ ...conn, host: e.target.value })} placeholder="host" size={12} />
        <input value={conn.port} onChange={(e) => setConn({ ...conn, port: e.target.value })} placeholder="port" size={5} />
        <input value={conn.user} onChange={(e) => setConn({ ...conn, user: e.target.value })} placeholder="user" size={10} />
        <input value={conn.keyPath} onChange={(e) => setConn({ ...conn, keyPath: e.target.value })} placeholder="private key path" size={50} />
        {connected ? <button onClick={() => invoke("disconnect").then(() => setConnected(false))}>Drop connection</button> : <button onClick={doConnect}>Connect</button>}
      </div>
      <div className="bar">
        <label>Throttle KB/s <input type="number" value={settings.throttle_kbps} onChange={(e) => setSettings({ ...settings, throttle_kbps: +e.target.value })} style={{ width: 70 }} /></label>
        <label>Fail after MB <input type="number" value={settings.fail_after_mb} onChange={(e) => setSettings({ ...settings, fail_after_mb: +e.target.value })} style={{ width: 60 }} /></label>
        <label><input type="checkbox" checked={settings.stuck_mode} onChange={(e) => setSettings({ ...settings, stuck_mode: e.target.checked })} /> Simulate stuck drag</label>
        <label>Watchdog s <input type="number" value={settings.watchdog_secs} onChange={(e) => setSettings({ ...settings, watchdog_secs: +e.target.value })} style={{ width: 45 }} /></label>
        <label>Stream lives on <select value={settings.stream_on_main ? "main" : "mta"} onChange={(e) => setSettings({ ...settings, stream_on_main: e.target.value === "main" })}>
          <option value="mta">its own MTA thread</option><option value="main">the main UI thread</option></select></label>
        <label>Drag starts from <select value={startMode} onChange={(e) => setStartMode(e.target.value)}>
          <option value="dragstart">HTML dragstart</option><option value="pointer">pointer events</option></select></label>
      </div>
      <div className="bar meters">
        <span className="spinner" />
        <span className={ping.max > 100 ? "bad" : ""}>Main thread round trip: {ping.now.toFixed(1)} ms (max 5 s: {ping.max.toFixed(0)} ms)</span>
        <span className={frameGap > 100 ? "bad" : ""}>Longest JS frame gap (3 s): {frameGap.toFixed(0)} ms</span>
        {listStats && <span>Listing {listStats.total.toLocaleString()} entries: first batch {listStats.ms_first.toFixed(0)} ms, all received {listStats.tArrived.toFixed(0)} ms ({listStats.readdir_calls} READDIRs, Rust {listStats.ms_total.toFixed(0)} ms), sort {listStats.sortMs.toFixed(0)} ms, rendered {listStats.tRendered.toFixed(0)} ms</span>}
      </div>
      <div className="main">
        <div className="files">
          <div className="pathbar">
            <button disabled={!connected} onClick={() => open(parent(path))}>Up</button>
            <span>{path}</span>
            <span className="hint">{selected.size} selected. Ctrl+click to multi-select, double-click a folder to open, drag rows out to Explorer.</span>
          </div>
          <FileList entries={entries} selected={selected} setSelected={setSelected} startMode={startMode} onOpen={(e) => e.is_dir && open(join(path, e.name))} onDrag={startDrag} />
        </div>
        <div className="side">
          <h3>Streams</h3>
          <div className="streams">
            {Object.values(streams).reverse().map((s) => (
              <div key={s.id} className={"stream " + s.state}>
                <div>#{s.id} {s.name} <b>{s.state}</b></div>
                <progress max={s.total || 1} value={s.bytes} /> {fmtSize(s.bytes)} / {fmtSize(s.total)} at {s.mbps.toFixed(1)} MB/s
              </div>
            ))}
          </div>
          <h3>Log <button onClick={() => setLogLines([])}>clear</button> <button onClick={() => navigator.clipboard.writeText(logLines.map((l) => `${l.t.toFixed(0)}\t${l.thread}\t${l.msg}`).join("\n"))}>copy</button></h3>
          <div className="log">
            {logLines.slice().reverse().map((l, i) => (
              <div key={i}><span className="t">{l.t.toFixed(0)}</span> <span className="th">{l.thread}</span> {l.msg}</div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

function FileList({ entries, selected, setSelected, startMode, onOpen, onDrag }) {
  const ref = useRef(null);
  const v = useVirtualizer({ count: entries.length, getScrollElement: () => ref.current, estimateSize: () => 24, overscan: 20 });
  const down = useRef(null);

  const click = (e, row) => {
    const next = new Set(e.ctrlKey ? selected : []);
    if (e.ctrlKey && selected.has(row.name)) next.delete(row.name); else next.add(row.name);
    setSelected(next);
  };

  return (
    <div className="list" ref={ref}>
      <div style={{ height: v.getTotalSize(), position: "relative" }}>
        {v.getVirtualItems().map((vi) => {
          const row = entries[vi.index];
          const handlers = startMode === "dragstart"
            ? { draggable: true, onDragStart: (e) => { e.preventDefault(); onDrag(row); } }
            : {
                onPointerDown: (e) => { if (e.button === 0) down.current = { x: e.clientX, y: e.clientY, fired: false }; },
                onPointerMove: (e) => {
                  const d = down.current;
                  if (d && !d.fired && e.buttons === 1 && Math.hypot(e.clientX - d.x, e.clientY - d.y) > 6) { d.fired = true; onDrag(row); }
                },
                onPointerUp: () => (down.current = null),
              };
          return (
            <div key={vi.key} className={"row" + (selected.has(row.name) ? " sel" : "")}
              style={{ position: "absolute", top: 0, left: 0, right: 0, height: vi.size, transform: `translateY(${vi.start}px)` }}
              onClick={(e) => click(e, row)} onDoubleClick={() => onOpen(row)} {...handlers}>
              <span className="name">{row.is_dir ? "📁 " : "📄 "}{row.name}</span>
              <span className="size">{row.is_dir ? "" : fmtSize(row.size)}</span>
              <span className="mtime">{fmtTime(row.mtime)}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

createRoot(document.getElementById("root")).render(<App />);
