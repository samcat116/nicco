# nicco

Early Rust foundation for a Linux network CLI and TUI. The current slice reads
interface/address status through rtnetlink; it does not change networking.
NetworkManager/networkd runtime-directory evidence is shown with ownership unknown.
ethtool diagnostics, backend configuration, and destination troubleshooting are deferred.
The project aims for safe configuration and consistent CLI/TUI behavior; these are
goals, not completed features or a firm roadmap.

Requires Rust 1.98.1. Live observations require Linux; labeled demo mode works on
other platforms. Build with the pinned toolchain and committed lockfile:

```sh
cargo build --workspace --locked
cargo run --locked -p nicco -- --help
cargo run --locked -p nicco -- status
cargo run --locked -p nicco -- status --json
cargo run --locked -p nicco -- tui
cargo run --locked -p nicco -- tui --demo
```

TUI: arrows/j/k select, `r` refreshes, q/Esc/Ctrl-C quits. Demo data is synthetic
and never observes the host. Status is a best-effort snapshot, not a connectivity
verdict. Commands need no elevated privileges and perform no network writes.

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

See [architecture and scope](docs/architecture.md) for JSON v1, lifecycle, platform
limitations, and deferred work. Licensed under the existing [MIT license](LICENSE).
