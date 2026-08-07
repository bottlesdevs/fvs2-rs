# fvs2-rs

Rust gRPC client bindings for [`fvs2d`](https://github.com/fvs-lab/fvs2d), the
FUSE-based versioning daemon used by Bottles Next to snapshot and restore
Wine prefixes.

- `client` — `Fvs2dClient`, an async client over the `fvs2d.v1` gRPC service
  (mount/unmount, commit, restore, layer management).
- `error` — client-side error types.
- `Layer`/`Repository`/`Commit`/`CommitSummary`/`Mount`/`Progress` — re-exported
  protobuf message types generated from `fvs2d`'s `.proto` definitions.

The `upstream` directory is a git submodule pointing at the `fvs2d` daemon
itself; `build.rs` compiles the protobuf definitions it provides.
