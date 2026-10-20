# Third-party components and licenses

The project's own code (Rust crates, C firmware sources, shell scripts,
Python tools, templates, docs) is [MIT](LICENSE). This file lists everything
that is not original to this repository.

## Prebuilt Linux kernel modules — GPL-2.0

`assets/firmware/g_serial.ko` and `assets/firmware/g_ether.ko` are prebuilt
Linux kernel USB gadget modules, compiled for the bq Cervantes 2 kernel
(`vermagic: 2.6.28 mod_unload ARMv5`):

| File | Upstream source | Authors |
|---|---|---|
| `g_serial.ko` | `drivers/usb/gadget/g_serial.c` (CDC-ACM) | David Brownell, Al Borchers, Felipe Balbi |
| `g_ether.ko` | `drivers/usb/gadget/g_ether.c` (CDC-ECM/RNDIS) | David Brownell, Benedikt Spanger |

These are licensed under the **GNU General Public License, version 2**
(as published by the FSF; `Module: license=GPL` in `modinfo`). The complete
corresponding source is the Linux kernel they were built against:

- Linux 2.6.28 source: <https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/tag/?h=v2.6.28>
  (or <https://cdn.kernel.org/pub/linux/kernel/v2.6/linux-2.6.28.tar.gz>)

The modules are redistributed unmodified, as shipped by the vendor kernel,
to support the bq Cervantes 2 (kernel 2.6.28, ARMv5TE) target device. The
GPL-2.0 license applies to these two files and to any redistribution of them;
it does not extend to the rest of this repository, which remains MIT.

All other firmware artifacts in `assets/firmware/` (`inkd.c`/`inkd`,
`epdraw.c`/`epdraw`, `epd_rt.h`) are original to this project (MIT).

## Rust dependencies

Listed in `Cargo.lock`; licenses of the direct dependencies:

| Crate | License |
|---|---|
| `serialport` | MPL-2.0 |
| `clap`, `image`, `sha2`, `serde`, `serde_json`, `hex`, `tiny_http`, `tray-icon` | MIT OR Apache-2.0 |

Transitive dependencies follow the same permissive set (MIT, Apache-2.0,
BSD, ISC, Unicode-3.0, MPL-2.0, Zlib); see `Cargo.lock` for the full graph.

## Referenced but not shipped

`assets/templates/usb_ready.html` references the
[Literata](https://github.com/google/fonts/tree/main/ofl/literata) font
(SIL Open Font License 1.1) by relative path for on-device rendering. The
font files are **not** included in this repository. `boot.4bpp` is a
pre-rendered image of that template.

The example fixture `examples/candles.json` is a capture of public market
data from the Coinbase Exchange public candles API (no personal data).
