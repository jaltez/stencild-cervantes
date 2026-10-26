# stencild-cervantes

Turn a **bq Cervantes 2** (E60M00 / Marvell M166E EPD) e-reader into a
network-driven **800×600 e-ink display panel** — without touching the
device's NAND.

Two binaries:
- **`stencild`** — host-side interface service: HTTP API, inbox watcher,
  serial transport to the device daemon (`inkd`), installer artifacts,
  self-update, and a resident tray app on Windows (pause/resume, clear to
  boot image, update, live status). Runs on Linux and Windows.
- **`sdgen`** — builds the init SD card tree (FAT32 card content) that turns
  the reader into a panel at boot.

```mermaid
flowchart LR
    A["app / script / cron<br/>(any image producer)"] -- "HTTP POST /image" --> S[stencild]
    S -- "EPD1 serial 115200" --> I[inkd on the reader]
    I -- "display ioctl" --> P[800x600 e-ink panel]
```


The reader boots from the SD card in panel mode (stock MFG hook), enumerates
as a USB CDC-ACM serial port, and `inkd` serves frames; pull the card and
the stock reader is exactly as it was.

## Quick start

```sh
cargo build --release
cargo test --release
./target/release/sdgen build --out /tmp/sdtest --size 64M   # inspect the SD tree
./target/release/stencild --listen 127.0.0.1:8787           # serve (device autodetected)
./target/release/stencild --no-device --frame-out /tmp/f.bin # dry-run, no hardware
cargo build --release --target x86_64-pc-windows-gnu --features tray   # Windows tray exe
stencild.exe tray                                              # resident: tray + HTTP + pause/clear/update
```

Push your first image (works in dry-run mode too):

```sh
python3 examples/btc_chart_demo.py   # renders examples/candles.json and POSTs it
# or, with any image at hand:
curl -X POST --data-binary @picture.png http://127.0.0.1:8787/image
```

## Documentation

| Doc | Contents |
|---|---|
| [Architecture](docs/architecture.md) | How the whole system works: components, boot chain, EPD1 protocol, internals |
| [Service API](docs/service-api.md) | HTTP endpoints, inbox, EPD1 wire protocol |
| [Apps guide](docs/apps-guide.md) | Building content producers; rendering rules; full BTC-chart example |
| [SD guide](docs/sd-guide.md) | Building and using the init SD; recovery back to reader mode |
| [CLI reference](docs/cli.md) | Every command and flag of `stencild` and `sdgen` |

## Repository layout

```
crates/stencild/    host service (HTTP + serial + inbox + update + installer)
crates/sdgen/       init-SD tree generator
assets/firmware/    device-side sources + builds (inkd, epdraw) and kernel modules
assets/scripts/     SD boot runner and orders (executed on the reader)
assets/templates/   boot image, HTML/Python conversion helpers
examples/           runnable demo app (BTC chart) + sample data
docs/               the guides above
```

## Requirements

- Host: Rust 1.70+ (stable). Linux or Windows. A free USB port.
- Device: bq Cervantes 2 (E60M00), stock firmware. A FAT32 SD card ≥ 64 MB.
- Hardware is optional for development: `--no-device --frame-out` runs the
  full conversion pipeline with no panel attached.

## License

Project code is [MIT](LICENSE) © 2026 Javier Altez. The prebuilt kernel
modules in `assets/firmware/` are GPL-2.0 (Linux 2.6.28 upstream sources,
linked in [THIRD_PARTY.md](THIRD_PARTY.md)); dependency licenses are also
summarized there.
