#!/usr/bin/env bash
# PROTOTYPE: test server for the drag-out test build. Throwaway.
#
# Installs OpenSSH on a Debian/Ubuntu box (WSL, VM or container), authorises
# one public key for USER, and creates the test files under ~USER/dragtest.
# Run as root:  sudo ./setup-server.sh <user> <public-key-file>
# Then start the server in the foreground on port 2222:
#   mkdir -p /run/sshd && /usr/sbin/sshd -D -p 2222
set -euo pipefail

user="$1"
pubkey="$2"
home="$(getent passwd "$user" | cut -d: -f6)"
root="$home/dragtest"

if ! command -v sshd >/dev/null; then
  apt-get update -q
  DEBIAN_FRONTEND=noninteractive apt-get install -y -q openssh-server
fi
mkdir -p /run/sshd

install -d -m 700 -o "$user" -g "$user" "$home/.ssh"
grep -qxF "$(cat "$pubkey")" "$home/.ssh/authorized_keys" 2>/dev/null \
  || cat "$pubkey" >> "$home/.ssh/authorized_keys"
chown "$user:$user" "$home/.ssh/authorized_keys"
chmod 600 "$home/.ssh/authorized_keys"

mkdir -p "$root"
cd "$root"

# 1. One small file, with an old modified time to check FD_WRITESTIME.
echo "hello from the drag-out test server" > small.txt
touch -d '2001-02-03 04:05:06' small.txt

# 2. Large files. Random bytes so nothing along the way can compress them.
[ -f big-200M.bin ] || head -c 200M /dev/urandom > big-200M.bin
[ -f big-2G.bin ] || head -c 2G /dev/urandom > big-2G.bin

# 3. A folder tree: nested folders, old mtimes, an empty folder, odd names.
rm -rf tree
mkdir -p tree/a/b/c tree/empty "tree/spaces and ünïcödé"
for d in tree tree/a tree/a/b tree/a/b/c "tree/spaces and ünïcödé"; do
  for i in 1 2 3; do head -c $((i * 40000)) /dev/urandom > "$d/file-$i.bin"; done
done
echo "note" > "tree/spaces and ünïcödé/naïve résumé.txt"
find tree -exec touch -d '2010-06-07 08:09:10' {} +

# 4. A tree deeper than Windows' 260-character MAX_PATH once it lands in Explorer.
rm -rf deep
p=deep
for i in $(seq 1 12); do p="$p/folder-with-a-fairly-long-name-$i"; done
mkdir -p "$p"
echo "deep" > "$p/deep-file.txt"

# 5. A folder with 100k entries for the listing measurement.
if [ ! -d many ] || [ "$(ls many | wc -l)" -ne 100000 ]; then
  rm -rf many && mkdir many
  python3 -c "
import os
for i in range(100000):
    open(f'many/entry-{i:06d}.txt', 'w').close()
"
fi

chown -R "$user:$user" "$root"
echo "Test files ready in $root"
