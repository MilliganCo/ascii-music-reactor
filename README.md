# ASCII Music Reactor

Windows system-audio loopback visualizer in Rust, with ASCII scenes and in-memory audio analysis.
Captured audio is processed in RAM; source review found no recording/upload/telemetry path.

## Build and check

```sh
cargo check --locked --all-targets
cargo test --locked --lib audio::queue::tests
cargo build --locked --release
```

Windows and a Rust/MSVC toolchain are required for WASAPI/window capture. See DEVELOPMENT.md for
architecture. The window/TUI applications were not launched for publication checks.

The capture producer now drops a new chunk when its bounded queue is full, and stops when the
consumer disconnects. This removes the known blocking-send path during producer shutdown.
It intentionally favors current visual responsiveness over retaining every audio chunk.


See [VALIDATION.md](VALIDATION.md) for actual publication checks.
