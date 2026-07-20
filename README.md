![RaveEngine](repo/rave.png)

# RaveEngine-Game

RaveEngine is a game engine and toolchain built with Rust and Bevy, featuring an integrated world editor, a networked game client, and an authoritative headless server.

## Components

| Binary | Description |
|---|---|
| **RaveEngineStudio** | Graphical world editor with egui UI, physics simulation, Lua scripting, and in-editor networked playtesting |
| **RaveEngineClient** | Rendered game client with character animation, prediction/interpolation, and full game UI |
| **RaveEngineServer** | Headless authoritative server with physics, replication, Lua scripting, and player management |

## Prerequisites

- Rust stable (install via [rustup](https://rust-lang.org/tools/install/))
- Windows: Visual Studio Build Tools or equivalent
- Linux: `libudev-dev`, `libasound2-dev`

## Building

```bash
cargo build --release
```

Then launch any of the three binaries from the project root (the `assets/` directory must be in the working directory):

```bash
./target/release/RaveEngineStudio
./target/release/RaveEngineClient --ip 127.0.0.1 --port 5000
./target/release/RaveEngineServer --port 5000 --map assets/maps/temp_playtest.vrtx
```

## Project Format

Projects use the `.vrtx` binary format (version 7). Legacy Godot GCPF files are also supported for import.

## CI & Quality

Continuous integration runs on push and PR:
- `cargo check --all-targets --all-features --locked`
- `cargo fmt --check`
- `cargo clippy --all-targets --all-features --locked -- -D warnings`
- `cargo test --all-targets --all-features --locked`
- `cargo audit`

## License

MIT
