# Read-only foundation

`nicco-core` owns typed observations, deterministic ordering, a synthetic fixture,
errors, and the Linux rtnetlink adapter. `nicco` owns Clap parsing, human/JSON
output, and Ratatui/Crossterm rendering and input. No networking writes are exposed.
Tokio drives collection; no tracing dependency is needed until useful diagnostics
can be added without leaking observed addresses by default.

The adapter issues link/address GET dumps in the current network namespace with
a five-second timeout. Connection tasks abort on completion, failure, or cancellation.
Link and address dumps are not atomic: interfaces may change between them; addresses
for disappeared interfaces are discarded. This is an observation, not a configuration
inventory or connectivity verdict. Operational state differs from administrative
state. Point-to-point IPv4 prefers the local address over the peer address.

NetworkManager and networkd detection currently checks their runtime directories.
This is limited evidence, not a service-health, API-capability, or ownership check.
Every interface owner is unknown. No daemon is started, subprocess invoked,
configuration read, address probed, or packet captured. Definitive ownership and
backend capabilities require future read-only backend adapters with explicit evidence.

## JSON v1

Successful JSON output is one envelope, with `schema_version: 1`, `source: live`
or `demo`, `interfaces`, and `backends`. Addresses are IP strings with integer
`prefix_len`; ownership unknown is JSON null. The demo fixture is an exact contract
test. Consumers should ignore additional fields and reject unsupported major schema
versions. Future breaking field/semantic changes need a new version. Operational
state strings are extensible kernel observations, not a fixed enum.

CLI argument errors exit 2; observation/I/O/terminal errors exit 1 and go to stderr;
success exits 0. Broken stdout pipes exit successfully. Errors do not emit partial
JSON observations. Demo addresses use documentation ranges and never inspect the host.

## TUI lifecycle

The TUI checks stdin/stdout terminals and collects before setting raw mode. A guard
restores the screen/cursor and raw mode on setup failures, errors, normal quit, and
panic unwind. Cleanup attempts both operations even if one fails. SIGKILL, process
abort, power loss and external signals that terminate without unwinding cannot be
guaranteed to restore a terminal. Raw-mode Ctrl-C is handled as a key for normal quit.
Refresh is manual and may block input for at most the five-second collection timeout;
failure retains the prior snapshot and labels it stale. Resize redraws; long detail
text is clipped to the pane in this initial implementation. No automatic probes occur.

## Toolchain and validation

Initial MSRV/toolchain is Rust **1.98.1**, matching the verified development toolchain;
no lower MSRV support is claimed. `rust-toolchain.toml` pins it, direct dependencies
have exact versions, and the application commits Cargo.lock. Use `--locked` builds.
Only official crates.io packages are used. Formatting, Clippy, fixture/contract,
CLI errors, input, resize and injected terminal lifecycle tests run on Linux CI;
CI also runs a read-only status smoke test without elevated privileges. macOS tests
exercise the unsupported live-platform path and demo rendering, not Linux networking.

## Deferred

ethtool driver/link diagnostics; NetworkManager/networkd APIs and writes;
safe configuration plans/rollback; routing/DNS/destination troubleshooting;
privilege handling; persistence; releases/package publication. These are goals,
not implemented functionality or a committed roadmap. The foundation never makes
network changes or automatic fixes and requires neither sudo nor capabilities.
