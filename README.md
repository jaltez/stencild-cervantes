# stencild-cervantes

Interface service and init-SD generator for **Stencild** e-ink panels
(bq Cervantes 2, E60M00 / Marvell M166E EPD, 800×600 landscape DC).

Two binaries:

- **`stencild`** — host-side interface service: HTTP API, inbox watcher,
  serial transport to the device daemon (`inkd`), installer artifacts,
  self-update, optional tray icon. Runs on Linux and Windows.
- **`sdgen`** — builds the init SD card tree (FAT32 card content) that turns
  the reader into a panel at boot.

## Quick start

```sh
cargo build --release
cargo test --release
./target/release/sdgen build --out /tmp/sdtest --size 64M   # inspect the SD tree
./target/release/stencild --listen 127.0.0.1:8787           # serve (device autodetected)
./target/release/stencild --no-device --frame-out /tmp/f.bin # dry-run, no hardware
```

## Documentation

- [Apps guide](docs/apps-guide.md) — how to build apps that render for the panel
  (HTML/JS libraries, image measures, 4bpp packing, refresh times, caveats)
- [SD guide](docs/sd-guide.md) — producing and using the init SD; reader-mode recovery
- [Service API](docs/service-api.md) — HTTP endpoints, inbox, EPD1 wire protocol
- [CLI reference](docs/cli.md) — every command and flag of `stencild` and `sdgen`

## Architecture

```
app (any producer)  --HTTP PNG-->  stencild (this service)  --EPD1 serial-->  inkd (device daemon)
```

`inkd` and the kernel modules shipped in `assets/firmware/` were validated on
hardware; they are embedded into `sdgen` and deployed by the SD tree.
