#!/usr/bin/env python3
"""png2epd.py — convert an image to the Cervantes 2 (E60M00) e-ink raw format.

Output: packed 4bpp grayscale, 800x600 LANDSCAPE (the kernel DC geometry the
reader app itself uses — fake_s1d13522.c:809), LOW nibble = even-x pixel.
The kernel rotates it onto the portrait panel. Feed to `epdraw`/`push_panel.py`.

Importable: convert(src_bytes_or_path, out_path, ...) is used by paneld/reloj.
"""
import argparse
import io
import sys
from PIL import Image, ImageOps

PANEL_W, PANEL_H = 800, 600   # DC landscape


def convert(src, out, invert=False, levels=16, dither=False):
    """src: path or bytes of any PIL-decodable image; out: .4bpp path."""
    data = src if isinstance(src, bytes) else open(src, "rb").read()
    im = Image.open(io.BytesIO(data)).convert("L")
    im = im.resize((PANEL_W, PANEL_H), Image.LANCZOS)
    if invert:
        im = ImageOps.invert(im)
    if dither and levels < 16:
        im = im.convert("P", palette=Image.ADAPTIVE, colors=levels).convert("L")

    px = im.load()
    out_buf = bytearray(PANEL_W * PANEL_H // 2)
    for yy in range(PANEL_H):
        row = yy * (PANEL_W // 2)
        for xx in range(0, PANEL_W, 2):
            if levels < 16:
                lo = px[xx, yy] * levels // 256
                hi = px[xx + 1, yy] * levels // 256
            else:
                lo = px[xx, yy] >> 4
                hi = px[xx + 1, yy] >> 4
            out_buf[row + (xx >> 1)] = (hi << 4) | (lo & 0xF)
    with open(out, "wb") as f:
        f.write(out_buf)
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("image")
    ap.add_argument("-o", "--out")
    ap.add_argument("--invert", action="store_true")
    ap.add_argument("--levels", type=int, default=16)
    ap.add_argument("--dither", action="store_true")
    a = ap.parse_args()
    if not 2 <= a.levels <= 16:
        sys.exit("--levels must be 2..16")
    dest = a.out or a.image + ".4bpp"
    convert(a.image, dest, a.invert, a.levels, a.dither)
    print(f"{dest}: {PANEL_W*PANEL_H//2} bytes, 4bpp {PANEL_W}x{PANEL_H} landscape, "
          f"levels={a.levels} invert={a.invert}")


if __name__ == "__main__":
    main()
