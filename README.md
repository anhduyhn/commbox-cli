# commbox-cli

A command-line tool for managing CommBox interactive panels over their TCP control protocol. Built for school IT staff managing fleets across multiple campuses.

Currently supports status queries, volume setting, and input switching. Targets single panels, named groups, or the entire fleet, with all operations fanning out concurrently.

## Quick start

1. `cargo install --path .`
2. `cp panels.txt.example panels.txt` and edit with your panel addresses
3. `commbox-cli`

## What it does

```
$ commbox-cli
? What would you like to do?
> Status
  Volume
  Input
  Quit

? How to specify panel(s)?
  Type IP
  Pick from list
  All panels in group
> All panels (every group)

⠋ Querying 8 panels...

BA-CBM-LIB (192.168.1.50)
  Power:  ON
  Volume: 60
  Mute:   OFF
  Input:  HDMI 1 (Vivi)

BA-CBM-STAFF (192.168.1.51)
  Power:  ON
  Volume: 45
  Mute:   OFF
  Input:  Android

BB-CBM-A1 (192.168.2.50)
  Power:  OFF
  ...
```

All panel operations run concurrently, so wall time is bounded by the slowest panel rather than the panel count. A 30-panel status sweep is about as fast as a single-panel query.

## Why this exists

CommBox panels expose a plaintext AV control protocol on TCP/4660.

This binary replaces a PowerShell utility previously used to manage panels across multiple campuses in a school deployment. The Rust version ships as a single sub-2MB binary, runs cross-platform, and handles fleet-wide concurrent operations cleanly rather than serially.

## Implementation status

- `Status` query: power, volume, mute, input state across one or many panels
- `Volume` set: 0-100 level on one or many panels
- `Input` switch: HDMI 1-4, DisplayPort, USB-C, OPC, Android, VGA, AV
- Mute, power, freeze set: planned, same shape as the existing setters

## Build and install

Requires a Rust toolchain. `rustup` is the easiest way to get one.

```bash
git clone https://github.com/anhduyhn/commbox-cli.git
cd commbox-cli
cargo install --path .
```

This installs `commbox-cli` to `~/.cargo/bin/`, which should already be on your PATH after rustup setup.

To cross-compile a Windows binary from Linux or WSL:

```bash
rustup target add x86_64-pc-windows-gnu
sudo apt install mingw-w64    # if not already present
cargo build --release --target x86_64-pc-windows-gnu
```

Binary lands at `target/x86_64-pc-windows-gnu/release/commbox-cli.exe`.

## Configuration

The tool reads a `panels.txt` file from the working directory by default. Override with `--panels-file path/to/file`.

A working starter file is committed as `panels.txt.example`. Copy it to `panels.txt` and edit:

```bash
cp panels.txt.example panels.txt
```

Format:

```
[Building A]
192.168.1.50      # BA-CBM-LIB
192.168.1.51      # BA-CBM-STAFF

[Building B]
192.168.2.50      # BB-CBM-A1

[Building C]
192.168.3.50      # BC-CBM-MAIN
```

Format rules:

- `[Group]` headers organise panels into named sections used for "All panels in group" mode. Casing is preserved in the menu display.
- IP per line, with an optional `# name` trailing comment that becomes the display name in interactive prompts.
- Blank lines and full-line comments are ignored.
- Panels listed before any header land in a `default` group.
- Non-default ports can be set inline: `192.168.1.50:5000`.

The file is optional. Without it, only the "Type IP" mode is available; the binary still runs.

## Architecture

Single-binary Cargo project, split into focused modules:

```
src/
├── main.rs           # entry, CLI args, interactive loop, action dispatch
├── protocol.rs       # raw TCP transport, FrameOutcome, timeouts
├── panels.rs         # panels.txt parser, PanelEntry
├── decode.rs         # human-readable conversion of protocol responses
└── interactive.rs    # inquire-based prompts
```

The transport layer is fully async on tokio. Fan-out happens at two levels:

- Within a single panel, the four status queries run concurrently via `futures::join_all`.
- Across multiple panels, all panel operations also run concurrently.

A 30-panel status sweep does 120 in-flight TCP queries on a single OS thread.

Each transport call returns a `FrameOutcome` enum with five variants (`ResponseOk`, `Sent`, `Locked`, `PanelError`, `Fail`) rather than a `Result<String, Error>`, because panel-level outcomes don't fit naturally into a binary success/failure split. The compiler enforces exhaustive handling at the call site.

## Dependencies

- `tokio` for the async runtime and TCP
- `clap` for CLI argument parsing (derive macro)
- `inquire` for interactive prompts
- `indicatif` for the progress spinner during fleet operations
- `futures` for `join_all` fan-out
- `anyhow` for error handling in the binary

## Protocol notes

CommBox panels listen on TCP/4660 and accept ASCII frames in the format:

```
!000<CMD> <VALUE>\r
```

`!000` is the panel address (a daisy-chain protocol artefact). `<CMD>` is a 4-character code: `POWR`, `VOLM`, `MUTE`, `INPT`, `FREZ`, `MDLN`, `SERN`, `FWVR`. `<VALUE>` is either a setpoint (`50`, `1`, `211`) or `?` to query current state.

Successful query responses come back as `!000<CMD>=<VALUE>\r`. Set commands often don't echo a response, which is treated as silent success rather than failure. Error responses contain `ERR4` for "panel locked or wrong panel ID", or other `ERR` substrings for generic errors.

Connect timeout 2000ms, read timeout 800ms. Each command opens a fresh TCP connection; the protocol does not require connection persistence.

## Licence

MIT
