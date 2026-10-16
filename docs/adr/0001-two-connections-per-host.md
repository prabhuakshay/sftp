# Two SSH connections per Host

Each connected Host gets one **browsing connection**, shared by every Tab on that Host and also used for Run command and `find`, and one separate **transfer connection**, opened on demand for Transfers, Edit uploads and drag-out reads. The split exists because of russh#789, where one stalled channel can freeze a whole connection: a slow or stuck Transfer must never freeze browsing. To keep the second login silent, the app holds each Host's decrypted key or password in memory until it quits. Keyboard-interactive (2FA) Hosts are the exception: their one-time codes can't be reused, so they prompt again.

## Considered Options

- **One connection per Tab**: isolates Tabs, but every new Tab costs a login and, on 2FA Hosts, a prompt.
- **One connection per running transfer**: isolates Transfers from each other, but costs up to 3 extra logins per Host.
- **Everything on one connection**: a single login, but a stuck Transfer freezes browsing.
