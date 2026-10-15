# deevnet-filesync

Custom cloud file synchronization system that enables full end-user control and local-first
priority.

- **Docs:** https://deevnet.github.io/deevnet-filesync/ (source in `docs/`, a Hugo site)
- **Code:** a Rust workspace in `crates/`:
  - `filesync-core`: sync engine, encryption, hub format, storage backends
  - `filesync-cli`: the `filesync` command

The code is currently the [Phase P prototype](https://deevnet.github.io/deevnet-filesync/docs/development/prototype/).

```bash
cargo build --release    # target/release/filesync
cargo test
make -C docs server      # docs at http://localhost:1313/deevnet-filesync/
```
