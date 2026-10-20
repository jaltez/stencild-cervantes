#!/usr/bin/env python3
"""btc_chart_demo.py — minimal Stencild app: fixture data -> 800x600 chart -> panel.

A complete, runnable distillation of the btc-chart app pattern
(fetch a series, render an 800x600 landscape image, POST it to stencild).
Uses the bundled public-market fixture so it needs no network:

    python3 examples/btc_chart_demo.py                    # render + POST
    python3 examples/btc_chart_demo.py --out chart.png    # render only

Requires: Pillow. The stencild service accepts any image format it was built
with (PNG/JPEG/BMP/GIF/WEBP); PNG is used here.
"""

import argparse
import json
import os
import sys
import urllib.request
from datetime import datetime, timezone

from PIL import Image, ImageDraw

W, H = 800, 600          # panel canvas: 800x600 LANDSCAPE (kernel rotates it)
WINDOW = 24 * 3600       # keep the newest 24 h of data
SERVICE = "http://127.0.0.1:8787"


def load_series(path):
    """Coinbase candles: [[time, low, high, open, close, volume], ...] newest-first."""
    with open(path) as f:
        candles = json.load(f)
    pts = sorted((c[0], c[4]) for c in candles)   # (time, close), oldest first
    t_end = pts[-1][0]
    pts = [(t, p) for t, p in pts if t >= t_end - WINDOW]
    if len(pts) < 2:
        sys.exit("not enough points in window")
    vol = sum(c[5] for c in candles if c[0] >= t_end - WINDOW)
    return pts, vol


def render(pts, vol):
    img = Image.new("L", (W, H), 255)             # white, grayscale
    d = ImageDraw.Draw(img)

    # chart area with a light-gray graticule (e-ink: keep it faint, 2 grays max)
    x0, y0, x1, y1 = 30, 80, 769, 520
    lo = min(p for _, p in pts)
    hi = max(p for _, p in pts)
    span = (hi - lo) or 1.0
    for frac in (0.25, 0.5, 0.75):
        y = int(y1 - frac * (y1 - y0))
        d.line([(x0, y), (x1, y)], fill=200, width=1)
    t0, t1 = pts[0][0], pts[-1][0]
    for k in range(1, 4):                          # UTC time ticks every 6 h
        t = t0 + k * (t1 - t0) / 4
        x = int(x0 + (t - t0) / (t1 - t0) * (x1 - x0))
        d.line([(x, y0), (x, y1)], fill=200, width=1)

    # price polyline, pure black (highest contrast on e-ink)
    xy = [
        (x0 + (t - t0) / (t1 - t0) * (x1 - x0), y1 - (p - lo) / span * (y1 - y0))
        for t, p in pts
    ]
    d.line(xy, fill=0, width=2)

    # header + footer strip (PIL default bitmap font; swap for TTFs in real apps)
    when = datetime.fromtimestamp(pts[-1][0], tz=timezone.utc).strftime("%d %b %H:%M UTC")
    chg = (pts[-1][1] - pts[0][1]) / pts[0][1] * 100
    d.text((x0, 24), "BTC-USD 24H", fill=0)
    d.text((x0, 44), f"{pts[-1][1]:,.0f} USD  ({chg:+.2f}%)  {when}", fill=0)
    d.text((x0, 540), f"LOW {lo:,.0f}   HIGH {hi:,.0f}   VOL {vol:,.0f} BTC", fill=0)
    return img


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--fixture", default=os.path.join(os.path.dirname(__file__), "candles.json"))
    ap.add_argument("--url", default=SERVICE, help="stencild base URL")
    ap.add_argument("--out", help="also write the PNG to this path")
    ap.add_argument("--no-push", action="store_true", help="render only, never POST")
    a = ap.parse_args()

    pts, vol = load_series(a.fixture)
    img = render(pts, vol)

    png_path = a.out or "/tmp/btc_chart_demo.png"
    img.save(png_path, "PNG")
    print(f"rendered {png_path} ({W}x{H})")

    if a.no_push:
        return
    with open(png_path, "rb") as f:
        req = urllib.request.Request(
            f"{a.url.rstrip('/')}/image", data=f.read(),
            headers={"Content-Type": "image/png"}, method="POST")
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            print(f"push: HTTP {r.status} {r.read().decode()}")
    except urllib.error.HTTPError as e:
        print(f"push: HTTP {e.code} {e.read().decode()}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
