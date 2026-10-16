# Feature inventory: SSH/SFTP remote file browsers

Prior-art survey for a new Rust SSH/SFTP file browser (github.com/prabhuakshay/sftp). Compiled 2026-10-16 from official docs, READMEs, feature pages and issue trackers. Sources are listed at the bottom.

**Project codes:** W = WinSCP, F = FileZilla (F-Pro = paid FileZilla Pro only), C = Cyberduck (MD = Mountain Duck only), T = Termius, ts = termscp, y = yazi, mc = Midnight Commander.
`(p)` = partial or limited support. `ext` = via official extension or plugin, not built in.

**Tiers:**
- **core**: nearly every client has it, and users expect it in an MVP.
- **common**: most mature clients have it, and it is a strong v1.x candidate.
- **advanced**: a differentiator or niche feature.

Who-has-it marks come from the vendor docs where they say so, plus well-known behavior of the products. Treat a missing code as "not documented / not found", not as proof that the product lacks the feature.

---

## 1. Connection & session management (17)

| Feature | What it does | Who has it | Tier |
|---|---|---|---|
| Quick connect | Connect by typing host/user/port/password without saving a site | W, F, C, T, ts | core |
| URL / CLI address | Open `sftp://user@host:port/path` from the command line or an OS URL | W, F, C, ts, y, mc | core |
| Saved sites (site manager) | Named, persistent connection entries with credentials and options | W, F, C, T, ts, y (vfs.toml) | core |
| Folders / groups for sites | Organise sites hierarchically; drag-and-drop ordering | W, F, C, T | common |
| Group-inherited settings | Child hosts inherit credentials, port, jump host and similar settings from their group | T | advanced |
| Tags, labels, colours | Tag or colour a site, and colour its tab or editor to tell prod from staging | W, T | common |
| Recent connections | History of recent hosts with last-access time | W, F, C, T, ts | core |
| Per-site initial directories | Remote and local start dirs per site | W, F, C, T, ts | core |
| Import `~/.ssh/config` hosts | Turn `Host` blocks into saved sites, or autofill when a host matches | W, C, T, ts | common |
| Import from other clients | Import sites from PuTTY, FileZilla, Transmit and others | W, C, T | common |
| Multiple simultaneous sessions | Several servers open at once (tabs or windows) | W, F, C, T, y, mc | core |
| Workspaces / session restore | Save and restore the set of open tabs, and reconnect on start | W, F, T | common |
| Keep-alive & auto-reconnect | Keep idle sessions alive; reconnect and resume after network drops | W, F, C, T | core |
| Cancel a hanging connect | Abort a stuck "Connecting..." attempt (open bug in ts #438) | W, F, C, T | core |
| Portable mode | Store config in a file next to the binary rather than the registry or user dir | W, F | advanced |
| Service discovery | Bonjour/mDNS discovery of servers | C | advanced |
| Connection profiles | Prebuilt provider templates (hosting companies) | C | advanced |

## 2. Authentication & keys (15)

| Feature | What it does | Who has it | Tier |
|---|---|---|---|
| Password auth | Plain SSH password login | all | core |
| Keyboard-interactive / 2FA | Multi-prompt auth: OTP, DUO, SecurID, Google Authenticator | W, F, C, T | core |
| Public-key file auth | Log in with a private key file | W, F, C, T, ts, y, mc | core |
| OpenSSH + PuTTY key formats | Read OpenSSH (`openssh-key-v1`, PEM) and `.ppk` keys | W, F, C (ppk RSA only), T | core |
| Modern key types | Ed25519, ECDSA, RSA with SHA-2 signatures | W, F, C, T, ts, y | core |
| Encrypted-key passphrase prompt | Prompt for, and optionally cache, a key passphrase (ts bug #451) | W, F, C, T, y | core |
| SSH agent | Use ssh-agent, Pageant, the Windows OpenSSH named pipe, or 1Password/Bitwarden via `IdentityAgent` | W (Pageant), F, C, T (own agent), ts, y, mc | core |
| Try default keys | Try `~/.ssh/id_*` automatically when none is configured | C (hidden option), ts | common |
| Agent forwarding | Forward the agent so the server can reach further hosts | W, T | common |
| Built-in key manager | Generate, import and store keys, and export the public key to a host | W (bundled PuTTYgen), T (Keychain), ts (key store) | common |
| OpenSSH user certificates | Authenticate with `*-cert.pub` signed certificates | W, C, T | advanced |
| FIDO2 / hardware keys | `sk-ed25519` / `sk-ecdsa` keys on YubiKey and similar devices | T | advanced |
| Device-bound passkeys | "SSH ID": non-exportable device keys published at a handle URL | T | advanced |
| GSSAPI / Kerberos | Single sign-on in enterprise realms | W | advanced |
| Change account password | Change the server password when it has expired | W | advanced |

## 3. Security & credential storage (12)

| Feature | What it does | Who has it | Tier |
|---|---|---|---|
| Host-key TOFU prompt | Show the fingerprint on first connect, then accept and store it | W, F, C, T, mc | core |
| Changed-host-key warning | Block or warn before sending credentials if the key has changed | W, F, C, T | core |
| Honour OpenSSH `known_hosts` | Read and write the user's standard `~/.ssh/known_hosts` | C, mc (libssh2) | common |
| Fingerprint display | Show the SHA256 (and MD5) host fingerprint in session info | W, F, C, T | core |
| "Don't save password" | Prompt for the password each time rather than storing it | all | core |
| OS keychain storage | Keep secrets in macOS Keychain, Windows Credential Manager or Secret Service | C, ts | common |
| Master password | Encrypt all stored credentials behind one passphrase | W, F (since 3.26) | common |
| Encrypted local vault | Encrypted local database for hosts and keys | T | common |
| E2E-encrypted cloud vault sync | Sync hosts, keys and snippets across devices and teams | T (paid tiers), F-Pro (site sync) | advanced |
| Client-side encryption | Encrypt file names and contents (Cryptomator-compatible vaults) | C, MD | advanced |
| Algorithm config | Choose or order ciphers, KEX, host-key and MAC algorithms | W, C | advanced |
| Admin lockdown | Policy that forbids unknown host keys or stored passwords | W | advanced |

## 4. Proxies, jump hosts & port forwarding (8)

| Feature | What it does | Who has it | Tier |
|---|---|---|---|
| HTTP / SOCKS proxy | Reach SSH through an HTTP CONNECT or SOCKS4/5 proxy | W, F, C, T | common |
| Single jump host | Connect through one bastion (ProxyJump with one hop) | W (Tunnel page), C (7.7+, ProxyJump), T | common |
| Multi-hop chains | Chain several bastions (`ProxyJump a,b,c`) | T | advanced |
| ProxyCommand / local proxy command | Use an arbitrary command as the transport (e.g. `cloudflared`, `aws ssm`) | W | advanced |
| Local port forward (-L) | Expose a remote service on a local port | T | advanced |
| Remote port forward (-R) | Expose a local service on the server | T | advanced |
| Dynamic SOCKS forward (-D) | Run a local SOCKS proxy through SSH | T | advanced |
| Saved, toggleable forwarding rules | Named rules started and stopped independently of a browse session | T | advanced |

## 5. Browsing & navigation (18)

| Feature | What it does | Who has it | Tier |
|---|---|---|---|
| Detailed listing | Columns for name, size, mtime, permissions, owner and group | W, F, C, T, ts, y, mc | core |
| Sort & dirs-first | Sort by any column; group directories first or last | W, F, C, T, ts, y, mc | core |
| Hidden-files toggle | Show or hide dotfiles | W, F, C, T, ts, y, mc | core |
| Editable path bar / go-to | Type or paste a path and jump to it | W, F, C, T, ts, y, mc | core |
| History & parent nav | Back, forward and up; Backspace for previous dir | W, F, C, ts, y, mc | core |
| Symlink handling | Show link targets and follow links into directories | W, F, C, ts, y, mc | core |
| Refresh / listing cache | Cache listings; manual and automatic refresh after operations | W, F, C, T, mc | core |
| Dual pane local + remote | Side-by-side local and remote panels | W (Commander), F, T, ts, mc | core |
| Single-pane explorer mode | Remote-only, Explorer- or Finder-style view | W (Explorer), C | common |
| Remote ↔ remote panes | Both panes on different servers | T, mc, C (two windows) | common |
| Directory tree | Collapsible folder tree or outline view | W, F, C (outline), mc | common |
| Synchronized browsing | Local and remote panes navigate in lock-step | W, F, ts | common |
| Directory bookmarks / hotlist | Saved paths per site or global | W, F, mc (hotlist), y (plugins) | common |
| Directory size calc | Compute the recursive size of a folder | W, C, ts, mc | common |
| Miller-column layout | Parent, current and preview columns (ranger-style) | y | advanced |
| Frecency / fuzzy jump | Jump to dirs with zoxide or fzf; fuzzy-match names | y, ts (nucleo) | advanced |
| Panelize | Turn command or find output into a browsable listing | mc | advanced |
| Custom listing format | User-defined columns or format string | W, F, ts, mc | advanced |

## 6. File operations (14)

| Feature | What it does | Who has it | Tier |
|---|---|---|---|
| Create file / directory | New empty file or folder | W, F, C, T, ts, y, mc | core |
| Rename | In-place rename | W, F, C, T, ts, y, mc | core |
| Delete (recursive) | Delete files and trees, with confirmation | W, F, C, T, ts, y, mc | core |
| Remote move | Move within the server (SFTP rename) | W, F, C, T, ts, y, mc | core |
| Multi-select | Range, toggle, select-all, invert, select by mask, visual mode | W, F, C, T, ts, y, mc | core |
| Remote copy / duplicate | Server-side copy (via `cp` or the SFTP copy-data extension) without a round trip | W, C, ts, mc | common |
| Create symlinks | Make symbolic (and hard) links | W, C, ts, mc | common |
| Copy path / URL | Copy the full path or URL to the clipboard | W, F, C | common |
| Bulk rename in editor | Rename many files by editing names in `$EDITOR` | y, mc (masks) | advanced |
| Remote trash | Delete by moving to a recycle directory | W, C (some protocols) | advanced |
| Server-side archive | Zip, tar or extract on the server through the shell | C, W (ext) | advanced |
| Browse archives as dirs | Enter tar or zip files like folders | mc, y (planned) | advanced |
| Server-to-server copy | Copy between two remote hosts | C, T (copy/paste), mc | advanced |
| File locking | Lock files to prevent conflicting edits (WebDAV, Office) | W, MD | advanced |

## 7. Transfers (25)

| Feature | What it does | Who has it | Tier |
|---|---|---|---|
| Upload / download (recursive) | Transfer files and whole trees | W, F, C, T, ts, y, mc | core |
| Drag-and-drop between panes | Drag from one panel to the other to transfer | W, F, C, T | core (GUI) |
| Transfer queue with progress | Per-item and total progress, speed and ETA | W, F, C, T, ts (p) | core |
| Background transfers | Keep browsing while transfers run (ts lacks it: #326 "wontdo") | W, F, C, T, y (tasks), mc | core |
| Cancel transfer | Abort an item or everything | W, F, C, T, ts, y, mc | core |
| Resume interrupted | Continue partial files instead of restarting | W, F, C | core |
| Conflict policy | Overwrite, skip, resume, rename, newer-only, size-differs, or ask, with "apply to all" | W, F, C, T, ts (p) | core |
| Pre-scan total size | Calculate the total before starting (needs feedback; ts #361 and #384) | W, ts | common |
| Parallel transfers | N concurrent connections for many files (W and F default 2) | W, F, C, y | common |
| Pause / suspend items | Suspend and resume individual or all jobs | W, F, C | common |
| Bandwidth limit | Global or per-transfer KB/s cap, adjustable mid-transfer | W, F, C | common |
| Preserve timestamps | Set the remote or local mtime to match the source | W, F, C | common |
| Permission mask on upload | Apply or preserve modes on upload and download | W, C | common |
| Transfer filters | Include or exclude by mask or regex (`.git`, `node_modules`) | W, F, C | common |
| Failed-transfer list & retry | Keep failed items and retry them | W, F, C | common |
| Persistent queue | The queue survives an app restart | F, C | common |
| Move / delete after transfer | Delete the source after it has been copied | W, C | common |
| Skip unreadable & continue | Don't abort the batch on one permission error (ts #450) | W, F | common |
| Completion notification | Desktop notification and taskbar progress | W, C, ts | common |
| Queue reorder / execute now | Change priority, or force-start an item past the limit | W, F | advanced |
| Segmented single-file download | Split a large file across connections and merge (C: >100 MB) | C, W | advanced |
| Temp-name + atomic rename | Upload to `file.filepart`, rename on success; enables auto-resume | W | advanced |
| Post-queue action | Disconnect, sleep or shut down when the queue empties | W, F | advanced |
| Checksum verify after transfer | Compare hashes of the source and the destination | C (S3), W (manual) | advanced |
| Text-mode line-ending conversion | Convert CRLF↔LF for text files (mostly FTP) | W, F | advanced |

## 8. Sync & comparison (9)

| Feature | What it does | Who has it | Tier |
|---|---|---|---|
| Directory compare (highlight) | Mark files that are newer, different or only on one side | W, F, mc | common |
| One-way sync with preview | Upload- or download-only sync, with a checklist to tick or untick actions | W, C | common |
| Two-way sync / mirror | Bidirectional newest-wins sync, or mirror with deletes | W, C | advanced |
| Compare criteria | mtime, size, checksum or byte-for-byte (mc "thorough") | W, C, mc | advanced |
| Watch local → auto upload | Watch a local dir and push changes ("keep remote up to date") | W, ts (one-way, ~5 s delay) | advanced |
| Sync timestamps only | Fix mtimes without transferring data | W | advanced |
| Sync filters / profiles | Saved sync definitions with excludes | W, C (CLI), F-Pro (CLI) | advanced |
| File diff local vs remote | Open a diff of the two versions | W (diff custom cmds / ext), mc (mcdiff) | advanced |
| Scheduled / unattended sync | Run a sync from cron or Task Scheduler through the CLI | W, C (duck), F-Pro | advanced |

## 9. Editing, viewing & preview (11)

| Feature | What it does | Who has it | Tier |
|---|---|---|---|
| Edit in external editor + auto-upload | Download to temp, watch for saves, upload back | W, F, C, T, ts | core |
| Open with default app | Open in the OS default handler | W, F, C, T, ts | core |
| Properties / info view | Size, times, mode, owner, link target | W, F, C, T, ts, mc | core |
| "Open with..." + remembered associations | Pick a program per extension and remember it | W, F, T, ts | common |
| Internal text editor | Built-in editor with encoding, find/replace and go-to-line | W, mc | common |
| Quick preview | Text with syntax highlighting, images, PDFs, archives, without a full download | C (Quick Look), y, mc (quick view) | common |
| New-file-and-edit | Create a file and open it in the editor straight away | W, ts | common |
| Remote-changed conflict detection | Warn when a file changed on the server while you edited it | (W p) | advanced |
| Hex viewer | Binary or hex view | mc | advanced |
| Edit as root / sudo | Edit files the login user can't write | (none natively; W via sudo `sftp-server`) | advanced |
| File versions / revert | View and restore previous versions (S3 versioning) | C, MD | advanced |

## 10. Permissions, ownership & properties (7)

| Feature | What it does | Who has it | Tier |
|---|---|---|---|
| chmod dialog | Checkbox and octal editor | W, F, C, T, ts, mc | core |
| Recursive chmod | Apply to a subtree, with files-only or dirs-only options | W, F | common |
| Bulk chmod on selection | Set the mode on many selected files at once (a Termius complaint) | W, F, C, mc | common |
| chown / chgrp | Change owner and group by name or uid | W, mc | common |
| Remote checksum | MD5 or SHA via SFTP `check-file` or a shell command | W | advanced |
| Free space / quota | `statvfs` or the quota extension shows free disk space | W, MD | advanced |
| chattr / ACLs | Linux attributes; cloud ACLs | mc (chattr), C (S3/Azure ACL) | advanced |

## 11. Search & filtering (8)

| Feature | What it does | Who has it | Tier |
|---|---|---|---|
| Quick filter of the listing | Live filter by substring, wildcard or regex | W, F, C, ts, y, mc | core |
| Type-to-jump / incremental find | Jump to the first match as you type, with match count | W, F, C, y, mc | core |
| Recursive name search | Find files by name or mask under the current dir | W, F, C, ts, y (local), mc | common |
| Named filter sets | Saved, reusable filters (e.g. "hide build output") | F | advanced |
| Search by size or date | Filter by size, modified-since and similar | W, mc | advanced |
| Content search (grep) | Search inside remote files | mc, W (ext only) | advanced |
| Server-side search via SSH exec | Run `find`, `fd` or `rg` on the server rather than walking SFTP (y #4158 asked for it) | (none built-in; y custom provider) | advanced |
| Fuzzy finder over results | fzf or nucleo over names and results | y (local), ts | advanced |

## 12. Terminal & remote commands (8)

| Feature | What it does | Who has it | Tier |
|---|---|---|---|
| Open SSH terminal here | Launch a shell at the current remote dir (external terminal or PuTTY) | W, C, T, mc | common |
| Embedded terminal pane | Terminal inside the app | T, ts | common |
| One-off remote command | Run a command and show its output in a console | W, C, ts, mc | common |
| Custom commands with placeholders | User-defined local or remote commands over the selection (`!`, `!&`, path and host vars) | W, mc (user menu), y (plugins) | advanced |
| Snippets library | Saved commands runnable on many hosts | T | advanced |
| Broadcast input | Type into several sessions at once | T | advanced |
| Command autocomplete / history | Suggest paths, history and snippets | T, ts (history) | advanced |
| sudo / su operations | Browse and transfer as another user (W: custom `sudo sftp-server` path; ts #323 unsolved) | W (p) | advanced |

## 13. Multi-protocol support (10)

| Feature | What it does | Who has it | Tier |
|---|---|---|---|
| SFTP | SSH File Transfer Protocol v3+ | all | core |
| Local filesystem as a "remote" | Same UI for local-to-local operations | W, C, ts, y, mc | common |
| SCP fallback | For servers without the SFTP subsystem | W, ts, mc | common |
| Shell-only file access (FISH) | Do file operations through a plain shell (fast in mc compared with sftpfs) | mc, W (SCP shell) | advanced |
| FTP / FTPS | Legacy protocols | W, F, C, ts, mc | advanced (out of scope?) |
| WebDAV | HTTP-based file access | W, C, ts, F-Pro | advanced |
| S3 & object storage | S3, GCS, Azure, B2, Swift | W (S3), C, ts (S3, GCS), F-Pro | advanced |
| SMB | Windows shares | C (MD 5), ts | advanced |
| Kubernetes pods | Browse files inside k8s pods | ts | advanced |
| Cloud drives / rclone | Google Drive, Dropbox, OneDrive, Box; rclone backends | C, F-Pro, y (rclone) | advanced |

## 14. Automation, scripting & CLI (8)

| Feature | What it does | Who has it | Tier |
|---|---|---|---|
| Open-at-target CLI | `app user@host:/path` launches straight into a session | W, F, C, ts, y | core |
| Non-interactive transfer CLI | Upload, download or sync from the shell with exit codes | W (`winscp.com`), C (`duck`), F-Pro (CLI) | common |
| Scripting language | Batch scripts of commands (open, put, get, synchronize, keepuptodate, checksum) | W | advanced |
| Library / API | .NET/COM assembly, PowerShell | W | advanced |
| Generate script/code from GUI | Export the current session or transfer as a script or code snippet | W | advanced |
| Plugins / extensions | Lua plugins (y), extension files (W), extfs scripts (mc) | W, y, mc | advanced |
| Env / config overrides | Override preferences via env vars and profiles | C | advanced |
| Self-update command | `app update` | ts | common |

## 15. UI conveniences (16)

| Feature | What it does | Who has it | Tier |
|---|---|---|---|
| Session / transfer log panel | Protocol transcript and errors visible in the app | W, F, C, T, ts | core |
| Keyboard-first operation | Every action reachable by keys (F-keys or letters) | W, mc, ts, y | core (TUI) |
| Help / shortcut overlay | F1 or `?` key reference | W, ts, y, mc | core (TUI) |
| Tabs | Multiple sessions or directories as tabs | W, F, C (macOS), T, y | common |
| Customisable keybindings | Remap keys, including vim presets (ts #374 open, #358 declined) | y, mc | common |
| Vim-style navigation | hjkl, visual mode, `/` search | y | common (TUI) |
| Themes / dark mode / skins | Colour schemes and custom themes | W, C, T, ts, y (flavors), mc (skins) | common |
| Configurable columns | Show, hide and reorder columns | W, F, C, mc | common |
| Status bar | Selection count and size, free space, connection info | W, F, C, mc | common |
| Mouse support (TUI) | Click and scroll in the terminal | y, mc | common |
| File-type icons | Nerd-font or OS icons per type | W, F, C, T, y | common |
| Localization | Multiple UI languages | W, F, C, MD | common |
| Split / workspace layouts | Several sessions side by side | T | advanced |
| Confirm-dangerous-ops settings | Per-action confirmations (delete, overwrite) | W, F, ts | common |
| Notifications config | Minimum size before a completion notification fires | ts | advanced |
| Logging to file | Persistent debug or session logs | W, F, C, ts | common |

## 16. OS integration (11)

| Feature | What it does | Who has it | Tier |
|---|---|---|---|
| Drag-and-drop with the OS file manager | Drag in from, and out to, Explorer or Finder | W, F, C, T | common (GUI) |
| Default `sftp://` handler | Register as the system URL handler | W, C | common |
| Taskbar / dock progress | Progress shown on the taskbar or dock icon | W, C | common |
| Mount as drive | Remote appears as a disk in Finder or Explorer (FUSE / File Provider / Cloud Files API) | MD, y (sshfs plugin) | advanced |
| Smart sync / offline files | Placeholders downloaded on open; pin for offline use | MD | advanced |
| OS search indexing | Spotlight or Windows Search over remote files | MD | advanced |
| Shell context-menu integration | "Upload with...", "Copy link" from Explorer or Finder | W, MD | advanced |
| Tray / menubar agent | Background agent showing sync status | MD | advanced |
| Jump lists / shortcut icons | Recent sites in the taskbar jump list | W | advanced |
| PuTTY interop | Share sites with PuTTY and launch it | W | advanced |
| No-admin install / portable | Install per user without admin rights | W, MD | common |

## 17. Sharing & teams (4)

| Feature | What it does | Who has it | Tier |
|---|---|---|---|
| Copy web / share URL | Map a remote path to an HTTP URL or pre-signed link | C, MD, W (p) | advanced |
| Team-shared vaults | Shared hosts and credentials with access control | T | advanced |
| Settings sync across machines | Sites and keys synced through the cloud | T, F-Pro | advanced |
| Shared session logs | Team-visible, searchable session logs | T | advanced |

**Total: 17 categories, 201 features.**

---

## termscp internals (closest Rust prior art)

- **SSH/SFTP:** `remotefs` (core trait) plus `remotefs-ssh` (SFTP and SCP), with `ssh2-config` for `~/.ssh/config`.
  - termscp 1.x builds `remotefs-ssh` with the pure-Rust `russh` backend.
  - Earlier releases used libssh2 (`ssh2` crate), then libssh from 0.19.
  - `remotefs-ssh` still supports all three backends as features.
- **UI:** `tuirealm` 4 and `tui-realm-stdlib`, a component framework on top of `ratatui`/`crossterm`. Other UI crates:
  - `tui-term` for the embedded terminal
  - `nucleo` for fuzzy matching
  - `notify` for the file watcher
  - `notify-rust` for desktop notifications
  - `keyring-core` with per-OS stores for secrets
  - `self_update`
  - `tokio` (multi-thread)
- **For comparison, yazi:** its SFTP VFS is its own `yazi-sftp` crate on `russh` 0.64 + `tokio`, with UI on `ratatui-core`/`ratatui-widgets`. mc's sftpfs uses libssh2.

## Gaps and user complaints (opportunities for a new tool)

**FileZilla**
- Saved passwords were stored in plaintext, then base64 in `sitemanager.xml`. The maintainer resisted encryption for about a decade, and a master password only arrived in 3.26 (2017).
- The 2018 adware-bundled installer was promoted as the primary download, which damaged trust.
- No ProxyJump or `~/.ssh/config` support. A forum moderator answered "This is not possible" and suggested a manual port forward.
- Folder synchronization has been requested since roughly 2005 (tickets #1591, #2328) and is still not in the free client. Only compare and synchronized browsing exist.

**WinSCP**
- Windows-only.
- Its tunnel supports only one jump hop; for multi-hop or non-default setups the docs say to use PuTTY instead.
- sudo needs a hand-configured `sudo sftp-server` path and a NOPASSWD sudoers rule.
- Find Files can't search content (it needs an extension) and only works on remote dirs.
- The internal editor is described as "similar to Notepad".

**Cyberduck / Mountain Duck**
- Multi-hop ProxyJump is not supported.
- `CertificateFile` is not supported, and `.ppk` keys must be RSA only.
- Long history of slow listings, especially with many symlinks, a UI that bogs down with large folders, and heavy memory use on big transfers.
- Sync has no pattern filters in the GUI and points users to the CLI or rsync.
- No dual pane.
- Mountain Duck is paid.

**Termius**
- Users report a slow SFTP client that freezes when deleting more than 1,000 files, missing shortcuts, and no bulk permission edit.
- Cross-device sync and the cloud vault are paywalled (Pro and up).
- Closed source and account-centric.

**termscp**
- SFTP throughput was once about 200 KB/s against 12 MB/s for OpenSSH `sftp` (#401, fixed in 1.0).
- Repeated backend churn (libssh2 → libssh → russh) caused auth regressions: #356, #415, #422, and #451 (passphrase prompt).
- No background or parallel transfers (#326, #359). The maintainer says the UI and the transfers share a thread and a fix would mean a near-rewrite: "If I had to implement termscp from scratch today I would do so".
- Can't cancel a hanging connect (#438).
- No customisable keybindings (#374).
- The watcher is one-way only with about a 5 s delay.
- sudo is unsupported (#323).
- A single unreadable file aborts the whole transfer (#450).

**yazi (SFTP VFS, new in 2025–26)**
- Doesn't read `~/.ssh/config`, so there is no ProxyJump. Each server must be declared in `vfs.toml`.
- Search and fzf are disabled on remote filesystems (#3395).
- Previews need a full download. #4158 asks for server-side `rg`/`fd`/preview generation.
- Agent auth bugs.

**Midnight Commander**
- sftpfs historically topped out around 500 KB/s, compared with about 10 MB/s for its shell-based FISH VFS (#2946).

**Cross-cutting opportunities**
- First-class OpenSSH config: Include, ProxyJump multi-hop, IdentityAgent, CertificateFile and Match. Very few clients do this fully.
- A transfer engine kept separate from the UI thread: async, parallel, pausable and resumable, with a persistent queue.
- Pipelined SFTP reads and writes so throughput matches OpenSSH `sftp`.
- Server-side acceleration over the same SSH connection: `find`/`rg` search, `du`, checksum, `cp`, tar-stream for many small files.
- sudo or "act as user" support.
- Safe-by-default secrets: OS keyring, standard `known_hosts`, no plaintext.
- Customisable keybindings.
- Cancellable connects.
- Conflict detection when re-uploading edited files.
- Graceful skip-and-continue on errors.
- Fast handling of directories with 100k+ entries.

---

## Sources

- WinSCP introduction / features: https://winscp.net/eng/docs/introduction
- WinSCP home: https://winscp.net/eng/index.php
- WinSCP task index: https://winscp.net/eng/docs/task_index
- WinSCP tunnel (jump host): https://winscp.net/eng/docs/ui_login_tunnel
- WinSCP background queue: https://winscp.net/eng/docs/transfer_queue
- WinSCP queue list: https://winscp.net/eng/docs/ui_queue
- WinSCP synchronize: https://winscp.net/eng/docs/task_synchronize_full
- WinSCP keep remote dir up to date: https://winscp.net/eng/docs/task_keep_up_to_date
- WinSCP resume / temp filename: https://winscp.net/eng/docs/ui_pref_resume
- WinSCP custom commands: https://winscp.net/eng/docs/custom_command
- WinSCP Commander UI: https://winscp.net/eng/docs/ui_commander
- WinSCP find files: https://winscp.net/eng/docs/task_find
- WinSCP internal editor: https://winscp.net/eng/docs/ui_editor
- WinSCP host key verification: https://winscp.net/eng/docs/ssh_verifying_the_host_key
- WinSCP checksum: https://winscp.net/eng/docs/scriptcommand_checksum
- WinSCP sudo discussion: https://winscp.net/forum/viewtopic.php?t=3046
- FileZilla features (official): https://filezilla-project.org/client_features.php
- FileZilla feature list (Flathub mirror of official text): https://flathub.org/apps/org.filezillaproject.Filezilla
- FileZilla Pro / Pro CLI: https://filezillapro.com/filezilla-pro/ , https://filezillapro.com/docs/v3/filezilla-product-comparisons/filezilla-cli-vs-filezilla-pro-cli-server-automation-vs-multi-cloud-integration
- FileZilla master password history: https://trac.filezilla-project.org/ticket/2935 , https://it.slashdot.org/story/17/05/26/2236205/10-years-later-filezilla-adds-support-for-master-password-that-encrypts-your-logins , https://en.wikipedia.com/wiki/FileZilla
- FileZilla jump host answer: https://forum.filezilla-project.org/viewtopic.php?p=178709
- FileZilla sync tickets: https://trac.filezilla-project.org/ticket/1591 , https://trac.filezilla-project.org/ticket/2328
- Cyberduck home / features: https://cyberduck.io/
- Cyberduck SFTP: https://docs.cyberduck.io/protocols/sftp/
- Cyberduck transfers: https://docs.cyberduck.io/cyberduck/transfer/
- Cyberduck sync: https://docs.cyberduck.io/cyberduck/sync/
- Cyberduck browser: https://docs.cyberduck.io/cyberduck/browser/
- Cyberduck CLI: https://docs.cyberduck.io/cli/
- Cyberduck ProxyJump multi-hop ticket: https://lists.cyberduck.io/pipermail/cyberduck-trac/2021-January/045330.html
- Cyberduck slow listing tickets: https://lists.cyberduck.io/pipermail/cyberduck-trac/2012-October/021509.html , https://lists.cyberduck.io/pipermail/cyberduck-trac/2018-September/039140.html , https://lists.cyberduck.io/pipermail/cyberduck-trac/2012-January/019995.html
- Mountain Duck: https://mountainduck.io/
- Termius home: https://termius.com/
- Termius docs overview: https://docs.termius.com/
- Termius SFTP: https://docs.termius.com/organize-and-connect-to-hosts/managing-files-with-sftp
- Termius connecting (auth, jump hosts, proxies, agent): https://docs.termius.com/organize-and-connect-to-hosts/connecting-to-a-server
- Termius port forwarding: https://docs.termius.com/organize-and-connect-to-hosts/port-forwarding-and-tunneling
- Termius keys & certificates / FIDO2: https://docs.termius.com/keychain/ssh-keys-and-certificates , https://termius.com/blog/using-fido2-with-ssh
- Termius pricing: https://termius.com/pricing
- Termius reviews (complaints): https://ca.trustpilot.com/review/termius.com
- termscp README: https://github.com/veeso/termscp
- termscp site: https://termscp.rs/
- termscp Cargo.toml: https://raw.githubusercontent.com/veeso/termscp/main/Cargo.toml
- termscp manual (shortcuts, file watcher, config, ssh keys): https://github.com/veeso/termscp/tree/main/docs/en-US
- termscp issues: https://github.com/veeso/termscp/issues (#323, #326, #356, #359, #374, #401, #415, #422, #438, #450, #451)
- remotefs-ssh backends: https://github.com/remotefs-rs/remotefs-rs-ssh
- yazi features: https://yazi-rs.github.io/features
- yazi VFS (SFTP) config: https://yazi-rs.github.io/docs/configuration/vfs
- yazi VFS known issues: https://github.com/sxyazi/yazi/issues/3395 , https://github.com/sxyazi/yazi/issues/4158
- yazi-sftp crate: https://github.com/sxyazi/yazi/tree/main/yazi-sftp
- Midnight Commander: https://midnight-commander.org/
- mc manual: https://source.midnight-commander.org/man/mc.html
- mc sftpfs slowness: https://github.com/MidnightCommander/mc/issues/2946
