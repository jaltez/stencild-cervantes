# Service API

`stencild` serves HTTP on `--listen` (default `127.0.0.1:8787`).

## Endpoints

### `GET /health`

```json
{"device":"COM13","online":true,"pushes":42,"last":"OK"}
```

`online` probes the device with a PING when a port is open; responds in
milliseconds when offline. Status 503 when the device is offline.

### `POST /image`

Body: any supported image bytes (PNG/JPEG/BMP/GIF/WEBP). The service resizes
to 800×600 grayscale, packs 4bpp, and pushes.

Query parameters:
- `invert=1` — invert polarity
- `waveform=N` — 0 INIT, 1 MU, 2 GU (default), 3 GC, 4 NOP
- `levels`, `dither` — reserved; quantization happens before posting

Responses: `200 {"device":"OK"}` on success, `500 {"error":...}` on decode
or device failure. The `Content-Type` header is ignored — the body is
sniffed by the decoder, so `curl --data-binary @file.png` works as-is.

```sh
curl -s -X POST --data-binary @chart.png http://127.0.0.1:8787/image
curl -s -X POST "http://127.0.0.1:8787/image?invert=1&waveform=3" \
     --data-binary @chart.png
```

### `POST /raw`

Body: exactly 240,000 bytes of packed 4bpp payload (see
[apps guide](apps-guide.md) for the packing). Skips conversion; always
waveform 2, no invert — use `/image` when you need those. Responses:
`200`, `400` when the size is wrong, `502` on device failure.

```sh
curl -s -X POST --data-binary @frame.bin http://127.0.0.1:8787/raw
```

## Inbox watcher

With `--inbox DIR`, any new image file in `DIR` is converted, pushed, and
moved to a sibling `done/` directory. Files must finish writing (size stable
for ~1 s) before processing.

## EPD1 wire protocol

Serial: 115200 8N1 over CDC-ACM. One frame:

```
"EPD1" | u32 LE payload_size | u8 waveform | u8 invert | payload bytes
```

- `payload_size = 0` → PING.
- Replies are one line: `OK`, `PONG`, `ERR <reason>`.
- Payload: 240,000 bytes, 800×600 4bpp grayscale, even-x pixel in the low
  nibble (see [apps guide](apps-guide.md)).

The device daemon (`inkd`) writes each frame with the kernel display ioctl;
full-screen frames trigger a full e-ink refresh.
