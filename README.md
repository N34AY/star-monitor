# Starlink Manager (Tauri + Vue)

Desktop app for Starlink dish monitoring and control.

This app now communicates directly with the dish over gRPC (`Device.Handle`) and does not depend on an external exporter service.

## Features

- Direct dish connection (default: `http://192.168.100.1:9200`)
- Telemetry sections for status, diagnostics, history, connections, speedtest, and config endpoints
- Optional location and GNSS queries
- Control actions:
	- Reboot
	- Stow / Unstow
	- Start speedtest
	- GPS inhibit on/off
	- RF inhibit on/off

## Development

### Prerequisites

- Node.js + npm
- Rust toolchain (`rustup`)
- Platform dependencies required by Tauri

### Run

```bash
npm install
npm run tauri dev
```

## Build Release Bundles

Use this command in the project root:

```bash
npm run tauri build
```

Output goes to:

`src-tauri/target/release/bundle`

### Cross-platform note

Native desktop bundles should be built on their target OS.

- macOS build on macOS
- Linux Mint build on Linux
- Windows build on Windows

### Typical outputs

- macOS: `.app`, `.dmg`
- Linux: `.deb`, `.rpm`, `.AppImage` (depends on installed packaging tools)
- Windows: `.msi`, `.exe`
