# Third-party crates carried with a patch

Each folder here is a crates.io release copied whole, with a patch Lamina PDF needs before upstream releases it. The workspace's `[patch.crates-io]` points Cargo at the copy. It isn't a workspace member, so the workspace's lints don't apply to it; `exclude` keeps a future glob from making it one. Each one goes once a release carries its patch.

| Crate | Release | Patch | Why |
| --- | --- | --- | --- |
| `minidumper` | 0.11.0 (MIT OR Apache-2.0, [EmbarkStudios/crash-handling](https://github.com/EmbarkStudios/crash-handling)) | `src/ipc/windows.rs`: `UnixStream::peer_pid` (`SIO_AF_UNIX_GETPEERPID`). `src/ipc/server.rs`: a Windows dump request whose `process_id` isn't the connection's peer is refused (`Error::UnknownClientPid`) before any process is opened, as the Linux server already checks. | The desktop app's crash monitor (D7f, ADR 0014 §10): without it a client could have any of the user's processes dumped. |

To see the patch: `diff -ru ~/.cargo/registry/src/*/minidumper-0.11.0 third_party/minidumper` (the copy drops the crate's `Cargo.lock`, `release.toml` and `.cargo_vcs_info.json`).
