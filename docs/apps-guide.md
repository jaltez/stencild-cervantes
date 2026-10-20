# Apps guide

How to build applications that display content on a Stencild panel.

## The contract

Your app produces an **image**; the service converts and transports it.
You never talk to the device directly.

- POST any PNG/JPEG/BMP/GIF/WEBP to `POST /image` (see [service API](service-api.md)).
- Target canvas: **800×600 landscape**. The kernel rotates it onto the physical
  portrait glass; author content in landscape orientation.

## Rendering with HTML + JS

Chrome/Chromium headless renders HTML to an 800×600 screenshot reliably:

```sh
google-chrome-stable --headless=new --no-sandbox --disable-gpu \
  --run-all-compositor-stages-before-draw --hide-scrollbars \
  --window-size=800,600 --default-background-color=FFFFFFFF \
  --screenshot=out.png template.html
```

Rules learned from the field:

- Reference local scripts (`<script src="...">`) with **absolute `file:///`
  URLs**; relative paths and `file://host/...` forms silently fail to load.
- On Windows, `fs::canonicalize` yields verbatim paths (`\\?\C:\...`) — strip
  the prefix before building the URL.
- Give `<canvas>` explicit `width`/`height` attributes and disable responsive
  sizing in chart libraries (Chart.js: `responsive:false`, `animation:false`).
- Pass `--user-data-dir=<isolated dir>` so a running browser session does not
  absorb the headless invocation (Windows singleton behavior).
- Use `Stdio::null()` (or equivalent) when spawning Chrome from a service;
  helper processes inherit pipes and can hang the parent on Windows.

## Recommended libraries

- **Chart.js 4** (vendored UMD build) — charts; pure black bars on white read
  best on e-ink.
- Any CSS the headless Chrome supports, including Tailwind via CDN (requires
  network at render time; vendoring is preferred for offline determinism).
- Fonts: `@font-face` with local TTF files; serif faces (Literata, Georgia)
  render well at e-ink sizes.

## Image measures and format

| Property | Value |
|---|---|
| Canvas | 800×600, landscape |
| Wire payload | 240,000 bytes, packed **4bpp grayscale** |
| Packing | even-x pixel in the **low nibble**, odd-x in the high nibble; row stride 400 bytes |
| Polarity | 0 = black, 0xF = white |
| Grays | 16 levels; avoid near-white grays (they ghost) |

The service packs this for you from any image; only generate 4bpp yourself
(`POST /raw`) if you have an exotic pipeline.

## Refresh times

Every push is a **full refresh**: ~1 s of panel update with the characteristic
black/white flash.

| Use case | Interval |
|---|---|
| Hard technical floor | ≥ 2 s |
| Comfortable dashboards | ≥ 10 s |
| Clock-style content | 60 s |

Partial updates (no flash) require region support in the device daemon; not
available in the current `inkd`.

## Caveats

- E-ink retains the last image with no power; an idle app burns nothing.
- The boot progress animation stops at the first successful display ioctl.
- Avoid dithered gradients; use pure black/white and one or two grays.
- If the USB link drops, `stencild` returns `ERR offline` immediately and
  retries on the next push; the device daemon also reopens its side.

## Worked example: a 24-hour BTC chart

A complete app is three steps, and only the first one is yours:

1. **Get the data** (any source; here the public Coinbase candles API).
2. **Render an 800×600 image** (any tool that outputs an image).
3. **POST it** to `stencild` and sleep until the next refresh.

[`examples/btc_chart_demo.py`](../examples/btc_chart_demo.py) is a runnable
distillation of exactly that, using the bundled public-market fixture
`examples/candles.json` (no network needed):

```sh
pip install pillow
python3 examples/btc_chart_demo.py --no-push --out chart.png   # render only
python3 examples/btc_chart_demo.py                             # render + POST
```

```python
# the whole integration is this:
img = render(series)                       # PIL, mode "L", 800x600, white bg
buf = io.BytesIO(); img.save(buf, "PNG")
urllib.request.urlopen(urllib.request.Request(
    "http://127.0.0.1:8787/image", data=buf.getvalue(),
    headers={"Content-Type": "image/png"}), timeout=30)
```

The production version of this app (from which the example is distilled)
follows the same shape in Rust: fetch 5-minute candles, keep the last 24 h,
draw a black price polyline over a faint graticule with a 5×7 bitmap font,
POST the PNG, sleep 300 s, repeat. Fields worth copying:

- One gray for the grid, black for data and text — no gradients, no dither.
- Numbers that matter (price, 24 h change, high/low, volume) in fixed
  positions, so consecutive refreshes change as few pixels as possible.
- First fetch failure is fatal (bad config); later failures keep the last
  good frame and retry on the next interval.

### Sending data without writing an app

Anything that can produce a file works. Two zero-code paths:

```sh
# one-shot push of any image
curl -X POST --data-binary @photo.png http://127.0.0.1:8787/image

# drop files into a watched directory; stencild pushes and archives them
stencild --inbox /home/me/panel-inbox
cp chart.png /home/me/panel-inbox/
```

For batch/offline conversion the repo also ships
[`assets/templates/png2epd.py`](../assets/templates/png2epd.py), which
converts any image to the native `.4bpp` format for `POST /raw`.
