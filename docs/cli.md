# CLI reference

## stencild

```
stencild [OPTIONS] [COMMAND]
```

Default command: `serve`.

### Options (global)

| Flag | Default | Description |
|---|---|---|
| `--listen <ADDR>` | `127.0.0.1:8787` | HTTP listen address |
| `--device <NAME>` | `auto` | Serial device: `auto`, `COM13`, `/dev/ttyACM0` |
| `--inbox <DIR>` | — | Watch DIR for new images; archive to sibling `done/` |
| `--waveform <N>` | `2` | Default waveform (0 INIT, 1 MU, 2 GU, 3 GC, 4 NOP) |
| `--no-device` | — | Never open a serial port (dry-run) |
| `--frame-out <FILE>` | — | Write each EPD1 frame to FILE instead of the port |
| `--tray-selftest` | — | Validate tray icon assets and exit |
| `--version`, `--help` | | |

### `serve`

Runs the HTTP service. Autodetection probes serial ports with a PING;
if nothing answers, continues offline (`/health` shows `online:false`).

### `tray`

Runs the HTTP service with a resident **tray icon** (Windows builds with the
`tray` feature). The process detaches from its console, so it survives the
launching terminal closing and works as a plain double-click target (the
console window flashes once and disappears).

Menu: live status line (refreshes every 2 s, doubles as tooltip),
Pause/Resume (paused pushes fail fast with `paused`), Clear panel (repaints
the boot image), Restart (respawns a detached copy with the same
arguments and exits), Check for update… (applies `manifest.json` found
next to the exe, then auto-restarts), Quit. Bind failures show a message
box instead of dying silently. Non-Windows builds run headless like
`serve`.

```sh
cargo build --release --target x86_64-pc-windows-gnu --features tray
stencild.exe installer --out dist-win          # exe + install.ps1
powershell -File dist-win/install.ps1          # → %LOCALAPPDATA%\stencild\bin
stencild.exe tray --device COM13               # or make a desktop shortcut of this
```

### `installer --dry-run --out <DIR>`

Writes installer artifacts to DIR: the binary, `install.sh` (POSIX),
`install.ps1` (Windows), and `tray-icon.png`. Never executes anything.
`sh DIR/install.sh --prefix <P>` installs to `P/bin/stencild`.

### `update --manifest <URL> [--apply] [--prefix <P>] [--selftest]`

Self-update from a local manifest (`file:///...` or absolute path):

```json
{"version":"0.2.0","url":"file:///path/to/binary","sha256":"<hex>"}
```

- Without `--apply`: validates and reports the available version.
- With `--apply`: verifies sha256, replaces `<prefix>/bin/stencild` atomically
  (skips when the manifest version equals the running version).
- With `--selftest`: executes the replaced binary's `--version` and requires
  the manifest version in its output.

Only local `file://` URLs are accepted.

On Windows the target is `stencild.exe`; a running executable is parked as
`stencild.exe.old` before the replacement moves in (Windows locks running
exes against replacement, not against rename).

## sdgen

```
sdgen build --out <DIR> [--size <SIZE>]
```

Writes the init SD card tree into DIR (see [SD guide](sd-guide.md)).
`--size` (default `2G`) is recorded in `SD_SIZE.txt`; no disk image is
created and no device is touched.
