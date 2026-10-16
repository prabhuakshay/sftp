# Tauri 2 on Windows: capability check for v1

This answers [#5](https://github.com/prabhuakshay/sftp/issues/5): what Tauri 2 can do on Windows for the v1 features, and how. Compiled 2026-10-16 from the Tauri 2 docs, plugin source and changelogs, docs.rs and crates.io, Microsoft Learn, and GitHub issues. Each claim cites its source inline.

**Versions as of 2026-10-16:** `tauri` 2.12.4, `@tauri-apps/api` 2.12.4, tauri-cli 2.12.3. Tauri 3.0.0-alpha.5 came out on 2026-10-14, so pin everything to `2.x`.

## Summary

| Capability | Verdict | Plugin / crate | Key catch |
|---|---|---|---|
| Drag remote files **out** to Explorer | **Works with a catch** | None off the shelf. Hand-written `IDataObject` with `CFSTR_FILEDESCRIPTOR`/`CFSTR_FILECONTENTS` + `IDataObjectAsyncCapability` via the `windows` crate. `tauri-plugin-drag` 2.1.1 is only good for the fallback (local files). | About 500 lines of COM code. Explorer owns the copy, so the app's queue, conflict rules and resume don't apply. The stream must not block the UI thread. A stuck `DoDragDrop` can wedge the app. |
| Drag files **in** from Explorer | **Works with a catch** | Tauri core `onDragDropEvent` | With the native handler on (needed to get paths), HTML5 drag-and-drop inside the webview is dead on Windows. Build in-app drag with pointer events (e.g. dnd-kit). |
| Windows notifications | **Works with a catch** | `tauri-plugin-notification` 2.7.0 | Proper name and icon only in the installed app. `tauri dev` shows it as PowerShell. Clicks are reported only while the toast is on screen. |
| Credential Manager | **Works** | `keyring` 4.2.0 (`windows-native-keyring-store`) | The blob is capped at 2560 bytes, so store passphrases and passwords, not big keys. v4's API differs from v3. |
| Windows Terminal + `ssh` | **Works with a catch** | `std::process::Command` (no plugin) | `wt` splits on `;` even inside arguments. Quote the remote `cd` carefully. Fall back to a console window if `wt` is missing. |
| Open in VS Code / default app, watch for saves | **Works with a catch** | `tauri-plugin-opener` 2.7.2, `notify` 8.2 + `notify-debouncer-full` 0.7 | VS Code's CLI is `code.cmd`, not `code.exe`. Editors save by rename, so watch the folder and debounce. |
| 100k-row list | **Works** | `@tanstack/react-virtual` 3.x; `tauri::ipc::Channel` | Stream entries in batches. Don't return one 100k-element JSON array. |
| Installer + auto-update | **Works with a catch** | Bundler (NSIS / WiX MSI), `tauri-plugin-updater` 2.14.0 | Updater signing is mandatory and the key can't be lost. MSI builds need Windows. SmartScreen warns until the signed app builds reputation. |

## 1. Dragging remote files out to Explorer

**Verdict: works with a catch.** You can start a native OLE drag of files that exist only on the server, from a Tauri 2 window on Windows. No published crate or plugin does it, though. The app has to implement the COM objects itself, in about 500 lines of `windows`-crate code. At least one Tauri SFTP client already ships this and reports it working against a live server.

### How Windows does it

- The drag source calls `DoDragDrop`. It needs `OleInitialize` on the calling thread and runs a modal loop until the drop or cancel ([DoDragDrop](https://learn.microsoft.com/en-us/windows/win32/api/ole2/nf-ole2-dodragdrop)).
- Microsoft's guidance for sources: "Existing files should be offered with the CF_HDROP format." "Offer file-like data with CFSTR_FILECONTENTS/CFSTR_FILEDESCRIPTOR formats … You should normally present the data as an IStream" ([Handling Shell Data Transfer Scenarios](https://learn.microsoft.com/en-us/windows/win32/shell/datascenarios)). This is exactly how FTP folders and other namespace extensions hand files to Explorer (same page).
- `CFSTR_FILEDESCRIPTOR` holds a `FILEGROUPDESCRIPTOR`: one `FILEDESCRIPTORW` per file, with name, size and attributes. The target then asks for each file's `CFSTR_FILECONTENTS` by setting `lindex` to its index ([Shell Clipboard Formats](https://learn.microsoft.com/en-us/windows/win32/shell/clipboard)). `FILEDESCRIPTORW` has `FD_FILESIZE`, `FD_WRITESTIME` and `FD_PROGRESSUI` ("a progress indicator is shown") flags, and `cFileName` is `WCHAR[MAX_PATH]` ([FILEDESCRIPTORW](https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/ns-shlobj_core-filedescriptorw)).
- A plain drag is synchronous: "The drop source calls DoDragDrop and blocks its primary thread until the function returns … the target extracts the data … on its primary thread." If the data object also implements `IDataObjectAsyncCapability`, the target "can extract data from the data object asynchronously on a background thread", and `DoDragDrop` returns straight away. The source learns that extraction finished through `EndOperation` ([same page, "Dragging and Dropping Shell Objects Asynchronously"](https://learn.microsoft.com/en-us/windows/win32/shell/datascenarios)).
- The source never learns where the files were dropped: "a drop source has no way of knowing which folder its data object has been dropped on" ([same page, Recycle Bin section](https://learn.microsoft.com/en-us/windows/win32/shell/datascenarios)). Raymond Chen describes the IStream virtual file as the answer to a customer who asked "We need to know what directory the user dropped a file onto" ([What a drag: Dragging a virtual file (IStream edition)](https://devblogs.microsoft.com/oldnewthing/20080319-00/?p=23073)).

So the plan is: the drag offers names and sizes only. Explorer opens a stream per file after the drop, the stream pulls bytes over SFTP, and Explorer writes them wherever the user let go.

### What existing crates and plugins do

| Option | Remote (virtual) files on Windows? | Notes |
|---|---|---|
| [`tauri-plugin-drag`](https://github.com/crabnebula-dev/drag-rs) 2.1.1 / [`drag`](https://crates.io/crates/drag) 2.1.1 (CrabNebula, May 2026) | **No** | `DragItem::Files` only takes local paths. The source says `DragItem::Data` is "**Windows**: Not supported. Will result in a dummy drag operation of current folder that will be cancelled upon dropping" ([lib.rs](https://github.com/crabnebula-dev/drag-rs/blob/v2/crates/drag/src/lib.rs)). The Windows `IDataObject` serves only `CF_HDROP`, and `EnumFormatEtc` returns `E_NOTIMPL` ([windows/mod.rs](https://github.com/crabnebula-dev/drag-rs/blob/v2/crates/drag/src/platform_impl/windows/mod.rs)). |
| Tauri core | **No** | The feature request [tauri#2593](https://github.com/tauri-apps/tauri/issues/2593) (2021) is still open. CrabNebula's drag-rs is the answer given there. |
| [`super_native_extensions`](https://github.com/superlistapp/super_native_extensions) (Rust core of Flutter's `super_drag_and_drop`) | Yes, but tied to Flutter | It implements Windows virtual files fully and is the best reference implementation. It hosts each virtual-file `IStream` on a background thread and hands out a marshalled proxy ([virtual_file_stream.rs](https://github.com/superlistapp/super_native_extensions/blob/main/super_native_extensions/rust/src/win32/virtual_file_stream.rs)). Its changelog has "delay virtual file request until IStream is read", and the code notes that VS Code "seems to periodically request the stream just to close it immediately." |
| [Reach](https://github.com/alexandrosnt/Reach) (Tauri 2 SSH/SFTP client), [`src-tauri/src/dragout/windows.rs`](https://github.com/alexandrosnt/Reach/blob/main/src-tauri/src/dragout/windows.rs) | Yes, hand-written | `IDataObject` + `IDataObjectAsyncCapability` + `IEnumFORMATETC` + a channel-fed `IStream`, started with `run_on_main_thread` from a Tauri command. Commit [11b40def](https://github.com/alexandrosnt/Reach/commit/11b40def) (2026-09-23) says: "Verified against a live server: a real file dragged into a folder arrived with its actual contents, and nothing was transferred until the drop." It also notes that Explorer enumerates formats during a drag, so an `EnumFormatEtc` that returns `E_NOTIMPL` (as in `drag`) means "a promise would never even be noticed". |

GitHub code search also finds several other Tauri apps (FTPeach, duskterm, OpenXTerm) with their own `CFSTR_FILECONTENTS` drag code. That is more evidence that the approach works from a WebView2 window, and that nobody has packaged it as a crate.

### How it would plug into the app

1. In the file list, the frontend handles the HTML `dragstart` on a row, calls `event.preventDefault()`, and invokes a Rust command with the selected entries (name, size, mtime). The `drag-rs` examples do the same with `dragDropEnabled: true` (Tauri's default), so `dragstart` still fires. Without `preventDefault()` the drag doesn't start ([drag-rs#42](https://github.com/crabnebula-dev/drag-rs/issues/42)).
2. The command calls `app.run_on_main_thread(...)`, builds the data object and calls `DoDragDrop`.
3. On drop, Explorer calls `GetData(CFSTR_FILECONTENTS, lindex)` and reads the `IStream`. The SFTP read starts on the first `Read`, not in `GetData`.

### Catches (all must be handled in the prototype)

- **`IStream::Read` must not block the UI thread.** Explorer is another process, so its calls into our COM objects are dispatched to the apartment that owns them. If the stream lives on the main STA thread, every `Read` runs there. Reach's `Read` does a blocking `rx.recv()` there, so a slow server would stall the window between chunks. `super_native_extensions` avoids this by creating the stream on a separate COM thread and returning a proxy made with `CoMarshalInterThreadInterfaceInStream`. Do the same. (This is inferred from COM's apartment rules and those two codebases. Measure it in the prototype.)
- **Implement `IDataObjectAsyncCapability`.** Without it, Explorer extracts inside `DoDragDrop`, so `DoDragDrop` and the main thread are held for the whole transfer ([datascenarios](https://learn.microsoft.com/en-us/windows/win32/shell/datascenarios)).
- **A `DoDragDrop` that never returns can wedge the whole app.** Reported against `drag` 2.1.1 on Windows 11 25H2 ([drag-rs#94](https://github.com/crabnebula-dev/drag-rs/issues/94)): mouse capture is stuck on OLE's `CLIPBRDWNDCLASS` window and the main thread spins. Posting `VK_ESCAPE` to the capture window recovers it, and the reporter uses a 5-second watchdog. There is an open PR for a timeout option ([drag-rs#95](https://github.com/crabnebula-dev/drag-rs/pull/95)). Our own implementation should ship the same watchdog.
- **The descriptor must be complete when the drag starts.** Explorer reads `CFSTR_FILEDESCRIPTOR` during the drag, so every name and size must be known up front. For folders, the whole remote tree has to be listed before `DoDragDrop`. Folder hierarchies are usually expressed as relative `cFileName`s (`dir\sub\file`) plus `FILE_ATTRIBUTE_DIRECTORY` entries. **Microsoft's reference pages don't document that convention, so verify it in the prototype**, along with the 260-character `cFileName` limit for deep trees.
- **Explorer, not the app, owns the copy.** Explorer shows its own progress dialog (`FD_PROGRESSUI`) and its own overwrite prompt. The app's transfer queue, conflict policy (overwrite/skip/rename/newer-only), resume and "temporary name then rename" don't apply to drag-out. The app can still count bytes served by `Read` to show progress in its queue. Whether Explorer applies `FD_WRITESTIME` (needed for "preserve modified times") is **unverified**, so test it.
- **Error mid-stream:** a failed SFTP read becomes an error `HRESULT` from `Read`. How Explorer reports it, and whether it leaves a partial file, needs testing.
- **Spurious stream requests:** some targets call `GetData(CFSTR_FILECONTENTS)` and release the stream without reading it, as VS Code does according to the `super_native_extensions` comment. Start the SFTP read lazily and cancel it when the stream is released.
- **Dropping back onto our own window:** Tauri's own drop handler reads paths only, so a virtual-file drop on the app produces no usable paths. The frontend should ignore it, or detect an internal drag and handle it as a remote move.
- **Bonus:** the same `IDataObject` passed to `OleSetClipboard` gives "Copy in app → Paste in Explorer" for free.

### Fallback: download to temp, then drag a real file

`tauri-plugin-drag` handles this well for files that are **already local**: `startDrag({ item: [paths], icon })` builds a `CF_HDROP`/shell-item data object. The catch is timing. `drag::start_drag` canonicalises each path and calls `ILCreateFromPathW`, so the files must exist **before** the drag starts. The download can't wait until the drop. That leaves two options:

- Download on mouse-down and start the drag when it finishes. This is fine for small files and unusable for large ones. Reach shipped exactly this first and then replaced it: "it downloaded files nobody ended up dragging, and it could not work at all for large ones without a visible wait" ([commit 11b40def](https://github.com/alexandrosnt/Reach/commit/11b40def)).
- Use a hand-written data object that serves `CF_HDROP` for temp paths and downloads at drop time. WinSCP's "download files via temporary folder" mode works this way. WinSCP documents its cost: "You cannot use background transfers in temporary folder download mode" ([WinSCP: Drag&Drop preferences](https://winscp.net/eng/docs/ui_pref_dragdrop)). Its other mode needs a shell extension that misuses Explorer's CopyHook to learn the drop folder ([WinSCP: shell extension](https://winscp.net/eng/docs/dragext)). That is a separate install with its own drawbacks, and it is out of scope here.

Known `drag` 2.1.1 Windows bugs if the plugin is used for the fallback:

- It panics on paths longer than 260 characters ([#76](https://github.com/crabnebula-dev/drag-rs/issues/76)).
- It panics on UNC or SMB-mapped paths ([#72](https://github.com/crabnebula-dev/drag-rs/issues/72); fix in open PR [#101](https://github.com/crabnebula-dev/drag-rs/pull/101)).
- It has the hang in #94.

Short `%TEMP%` paths avoid the first two.

**Recommendation:** build the virtual-file drag (`CFSTR_FILEDESCRIPTOR`/`CFSTR_FILECONTENTS`, async-capable, stream on its own COM thread) as the v1 path for files. Make it the first thing the prototype proves. Keep "download, then drag a real file" via `tauri-plugin-drag` as the fallback if the prototype fails. Also consider it for folder drags if tree listing turns out too slow. Either way, drag-out bypasses the app's transfer queue and conflict rules, and the v1 spec should say so.

## 2. Dragging files in from Explorer

**Verdict: works with a catch.**

- **API:** `getCurrentWebview().onDragDropEvent()` in JS, or `DragDropEvent` in Rust.
  - The variants are `Enter{paths, position}`, `Over{position}`, `Drop{paths, position}` and `Leave`.
  - `position` is "physical pixels, relative to the webview's top-left corner" ([docs.rs DragDropEvent](https://docs.rs/tauri/latest/tauri/enum.DragDropEvent.html), [JS webview API](https://v2.tauri.app/reference/javascript/api/namespacewebview/)).
  - So the frontend can find the folder row under the drop: divide by `devicePixelRatio`, then call `document.elementFromPoint()`.
  - The position may be off while the devtools panel is docked open (same docs).
- **Folders:** `paths` are plain paths, so a dropped folder arrives as its directory path. The docs don't say this outright. Check `is_dir()` and walk folders in Rust.
- **The catch:** `dragDropEnabled` defaults to `true`, and its config doc says: "Disabling it is required to use HTML5 drag and drop on the frontend on Windows since we replace the drag drop handler of WebView2" ([config reference](https://v2.tauri.app/reference/config/)).
  - This is still true. A maintainer calls it a Windows technical limitation, and the redesign that would fix it is planned as a breaking change ([tauri#14373](https://github.com/tauri-apps/tauri/issues/14373), open).
  - The setting can't be toggled at runtime ([tauri#13189](https://github.com/tauri-apps/tauri/issues/13189), open). It is fixed when the window is created.
- **Design consequence:** keep the native handler on, because it's the only way to get absolute paths for uploads. Build every in-app drag interaction with pointer events, not HTML5 `draggable`: rows onto folders, moving files, reordering Tabs. A pointer-based library such as dnd-kit does this.
  - Exception: `dragstart` still fires, which is what the drag-out code hooks.
  - Also, drag-out drops onto our own window arrive here with no usable paths (see §1).

## 3. Windows notifications

**Verdict: works with a catch.** Use `tauri-plugin-notification` 2.7.0.

- **Installed builds only.** The plugin docs say: "Only works for installed apps. Shows powershell name & icon in development" ([plugin docs](https://v2.tauri.app/plugin/notification/)).
  - The plugin sets the AppUserModelID to the bundle `identifier` only when the exe is not under `target\debug` or `target\release` ([desktop.rs](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/notification/src/desktop.rs)).
  - To test branding, install the NSIS/MSI build.
- **Clicks:** since 2.6.0, clicks and action buttons reach `onAction` and Rust `Notification::on_action` on desktop ([CHANGELOG](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/notification/CHANGELOG.md)). The docs page still says actions are mobile-only, but it's out of date.
  - On Windows a click is reported only while the toast is on screen, not after it moves to Action Center. Close events aren't reported (desktop.rs).
  - So "click the toast to open the transfer list" works if the user clicks it soon. Don't depend on it.

## 4. Secrets in Windows Credential Manager

**Verdict: works.** Use `keyring` 4.2.0 (2026-08-29).

- **How v4 is built:** v4 is a thin wrapper over `keyring-core` 1.0 plus per-platform stores.
  - The default `v1` feature pulls in `windows-native-keyring-store` 1.1 and picks Credential Manager automatically ([features](https://docs.rs/crate/keyring/latest/features), [keyring::v1](https://docs.rs/keyring/latest/keyring/v1/index.html)).
  - v3's `windows-native` feature no longer exists, so older examples won't compile as written.
  - For explicit control, use `keyring-core` + `windows-native-keyring-store` directly ([store docs](https://docs.rs/windows-native-keyring-store/latest/windows_native_keyring_store/)).
- **Size limit:** a credential blob is at most `CRED_MAX_CREDENTIAL_BLOB_SIZE` = 2560 bytes ([CREDENTIALW](https://learn.microsoft.com/en-us/windows/win32/api/wincred/ns-wincred-credentialw)).
  - Passwords are stored as UTF-16, so they max out around 1280 characters. `set_secret` gets the full 2560 bytes ([utils.rs](https://github.com/open-source-cooperative/windows-native-keyring-store/blob/main/src/utils.rs)).
  - That's plenty for passwords and key passphrases, which is all v1 stores. Keys stay as files.
- **Target name:** the default is `<user>.<service>`. Use a fixed service prefix (for example the app identifier) plus the Host alias, so entries are easy to find and remove.
- **Roaming:** the default persistence is Enterprise, which roams with roaming profiles. That's harmless for a single-user PC.
- `tauri-plugin-stronghold` is **not** this. It's an app-managed encrypted vault file, and v1 ruled out a master password.

## 5. Launching Windows Terminal with `ssh`

**Verdict: works with a catch.** No plugin is needed. Spawn from Rust with `std::process::Command`. `tauri-plugin-shell` 2.5.0 is only for spawning from JS, and needs scoped capabilities ([shell plugin](https://v2.tauri.app/plugin/shell/)).

- **Spawning `wt`:** `wt.exe` is an App Execution Alias in `%LOCALAPPDATA%\Microsoft\WindowsApps` ([wt command line](https://learn.microsoft.com/en-us/windows/terminal/command-line-arguments)). A local probe confirmed that `Command::new` can spawn execution aliases.
- **Command shape:** `wt -w 0 new-tab --title <alias> -- ssh -t <alias> "cd '<path>' && exec $SHELL -l"`, with each piece passed as a separate `.arg()`.
  - No local shell is involved, so `$SHELL` reaches the server unexpanded.
  - `-d` sets a *local* start folder, so it can't land in the remote folder.
- **Catches:**
  - `wt` treats `;` as its own command separator even inside arguments, and `--` reportedly doesn't stop it ([microsoft/terminal#13264](https://github.com/microsoft/terminal/issues/13264), open). Use `&&`, never `;`, and escape `;` in paths as `\;`.
  - In the remote path, escape single quotes in the POSIX way: close the quote, add an escaped quote, reopen.
  - `--` before the command line is used in practice ([terminal#18933](https://github.com/microsoft/terminal/issues/18933)) but isn't on the Learn page.
- **Fallback:** if spawning `wt` fails with `NotFound`, run `ssh.exe` in a new console (`CREATE_NEW_CONSOLE`, or `cmd /c start "" ssh -t ...`).
- **Interaction with the Login design:** the terminal runs Windows OpenSSH, not the app's own SSH session. It authenticates on its own using `~/.ssh/config`. The user's ssh-agent is disabled, so it will prompt for the key passphrase again. That's acceptable for v1, but say so in the spec.

## 6. Opening a file in VS Code or the default app, and watching for saves

**Verdict: works with a catch.**

- **Open:** use `tauri-plugin-opener` 2.7.2. In Rust, `app.opener().open_path(path, None)` opens the default app, and `Some("program")` opens a named one. In JS, use `openPath(path, with?)`, which needs an `opener:allow-open-path` scope ([opener plugin](https://v2.tauri.app/plugin/opener/), [docs.rs](https://docs.rs/tauri-plugin-opener/latest/tauri_plugin_opener/struct.Opener.html)).
- **VS Code:** the CLI is `bin\code.cmd`.
  - `std::process::Command` only fills in `.exe` ([std docs](https://doc.rust-lang.org/std/process/struct.Command.html)). A local probe confirmed that `Command::new("code")` fails with NotFound and `Command::new("code.cmd")` works.
  - Useful flags: `-r` (reuse window) and `-g file:line` ([VS Code CLI](https://code.visualstudio.com/docs/configure/command-line)).
  - `.cmd` files have odd argument parsing, so never pass unsanitised text through.
- **Watch:** use `notify` 8.2.0, whose Windows backend is ReadDirectoryChangesW, plus `notify-debouncer-full` 0.7.0 ([notify](https://github.com/notify-rs/notify)).
  - Editors may truncate the file or "create a new one and replace the old one" ([notify docs](https://docs.rs/notify/latest/notify/)). So watch the temp file's **parent folder**, non-recursively, and filter by name.
  - The full debouncer merges rename pairs and de-duplicates events ([debouncer-full](https://docs.rs/notify-debouncer-full/latest/notify_debouncer_full/)).
  - Still compare size and mtime, or a hash, before uploading. One save can produce several events.

## 7. Rendering 100k rows in WebView2

**Verdict: works.**

- **Virtualize:** `@tanstack/react-virtual` 3.14 is headless and fits a shadcn/Tailwind table. `react-window` 2.x also works.
- **Height limit:** Blink's layout unit caps element height around 33.5M px ([layout_unit.h](https://chromium.googlesource.com/chromium/src/+/HEAD/third_party/blink/renderer/platform/geometry/layout_unit.h)). The practical limit is reported nearer 16.7M px. 100k rows × 28 px = 2.8M px, so there's plenty of headroom. Scaling tricks only matter past about 600k rows.
- **IPC:** Tauri 2 IPC uses custom protocols and raw payloads ([Tauri 2.0 blog](https://v2.tauri.app/blog/tauri-20/)), but command return values are still JSON.
  - For a big listing, stream batches of 1–5k entries over `tauri::ipc::Channel<T>`, which is "designed to be fast and deliver ordered data".
  - Don't use events, which are "not designed for low latency or high throughput" ([calling the frontend](https://v2.tauri.app/develop/calling-frontend/)).
  - For raw bytes, return `tauri::ipc::Response` ([calling Rust](https://v2.tauri.app/develop/calling-rust/)).
  - Keep rows compact. One 100k-entry JSON array is an estimated 10–20 MB and stalls the UI during `JSON.parse`. That figure is an estimate, not measured.
- **Fits the planned architecture:** sorting, the quick filter and type-to-jump over 100k rows are cheap in JS once the rows are loaded. The alternative is doing them in Rust and sending only the visible window. That's a call for the architecture ticket. The prototype should measure both.

## 8. Installer and auto-update

**Verdict: works with a catch.**

- **Bundles:** NSIS (`-setup.exe`) and WiX (`.msi`). "`.msi` installers can only be created on Windows" ([Windows installer guide](https://v2.tauri.app/distribute/windows-installer/)).
  - NSIS `installMode` defaults to `currentUser`, which installs to `%LOCALAPPDATA%` without admin. `perMachine` and `both` need admin.
  - For WiX, keep `upgradeCode` fixed across releases ([config](https://v2.tauri.app/reference/config/)).
  - **Recommendation for a personal app:** NSIS, `currentUser`.
- **WebView2:** `webviewInstallMode` takes one of these values:

  | Mode | Extra size | Notes |
  |---|---|---|
  | `downloadBootstrapper` | none | Default; needs internet |
  | `embedBootstrapper` | ~1.8 MB | |
  | `offlineInstaller` | ~127 MB | |
  | `fixedRuntime` | ~180 MB | |
  | `skip` | none | |

  Windows 10 (April 2018 or later) and Windows 11 already ship WebView2, so the default is fine.
- **Updater:** `tauri-plugin-updater` 2.14.0 ([updater plugin](https://v2.tauri.app/plugin/updater/)).
  - Signing "cannot be disabled". Generate the keys with `tauri signer generate`, set `TAURI_SIGNING_PRIVATE_KEY` and its password as environment variables (not `.env`), and turn on `createUpdaterArtifacts`.
  - The endpoint can be a static JSON file such as a GitHub Releases `latest.json`, or a dynamic server.
  - On Windows the app exits while the installer runs (`passive`, `basicUi` or `quiet`).
  - Losing the private key strands every installed copy, so back it up.
- **Code signing:** this is separate from updater signing. Set `bundle.windows.signCommand`, which defaults to `signtool`.
  - The docs now show Azure Artifact Signing (formerly Trusted Signing) and Azure Key Vault via `relic`.
  - "Since 2024, an EV Certificate no longer gives your app an immediate reputation with Microsoft SmartScreen" ([Windows signing](https://v2.tauri.app/distribute/sign/windows/)).
  - For a personal app, unsigned builds are workable. Expect a SmartScreen "unknown publisher" prompt on each fresh download.

## What this means for other tickets

- **Prototype:** the first thing to prove is the virtual-file drag-out. Test it in this order:
  1. A data object with descriptor + contents and async capability, with the stream on its own COM thread.
  2. A large file over a slow link, checking the UI stays responsive.
  3. A folder tree.
  4. Whether Explorer applies `FD_WRITESTIME`.
  5. A mid-stream error.
  6. The stuck-`DoDragDrop` watchdog.

  Also measure 100k-row listing over a `Channel`.
- **Architecture:**
  - The drag-out code is Windows COM in the Rust backend, and it must call the SFTP layer from a non-UI thread. The SFTP session layer needs a blocking-friendly streaming read that a COM thread can pull from.
  - In-app drag-and-drop must use pointer events, not HTML5.
  - "Open terminal here" runs the system `ssh`, separate from the app's session.
- **v1 spec:** "Drag-and-drop to Explorer" will **not** go through the transfer queue. Explorer shows its own progress and overwrite prompt, and resume, conflict rules and temp-name-then-rename don't apply to that path. Either accept this for drag-out, or offer "Download to…" (which uses the queue) as the full-featured path. The spec should say which.
- **Packaging:** use NSIS `currentUser`, `downloadBootstrapper`, and `tauri-plugin-updater` with a GitHub Releases `latest.json`. Notifications only show the app's own name and icon in the installed build.
