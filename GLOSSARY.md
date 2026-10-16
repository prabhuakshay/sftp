# sftp

A Windows desktop file browser for the user's own servers, reached over SSH. It exists so remote filesystems can be browsed and managed as easily as local folders.

## Language

**Host**:
A server the app can connect to, defined by a `Host` entry in the user's SSH config and named by its alias. The app reads Hosts but never edits the SSH config.
_Avoid_: Site, server profile, connection

**Bookmark**:
A folder on a Host that the user has saved for quick return, listed under that Host in the sidebar. Bookmarks belong to the app, not the SSH config.
_Avoid_: Favourite, pinned folder, shortcut

**Quick Connect**:
Connecting to a server typed as `user@host[:port]` that has no Host entry. Nothing about it is saved.
_Avoid_: Ad hoc connection, temporary site

**Edit**:
A remote file the user has opened in their own editor. While the Edit lasts, every save to its local copy is uploaded back to the Host. Edits belong to the app, not to a Tab.
_Avoid_: Edit session, checkout, temp file, sync

**Batch**:
The files moved by one upload, download or drop. Conflicts are answered and failures are summarised per Batch.
_Avoid_: Job, task, transfer group

**Tab**:
One browsing view showing a single Host at a current remote folder. Several Tabs may show the same Host.
_Avoid_: Session, window, pane
