# Choosing the Rust SSH/SFTP library

Research for [#4](https://github.com/prabhuakshay/sftp/issues/4). Compiled 2026-10-16 from crates.io, the crates' source code at their current releases, their changelogs and issue trackers. Versions and dates come from the crates.io API on that day.

## Answer

**Use `russh` 0.64 with `russh-sftp` 3.0 directly, and parse `~/.ssh/config` with `ssh2-config` 0.8.** Build `russh` with the `ring` crypto backend so the Windows build needs nothing beyond the MSVC Build Tools that Tauri already needs.

**Runner-up: `ssh2` 0.9 (libssh2).** It works, but on Windows its default crypto backend cannot handle Ed25519 keys at all, so it would need OpenSSL. It is also blocking, so it does not fit a Tokio-based Tauri backend.

**Don't use `remotefs-ssh`.** It is a synchronous wrapper with a small user base. It still pins `russh-sftp` 2.x, the version from before pipelined reads. Most of what it adds is what this app needs to control itself: the host key prompt, cancelling, and the transfer queue.

What we still have to write ourselves:
- **known_hosts handling.** `russh`'s is too simple for a real `known_hosts` file.
- **Connection lifecycle.** Reconnect, cancel and keep-alive policy.
- **A few defensive settings on `russh-sftp`.** These are listed under [Risks](#risks-and-catches).

## Candidates at a glance

| | `russh` + `russh-sftp` | `ssh2` (libssh2) | `remotefs-ssh` |
|---|---|---|---|
| Latest | russh 0.64.1 (2026-10-05), russh-sftp 3.0.1 (2026-09-28) | ssh2 0.9.6 (2026-06-30), libssh2-sys 0.3.3 (2026-09-05) | 1.0.0 (2026-09-15) |
| Language | Pure Rust, async (Tokio) | C library bindings, blocking | Rust wrapper over libssh2, libssh or russh; sync `RemoteFs` trait |
| Releases in the last 3 months | russh: 7, russh-sftp: 3 | ssh2: 0, libssh2-sys: 1 | 4 |
| Recent downloads (90 days) | 3.56 M / 1.73 M | 1.25 M | 5.5 k |
| Ed25519 key on Windows, no OpenSSL | Yes | **No** (WinCNG backend has no Ed25519) | Depends on backend |
| Pipelined SFTP | Yes, 16 reads and 16 writes in flight by default (since 3.0.0) | libssh2 reads ahead and splits large writes | Uses russh-sftp **2.x**, without the 3.0 read pipeline |
| Windows build | MSVC Build Tools. With `ring`, nothing else. With `aws-lc-rs`, NASM or the `prebuilt-nasm` feature | WinCNG: MSVC only. With Ed25519 support: OpenSSL through vcpkg or vendored, and vendored needs Perl | Default backend pulls in `openssl` |

Sources: crates.io API (`/api/v1/crates/<name>`), queried 2026-10-16. The other rows are cited in the sections below.

## 1. SFTP transfer speed

**russh-sftp 3.0 pipelines both directions.** Version 3.0.0 (2026-09-08) shipped `perf(client): pipeline reads; reduce packet copies; fix request waits` ([commit log](https://github.com/AspectUnk/russh-sftp/commits/master)).
- **Reads.** `File`'s `AsyncRead` probes the server's chunk size with one request, then keeps up to `max_concurrent_reads` `SSH_FXP_READ`s in flight. A short read drops the queue and restarts from the gap ([`client/fs/file.rs` L49-137](https://github.com/AspectUnk/russh-sftp/blob/2ef937985fa965a3e1a8a83828ce7acab30719d6/src/client/fs/file.rs#L49-L137)).
- **Writes.** They are queued up to `max_concurrent_writes` unacknowledged requests (same file, `poll_write`).
- **Defaults.** 16 reads and 16 writes in flight, 32 KiB write packets, 256 KiB maximum packet. It honours `limits@openssh.com` when the server offers it ([`client/mod.rs` L32-55](https://github.com/AspectUnk/russh-sftp/blob/2ef937985fa965a3e1a8a83828ce7acab30719d6/src/client/mod.rs#L32-L55), [`client/session.rs`](https://github.com/AspectUnk/russh-sftp/blob/2ef937985fa965a3e1a8a83828ce7acab30719d6/src/client/session.rs)).
- **For comparison:** OpenSSH `sftp` defaults to 64 requests of 32 KiB in flight (`sftp -R 64 -B 32768`, [sftp(1)](https://man.openbsd.org/sftp)). On high-latency links we should raise `max_concurrent_*` to about 64 to match.

**Benchmarks:**
- **russh-sftp README.** Six runs, median, 128 MiB, against OpenSSH in Docker on tmpfs, AES-128-GCM, 16 in flight, `TCP_NODELAY` on. Results ([README, Performance](https://github.com/AspectUnk/russh-sftp#performance)):
  - russh-sftp with russh 0.63.2: 257 MiB/s up, 203 MiB/s down.
  - Go `pkg/sftp`: 255 / 197 MiB/s.
  - Node `ssh2`: 253 / 198 MiB/s.
  - This is the crate's own benchmark, run on loopback, so it shows CPU overhead, not WAN behaviour.
- **termscp maintainer.** russh backend through remotefs-ssh with a 64 KiB read/write loop, on loopback: 139–149 MiB/s, against `sftp(1)` at 164–192 MiB/s, so 18–29 % slower ([termscp#401, closing comment](https://github.com/veeso/termscp/issues/401)). That run predates russh-sftp 3.0's read pipeline.
- **The README's tuning advice.** It says to set `russh::client::Config { nodelay: true, .. }` for these numbers. `client::connect` applies it ([russh `client/mod.rs` `connect`](https://github.com/Eugeny/russh/blob/v0.64.1/russh/src/client/mod.rs)).

**The termscp 200 KB/s regression was not russh.** The report ([termscp#401](https://github.com/veeso/termscp/issues/401), March 2026) was against termscp 0.19.1:
- That version used the **libssh** backend (`libssh-rs`). The maintainer says so in the thread: "since 0.19 I switched to libssh".
- It got under 200 KB/s where `sftp` got 12 MB/s.
- The fix was to switch termscp 1.0 to russh ([commit 52dd762](https://github.com/veeso/termscp/commit/52dd762), "replaced libssh with russh for remotefs-ssh").
- An earlier complaint about slow SFTP ([termscp#81](https://github.com/veeso/termscp/issues/81), 2021) was blamed by the same maintainer on libssh2.

**ssh2 / libssh2.** libssh2's `sftp.c` keeps a list of read-ahead chunks and splits a large write buffer into packets of up to `MAX_SFTP_OUTGOING_SIZE`. Both directions are therefore pipelined, but only if the caller passes large buffers ([libssh2 `src/sftp.c`, bundled commit 4aded1b](https://github.com/libssh2/libssh2/blob/4aded1bf2de0a7ceb02cd50b0b1f7a826984783a/src/sftp.c)). We found no current, independent ssh2-rs vs. OpenSSH benchmark.

## 2. Login methods

| Need | russh | ssh2 |
|---|---|---|
| Ed25519 key, no passphrase | `keys::load_secret_key(path, None)` then `authenticate_publickey` | Only with the OpenSSL backend on Windows (see §7) |
| Ed25519 key with passphrase | `load_secret_key(path, Some(pass))`. Returns `Error::KeyIsEncrypted` when one is needed, which is our cue to show the prompt | `userauth_pubkey_file(.., passphrase)`, OpenSSL backend only |
| Password | `authenticate_password` | `userauth_password` |
| Keyboard-interactive (2FA) | `authenticate_keyboard_interactive_start` / `_respond`. Async, so the UI can answer prompts as they arrive | `userauth_keyboard_interactive` with a synchronous `KeyboardInteractivePrompt` callback, which blocks that thread while the UI answers |
| PuTTY `.ppk` (later) | `decode_secret_key` detects `PuTTY-User-Key-File-` and parses it | No |

Sources:
- russh: [`client/mod.rs` `authenticate_*`](https://github.com/Eugeny/russh/blob/v0.64.1/russh/src/client/mod.rs), [`keys/mod.rs` `load_secret_key`, `Error::KeyIsEncrypted`](https://github.com/Eugeny/russh/blob/v0.64.1/russh/src/keys/mod.rs), [`keys/format/mod.rs` L44-46](https://github.com/Eugeny/russh/blob/v0.64.1/russh/src/keys/format/mod.rs#L44-L46). The README lists `publickey`, `password`, `keyboard-interactive`, `none` and certificate auth ([README](https://github.com/Eugeny/russh#authentication-methods)).
- ssh2: [`src/session.rs`](https://github.com/rust-lang/ssh2-rs/blob/0a11bdd4eecf4525e540c30f14b15ab1c07194b7/src/session.rs) (`userauth_keyboard_interactive` L356, `userauth_pubkey_file` L507).

The README's recommended algorithms include `ssh-ed25519`, `curve25519-sha256`, `chacha20-poly1305@openssh.com`, AES-GCM and post-quantum `mlkem768x25519-sha256`. That covers Windows OpenSSH 10.5 and any current server.

## 3. Host key checks against `~/.ssh/known_hosts`

**russh gives us a starting point, not a finished feature.**
- **The API.** The `russh::keys::known_hosts` module has `check_known_hosts[_path]`, `known_host_keys[_path]` and `learn_known_hosts[_path]`. A mismatching key of the same algorithm returns `Error::KeyChanged { line }`.
- **The default path.** It is `env::home_dir()/.ssh/known_hosts` ([`keys/known_hosts.rs`](https://github.com/Eugeny/russh/blob/v0.64.1/russh/src/keys/known_hosts.rs)). That resolves to `%USERPROFILE%\.ssh\known_hosts` on Windows, the same file Windows OpenSSH uses.
- **The hook.** `Handler::check_server_key` takes a `PublicKeyOrCertificate` since 0.63.0 ([release notes](https://github.com/Eugeny/russh/releases/tag/v0.63.0)). That is where we decide trust, so the "trust this new key?" dialog fits naturally.

What it handles:
- Plain hostnames.
- `[host]:port` for ports other than 22.
- Comma-separated host lists.
- Hashed `|1|salt|hash` entries.
- Appending a new line, unhashed, which matches Windows OpenSSH's default `HashKnownHosts no` (`match_hostname` L109-131, `learn_known_hosts_path` L137-174).

What it does **not** handle (from reading the same file):
- **Wildcards** (`*`, `?`) and negation (`!`). The comparison is exact (`host == entry`).
- **`@cert-authority` and `@revoked` markers.** A marker line is read with the marker as the host field, so it never matches, and `@revoked` is silently ignored.
- **Tabs or several spaces between fields.** The line is split on single spaces.
- **Accurate line numbers.** Comment lines are skipped before the counter is incremented, so the `line` in `KeyChanged` is off by the number of comment lines above it.
- **Other files.** `UserKnownHostsFile` and `GlobalKnownHostsFile` from the SSH config are not consulted.
- **A different key type.** A recorded key of another algorithm for the same host counts as "not found", not "changed". This is close to OpenSSH, but we should show the dialog differently.

Our own `known_hosts` module of about 200 lines would fix all of these. We would keep russh's hashed-host matcher idea and append lines the same way. It can be tested without a server.

**ssh2** wraps libssh2's `knownhost` API ([`src/knownhosts.rs`](https://github.com/rust-lang/ssh2-rs/blob/0a11bdd4eecf4525e540c30f14b15ab1c07194b7/src/knownhosts.rs)), which has two problems:
- **Reading stops early.** `libssh2_knownhost_readfile` stops at the first line it can't parse ([`knownhost.c` L978-1008](https://github.com/libssh2/libssh2/blob/4aded1bf2de0a7ceb02cd50b0b1f7a826984783a/src/knownhost.c#L978-L1008)).
- **Writing loses data.** `libssh2_knownhost_writefile` reopens the file for writing and dumps its in-memory list (L1210-1227). Comment lines were never read in (L931), so a write-back loses them. It is unsafe to point at the user's real file.

## 4. Reading `~/.ssh/config`

| | `ssh2-config` 0.8.2 | `russh-config` 0.58.0 | Own parser |
|---|---|---|---|
| `Include` | Yes, with glob and nested scopes. Scope bugs fixed in 0.8.1 (2026-09-28) | No | Must write |
| List every `Host` (needed for the sidebar) | `SshConfig::get_hosts()` | No. `parse(file, host)` resolves a single host | Must write |
| `Host` patterns and first-value-wins | Yes | Partial (globset) | Must write |
| `Match`, `%` tokens | No | No | Optional |
| Tied to a backend | **No.** It depends only on `dirs`, `glob`, `wildmatch` etc., not on `ssh2` | russh workspace, but last released 2026-03-18, 6 releases behind russh | n/a |

Sources:
- ssh2-config: [README, "Exposed attributes" and "Missing features"](https://github.com/veeso/ssh2-config#about-ssh2-config), [Cargo.toml](https://github.com/veeso/ssh2-config/blob/v0.8.2/Cargo.toml), [CHANGELOG](https://github.com/veeso/ssh2-config/blob/v0.8.2/CHANGELOG.md), [`src/lib.rs` `get_hosts`](https://github.com/veeso/ssh2-config/blob/v0.8.2/src/lib.rs).
- russh-config: [`russh-config/src/lib.rs`](https://github.com/Eugeny/russh/blob/v0.64.1/russh-config/src/lib.rs), which handles only `host`, `user`, `hostname`, `port`, `identityfile`, `proxycommand`, `proxyjump`, `addkeystoagent`, `userknownhostsfile` and `stricthostkeychecking`.

**Recommendation:** use `ssh2-config` with `ParseRule::ALLOW_UNKNOWN_FIELDS`, so a directive it doesn't know does not fail the whole file. Its name is misleading: it is a plain parser and works with russh. remotefs-ssh already uses it alongside russh.

**Windows catch.** `resolve_include_path` treats a path as absolute only if it starts with `\` ([`src/parser.rs` L576-603](https://github.com/veeso/ssh2-config/blob/v0.8.2/src/parser.rs#L576-L603)). So `Include C:\Users\me\.ssh\conf.d\*` or `Include C:/...` gets `%USERPROFILE%\.ssh\` put in front of it and silently matches nothing.
- `Include ~/...` and paths relative to `.ssh` work.
- The config ticket should test this case and either send a fix upstream or normalise the path first.

## 5. Running a command on the same connection

- **russh:** `Handle::channel_open_session()` then `Channel::exec(true, cmd)`. Output arrives as `ChannelMsg::Data`, `ExtendedData` and `ExitStatus`. Several channels share one connection, so SFTP and `find` can run side by side ([`client/mod.rs` L802](https://github.com/Eugeny/russh/blob/v0.64.1/russh/src/client/mod.rs#L802), [example `client_exec_simple.rs`](https://github.com/Eugeny/russh/blob/v0.64.1/russh/examples/client_exec_simple.rs)).
- **SFTP over that connection:** `request_subsystem(true, "sftp")` then `russh_sftp::client::SftpSession::new(channel.into_stream())` ([example `sftp_client.rs`](https://github.com/Eugeny/russh/blob/v0.64.1/russh/examples/sftp_client.rs)).
- **ssh2:** `Session::channel_session()?.exec(cmd)`. One libssh2 session cannot be driven from two threads at once, so SFTP and exec on the same session take turns.

## 6. Keep-alive, reconnect, cancelling a connection

**russh:**
- **Keep-alive and dead-server detection** are built in. `client::Config` has `keepalive_interval`, `keepalive_max` and `inactivity_timeout`. The session loop sends `keepalive@openssh.com` and drops the connection after `keepalive_max` unanswered pings ([`client/mod.rs` L1358-1436](https://github.com/Eugeny/russh/blob/v0.64.1/russh/src/client/mod.rs#L1358-L1436)). There is also `Handle::send_ping` for an awaited round trip.
- **Cancelling** is just dropping the future. `client::connect` / `connect_stream` are ordinary async functions, so wrapping them in `tokio::select!` with a cancel token, or in `tokio::time::timeout`, aborts at any stage: TCP, kex or auth. termscp, built on remotefs-ssh's sync API, still has an open bug for exactly this ([termscp#438](https://github.com/veeso/termscp/issues/438)).
- **Reconnect** is not a library feature in any candidate. It is our state machine: detect disconnect through `Handler::disconnected` or a failed request, then reconnect with the same credentials and reopen the SFTP subsystem.

**ssh2:**
- `set_keepalive` and `keepalive_send` exist, but the caller must call `keepalive_send` on a timer ([`session.rs` L1056-1065](https://github.com/rust-lang/ssh2-rs/blob/0a11bdd4eecf4525e540c30f14b15ab1c07194b7/src/session.rs#L1056-L1065)).
- Cancelling a blocking `handshake()` or auth means either a timeout (`set_timeout`) or closing the socket from another thread.

## 7. Building on Windows

**russh.** You must pick exactly one crypto backend: `aws-lc-rs` (the default) or `ring` ([russh `Cargo.toml`](https://github.com/Eugeny/russh/blob/v0.64.1/russh/Cargo.toml), [README "Crypto backends"](https://github.com/Eugeny/russh#crypto-backends)).

- **`ring` 0.17 from crates.io**, on `x86_64-pc-windows-msvc`:
  - Needs only "Build Tools for Visual Studio" with the C++ workload. The crate ships precompiled assembly objects, so neither NASM nor Perl is needed ([ring BUILDING.md](https://github.com/briansmith/ring/blob/main/BUILDING.md)).
  - Tauri already needs the MSVC Build Tools ([Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)), so **nothing extra to install.**
  - Feature line: `russh = { version = "0.64", default-features = false, features = ["ring", "flate2", "rsa"] }`
- **`aws-lc-rs`**:
  - Needs a C compiler and NASM. CMake is only for FIPS builds.
  - Without NASM, it can use prebuilt NASM objects if the `prebuilt-nasm` feature is enabled or `AWS_LC_SYS_PREBUILT_NASM=1` is set ([aws-lc-rs Windows requirements](https://aws.github.io/aws-lc-rs/requirements/windows.html), [aws-lc-rs `Cargo.toml`](https://github.com/aws/aws-lc-rs/blob/main/aws-lc-rs/Cargo.toml)).
  - russh's own Windows CI installs NASM with Chocolatey ([`.github/workflows/rust.yml`](https://github.com/Eugeny/russh/blob/main/.github/workflows/rust.yml)).
  - The default-features build failing on Windows is still an open issue ([russh#541](https://github.com/Eugeny/russh/issues/541)).
- **No OpenSSL anywhere.**

**ssh2 / libssh2-sys** has two Windows paths:
- **WinCNG (the default).** `build.rs` defines `LIBSSH2_WINCNG` unless the `openssl-on-win32` feature is set ([`libssh2-sys/build.rs` L102-118](https://github.com/rust-lang/ssh2-rs/blob/0a11bdd4eecf4525e540c30f14b15ab1c07194b7/libssh2-sys/build.rs#L102-L118)). The bundled libssh2 sets `#define LIBSSH2_ED25519 0` for WinCNG ([`src/wincng.h` L68](https://github.com/libssh2/libssh2/blob/4aded1bf2de0a7ceb02cd50b0b1f7a826984783a/src/wincng.h#L68)). `curve25519-sha256` kex and `ssh-ed25519` host and user keys are compiled only under `#if LIBSSH2_ED25519` ([`src/kex.c` L2865-2916](https://github.com/libssh2/libssh2/blob/4aded1bf2de0a7ceb02cd50b0b1f7a826984783a/src/kex.c#L2865-L2916)). **So the user's `id_ed25519` key cannot be used.** That rules out WinCNG.
- **OpenSSL.** Use `openssl-on-win32` with an OpenSSL from vcpkg (build.rs calls `try_vcpkg()` first), or `vendored-openssl`. The vendored build "requires a C compiler, perl (and perl-core), and make" ([openssl crate docs, Vendored](https://docs.rs/openssl/latest/openssl/#vendored)). On Windows that means installing Strawberry Perl. This is the "OpenSSL or vcpkg pain" the ticket wants to avoid.

**remotefs-ssh**:
- Its default feature is `libssh2`, which depends on `openssl`.
- The `russh` feature works, but pins `russh = "0.63"` and `russh-sftp = "2"` ([`Cargo.toml` at v1.0.0](https://github.com/remotefs-rs/remotefs-rs-ssh/blob/v1.0.0/Cargo.toml)).

## 8. Maintenance (as of 2026-10-16)

- **russh** ([Eugeny/russh](https://github.com/Eugeny/russh), formerly warp-tech/russh):
  - 1.9 k stars, 76 open issues.
  - Releases every few weeks. 0.63.0 through 0.64.1 came between 2026-08-21 and 2026-10-05, several of them security fixes ([releases](https://github.com/Eugeny/russh/releases)).
  - Pre-1.0, with breaking changes in minor versions. 0.63.0 changed the `check_server_key` signature, and 0.64.0 dropped a key parser.
  - Bus factor: Eugeny wrote 68 of the last 100 commits (since 2026-04-16). About 20 other people contributed.
  - Has a security-advisory process (GHSA entries in every recent release).
- **russh-sftp** ([AspectUnk/russh-sftp](https://github.com/AspectUnk/russh-sftp)):
  - 117 stars, 4 open issues.
  - 3.0.0 and 3.0.1 in September 2026.
  - Effectively one maintainer (AspectUnk, 173 commits; the next contributor has 3). **This is the thinnest part of the stack.**
  - Does not depend on russh (it is generic over `AsyncRead + AsyncWrite`), so russh upgrades don't force russh-sftp upgrades.
- **ssh2** ([rust-lang/ssh2-rs](https://github.com/rust-lang/ssh2-rs)):
  - In maintenance mode: 0.9.5 in 2025-02, 0.9.6 in 2026-06, 46 open issues. Recent commits are CI and build fixes.
  - The libssh2 it bundles is a master commit from 2026-08-04. libssh2's last tagged release is 1.11.1 (2024-10-16) ([libssh2 releases](https://github.com/libssh2/libssh2/releases)).
- **ssh2-config** ([veeso/ssh2-config](https://github.com/veeso/ssh2-config)):
  - 0 open issues, three releases since 2026-08-31.
  - One maintainer (veeso, also of termscp and remotefs).
- **remotefs-ssh**: 14 stars, 4 open issues, one maintainer. It lags its own dependencies.

## Risks and catches

1. **Non-UTF-8 file names break listing in russh-sftp.** Names are decoded with `String::from_utf8`, so `read_dir` on a folder with one non-UTF-8 name fails entirely ([russh-sftp#42](https://github.com/AspectUnk/russh-sftp/issues/42), open since 2024-07). This is rare on modern servers, but for a file browser it is a visible failure. Options:
   - Patch it upstream or fork, keeping names as bytes.
   - At least report "folder has an unreadable name" instead of an empty listing.
2. **One odd server reply kills the SFTP session.** An unknown `SSH_FXP_STATUS` code fails to deserialise, stops the reader task, and every later request fails with "session closed" ([russh-sftp#102](https://github.com/AspectUnk/russh-sftp/issues/102), opened 2026-10-14). Our connection layer must treat "SFTP session closed" as reconnectable.
3. **Request timeout under load.**
   - Each request's 10 s timeout (`request_timeout_secs`) starts when it is queued, not when it is sent. Busy concurrent transfers can therefore time out at random ([russh-sftp#95](https://github.com/AspectUnk/russh-sftp/issues/95)).
   - Set a larger timeout for transfer sessions, and rely on SSH keep-alive to detect dead links.
4. **A stalled channel can freeze the whole SSH connection** ([russh#789](https://github.com/Eugeny/russh/issues/789), opened 2026-10-15 with a proposed fix, affects 0.62.5 to 0.64.1).
   - **How it happens:** a fast writer on a slow link gets a channel's window counted twice. While that channel holds pending data, nothing else on the connection is sent: no other channel's data, no opens, not even `disconnect`. If the peer then closes that channel, the stall is permanent.
   - **Mitigation:**
     - Give the transfer queue its own SSH connection(s), separate from the browsing connection of the Tab.
     - Have keep-alive failure trigger a reconnect.
     - Watch for the fix in 0.64.x.
5. **`client::Handler::adjust_window` has no effect on the client** ([russh#770](https://github.com/Eugeny/russh/issues/770)). The client's receive window comes only from `Config::window_size`. Set it there if downloads need a bigger window.
6. **Breaking changes in russh minor versions.** Pin `russh = "~0.64"` and plan upgrades on purpose. The release notes call out every breaking change.
7. **The crypto backend choice is ours.** If another dependency (for example a `rustls`-based HTTP client in an updater plugin) pulls in `aws-lc-rs`, NASM or `prebuilt-nasm` comes back into play anyway. Check `cargo tree -i aws-lc-sys` when adding dependencies.

## Later features: what they would need

- **`ProxyJump`:**
  - **russh:** works natively. Open `channel_open_direct_tcpip(host, port, ..)` on the jump connection, convert it with `Channel::into_stream()`, and pass that to `client::connect_stream` for the target ([`client/mod.rs` L845, L1166](https://github.com/Eugeny/russh/blob/v0.64.1/russh/src/client/mod.rs#L845)). remotefs-ssh 1.0 added ProxyJump support on its russh backend ([CHANGELOG](https://github.com/remotefs-rs/remotefs-rs-ssh/blob/v1.0.0/CHANGELOG.md)), which shows the approach works.
  - **ssh2:** awkward. `Session::set_tcp_stream` requires a real OS socket (`AsRawSocket`) ([`session.rs` L324](https://github.com/rust-lang/ssh2-rs/blob/0a11bdd4eecf4525e540c30f14b15ab1c07194b7/src/session.rs#L324)), so a jump channel has to be bridged through a loopback socket and a pump thread.
- **ssh-agent and Pageant:**
  - **russh:** both are built in on Windows. `AgentClient::connect_named_pipe(r"\\.\pipe\openssh-ssh-agent")` talks to Windows OpenSSH's agent, and `AgentClient::connect_pageant()` talks to Pageant ([`keys/agent/client.rs` L90-110](https://github.com/Eugeny/russh/blob/v0.64.1/russh/src/keys/agent/client.rs#L90-L110)). `authenticate_publickey_with(user, key, hash_alg, &mut agent)` signs through the agent. The `pageant` crate is a Windows-only dependency of russh already. A Pageant out-of-bounds read was fixed in 0.63.2 (GHSA-g4mp-vgx3-xrvm).
  - **libssh2:** has Pageant and Windows OpenSSH agent backends too ([`src/agent.c`](https://github.com/libssh2/libssh2/blob/4aded1bf2de0a7ceb02cd50b0b1f7a826984783a/src/agent.c)).
- **PuTTY `.ppk` keys:** russh already parses them (see §2).

## Not considered further

- **libssh (`libssh-rs`).** It is the source of termscp's 200 KB/s regression, and its vendored build needs OpenSSL (remotefs-ssh's `libssh-vendored` feature enables `vendored-openssl`).
- **`openssh` / `openssh-sftp-client`.** The `openssh` crate controls the system `ssh` binary through ControlMaster multiplexing, which Windows OpenSSH does not support. Driving `ssh.exe -s sftp` over pipes would give up control of the host key prompt, passphrase prompt and cancellation, which v1 needs in-app.
- **`async-ssh2-tokio`.** A thin convenience wrapper over russh for exec. It adds nothing for SFTP.
