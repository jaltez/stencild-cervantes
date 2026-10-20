# SD guide

The init SD card turns the bq Cervantes 2 into a Stencild panel at boot,
without touching the device's NAND.

## Building the card

```sh
sdgen build --out /tmp/sdtest --size 64M
```

Copy the resulting tree onto a **FAT32** card (any size ≥ 64 MB). `--size`
records the intended card size in `SD_SIZE.txt` only; no image file is written.
`MANIFEST.sha256` in the root verifies every file (`sha256sum -c`).

## What's on the card

| Path | Purpose |
|---|---|
| `NetronixInc_MFG.sh` | MFG runner executed as root at boot (stock `/bin/g` hook) |
| `ordenes/00…21_*.sh` | Boot orders: paint boot image, load `g_serial`, start `inkd` |
| `panel/imagen.4bpp` | Image painted at boot (240,000 bytes, 800×600 4bpp) |
| `panel/epdraw` | One-shot painter (ARMv5, freestanding) |
| `panel/usb/inkd` | Device daemon: EPD1 frames → display ioctl |
| `panel/usb/g_serial.ko`, `g_ether.ko` | Kernel gadget modules (CDC-ACM / CDC-ECM) |

`g_serial.ko` / `g_ether.ko` are stock Linux 2.6.28 gadget modules
redistributed under GPL-2.0 (see [THIRD_PARTY](../THIRD_PARTY.md)); the
other card payloads (`epdraw`, `inkd`) are built from the sources in
[`assets/firmware/`](../assets/firmware/).

## Boot behavior

- Card inserted at power-on → panel mode: the reader app (`ebrmain`) never
  starts; USB enumerates as a CDC-ACM serial port.
- Card absent at power-on → normal reader. Nothing is ever written to NAND.
- Create a file named `LEER` in the card root to boot the normal reader even
  with the card inserted.

## Recovery

Remove the card and power-cycle. The stock firmware is untouched; the kit
runs entirely from the card.
