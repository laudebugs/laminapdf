# Third-party crates carried with a patch

Each folder here is a crates.io release copied whole, with a patch Lamina PDF needs before upstream releases it. The workspace's `[patch.crates-io]` points Cargo at the copy. Outside the workspace (`exclude`), it escapes the workspace's lints. Delete a crate's copy, and its `[patch.crates-io]` entry, once a release carries its patch.

| Crate | Release | Patch | Why |
| --- | --- | --- | --- |
| `minidumper` | 0.11.0 (MIT OR Apache-2.0, [EmbarkStudios/crash-handling](https://github.com/EmbarkStudios/crash-handling)) | `src/ipc/windows.rs`: `UnixStream::peer_pid` (`SIO_AF_UNIX_GETPEERPID`). `src/ipc/server.rs`: a Windows dump request whose `process_id` isn't the connection's peer, or whose peer can't be named, is refused before any process is opened: the server logs it and drops that client, where the Linux server already checks the peer. | The desktop app's crash monitor (D7f, ADR 0014 §10): the main process's dump server on Windows. Without the patch, a client could have any of the user's processes dumped. |

To see the patch: `diff -ru ~/.cargo/registry/src/*/minidumper-0.11.0 third_party/minidumper` (the copy drops the crate's `Cargo.lock`, `release.toml` and `.cargo_vcs_info.json`).
