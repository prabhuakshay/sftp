# PROTOTYPE: drag-out test build

Throwaway. It answers one question for the ticket
[Drag-out to Explorer test build](https://github.com/prabhuakshay/sftp/issues/14):
does dragging remote files out to Windows Explorer, using our own `IDataObject`
with virtual files (`CFSTR_FILEDESCRIPTORW` + `CFSTR_FILECONTENTS`), work well
enough to keep in v1? It lives on the `prototype/drag-out` branch and is never merged.

## Run it

1. Get a test server. On any Debian/Ubuntu machine (WSL works), as root:

   ```sh
   ./setup-server.sh <user> <public-key-file>
   mkdir -p /run/sshd && /usr/sbin/sshd -D -p 2222
   ```

   This installs OpenSSH, authorises the key, and fills `~<user>/dragtest` with
   test files: a small file with an old modified time, 200 MB and 2 GB random
   files, a nested folder tree, a tree deeper than 260 characters, and a folder
   of 100,000 entries.

2. Start the app: `npm install`, then `npm run dev`.
3. Enter the host, port 2222, user and private key path (an unencrypted key),
   then click **Connect**. It opens `~/dragtest`.

## What's on screen

- **Settings row.** Throttle (simulates a slow link), fail after N MB
  (simulates a failure mid-download), simulate stuck drag (ignores the mouse
  release so the watchdog has to step in), where the `IStream` lives (its own
  MTA thread, or the main UI thread for comparison), and how the drag starts.
- **Meters.** Main-thread round trip and the longest JS frame gap. Red means
  the window stalled for more than 100 ms.
- **Streams.** One row per file Explorer pulls, with bytes served and speed.
- **Log.** Every COM call Explorer makes, and which thread it ran on.
- **Drop connection.** Disconnects SSH, for a real mid-download failure.

## Checks

1. Drag `small.txt` out to an Explorer folder.
2. Throttle to 2,000 KB/s and drag `big-200M.bin`. Watch the meters. Then
   repeat with the stream on the main UI thread to see the difference.
3. Drag the `tree` folder, and the `deep` folder (deeper than 260 characters).
4. Check the modified times in Explorer (`small.txt`: 2001-02-03; `tree`: 2010-06-07).
5. Set fail after 50 MB and drag `big-200M.bin`. Then try **Drop connection** halfway.
6. Tick simulate stuck drag, drag a file, and release. The watchdog should
   cancel the drag after the set number of seconds.
7. Open `many` (100k entries) and read the listing timings.

## Results

`run-2026-10-16.log` is the full log of the first run against WSL Ubuntu. The
verdict and findings are on the ticket
[Drag-out to Explorer test build](https://github.com/prabhuakshay/sftp/issues/14).
