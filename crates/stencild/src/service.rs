//! Image decoding + HTTP API + inbox watcher for the Stencild service.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use image::imageops::FilterType;
use tiny_http::{Header, Method, Response};

use crate::device::Device;
use crate::frame::{encode, PAYLOAD, WIDTH, HEIGHT};

pub struct Service {
    pub device: Device,
    pub pushes: AtomicU64,
    pub last: std::sync::Mutex<String>,
    pub frame_out: Option<PathBuf>,
    pub inbox: Option<PathBuf>,
    /// When set, pushes fail fast with "paused" (tray Pause button).
    pub paused: AtomicBool,
}

/// Boot screen ("USB READY") payload used by Clear: 240,000 packed 4bpp bytes.
pub const CLEAR_PAYLOAD: &[u8] = include_bytes!("../../../assets/templates/boot.4bpp");

impl Service {
    pub fn new(device: Device, frame_out: Option<PathBuf>, inbox: Option<PathBuf>) -> Arc<Self> {
        Arc::new(Service {
            device,
            pushes: AtomicU64::new(0),
            last: std::sync::Mutex::new(String::new()),
            frame_out,
            inbox,
            paused: AtomicBool::new(false),
        })
    }

    #[cfg_attr(not(all(feature = "tray", windows)), allow(dead_code))]
    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    /// Reset the panel to the boot image (tray Clear / POST /clear).
    pub fn clear_panel(&self) -> Result<String, String> {
        self.push_payload(CLEAR_PAYLOAD, None, false)
    }

    /// Convert image bytes (any supported format) to a panel payload.
    pub fn image_to_payload(bytes: &[u8], invert: bool) -> Result<Vec<u8>, String> {
        let img = image::load_from_memory(bytes).map_err(|e| format!("unrecognized image: {e}"))?;
        let gray = image::imageops::resize(&img.to_luma8(), WIDTH, HEIGHT, FilterType::Lanczos3);
        let mut payload = vec![0u8; PAYLOAD];
        for (i, px) in gray.pixels().enumerate() {
            let mut v = px.0[0] >> 4;
            if invert {
                v = 15 - v;
            }
            if i & 1 == 0 {
                payload[i >> 1] |= v;
            } else {
                payload[i >> 1] |= v << 4;
            }
        }
        Ok(payload)
    }

    /// Send a payload to the panel (or dump the frame when in dry-run mode).
    pub fn push_payload(&self, payload: &[u8], waveform: Option<u8>, invert: bool) -> Result<String, String> {
        if self.is_paused() {
            let e = "paused".to_string();
            *self.last.lock().unwrap() = e.clone();
            return Err(e);
        }
        let wf = waveform.unwrap_or(self.device.waveform);
        let result = if let Some(path) = &self.frame_out {
            let frame = encode(payload, wf, invert as u8);
            fs::write(path, &frame).map_err(|e| format!("frame write: {e}"))?;
            Ok("OK".to_string())
        } else {
            self.device.send_frame(payload, wf, invert)
        };
        match &result {
            Ok(r) => {
                *self.last.lock().unwrap() = r.clone();
                self.pushes.fetch_add(1, Ordering::Relaxed);
            }
            Err(e) => {
                *self.last.lock().unwrap() = e.clone();
            }
        }
        result
    }

    pub fn push_image(&self, bytes: &[u8], invert: bool, waveform: Option<u8>) -> Result<String, String> {
        let payload = Service::image_to_payload(bytes, invert)?;
        self.push_payload(&payload, waveform, false)
    }

    pub fn health_json(&self) -> String {
        let online = self.device.is_open() && self.device.ping();
        format!(
            "{{\"device\":\"{}\",\"online\":{},\"paused\":{},\"pushes\":{},\"last\":\"{}\"}}",
            self.device.name,
            online,
            self.is_paused(),
            self.pushes.load(Ordering::Relaxed),
            self.last.lock().unwrap()
        )
    }

    /// Watch the inbox directory; convert+push new files, archive to done/.
    pub fn spawn_inbox(self: &Arc<Self>) {
        let Some(inbox) = self.inbox.clone() else { return };
        let done = inbox.parent().unwrap_or(Path::new(".")).join("done");
        let me = Arc::clone(self);
        fs::create_dir_all(&inbox).ok();
        fs::create_dir_all(&done).ok();
        std::thread::spawn(move || {
            let mut seen: Vec<String> = Vec::new();
            loop {
                if let Ok(entries) = fs::read_dir(&inbox) {
                    for e in entries.flatten() {
                        let name = e.file_name().to_string_lossy().to_string();
                        if name.starts_with('.') || seen.contains(&name) {
                            continue;
                        }
                        let path = e.path();
                        let Ok(meta) = fs::metadata(&path) else { continue };
                        if !meta.is_file() || meta.len() == 0 {
                            continue;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(800));
                        let Ok(meta2) = fs::metadata(&path) else { continue };
                        if meta2.len() != meta.len() {
                            continue; // still being written
                        }
                        seen.push(name.clone());
                        let Ok(bytes) = fs::read(&path) else { continue };
                        let resp = if bytes.len() == PAYLOAD {
                            me.push_payload(&bytes, None, false)
                        } else {
                            me.push_image(&bytes, false, None)
                        };
                        println!("[inbox] {name}: {:?}", resp);
                        let _ = fs::rename(&path, done.join(&name));
                    }
                }
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
        });
    }
}

pub fn serve(service: Arc<Service>, listen: &str) -> Result<(), String> {
    let server = tiny_http::Server::http(listen).map_err(|e| format!("bind {listen}: {e}"))?;
    println!("[stencild] HTTP on {listen} (/health /image /raw /clear)");
    for mut req in server.incoming_requests() {
        let (method, url) = (req.method().clone(), req.url().to_string());
        let mut body = Vec::new();
        req.as_reader().read_to_end(&mut body).ok();
        let json_ct = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
        let route = url.split('?').next().unwrap_or("");

        let resp = match (method, route) {
            (Method::Get, "/health") => {
                let online = service.device.is_open();
                Response::from_string(service.health_json())
                    .with_status_code(if online { 200 } else { 503 })
                    .with_header(json_ct)
            }
            (Method::Post, "/raw") => {
                if body.len() != PAYLOAD {
                    Response::from_string(format!("{{\"error\":\"expected {PAYLOAD} bytes\"}}"))
                        .with_status_code(400)
                        .with_header(json_ct)
                } else {
                    match service.push_payload(&body, None, false) {
                        Ok(r) => Response::from_string(format!("{{\"device\":\"{r}\"}}"))
                            .with_status_code(200)
                            .with_header(json_ct),
                        Err(e) => Response::from_string(format!("{{\"error\":\"{e}\"}}"))
                            .with_status_code(502)
                            .with_header(json_ct),
                    }
                }
            }
            (Method::Post, "/image") => {
                let invert = url.contains("invert=1") || url.contains("invert=true");
                let wf = url
                    .split("waveform=")
                    .nth(1)
                    .and_then(|s| s.split('&').next())
                    .and_then(|s| s.parse::<u8>().ok());
                match service.push_image(&body, invert, wf) {
                    Ok(r) => Response::from_string(format!("{{\"device\":\"{r}\"}}"))
                        .with_status_code(200)
                        .with_header(json_ct),
                    Err(e) => Response::from_string(format!("{{\"error\":\"{e}\"}}"))
                        .with_status_code(500)
                        .with_header(json_ct),
                }
            }
            (Method::Post, "/clear") => match service.clear_panel() {
                Ok(r) => Response::from_string(format!("{{\"device\":\"{r}\"}}"))
                    .with_status_code(200)
                    .with_header(json_ct),
                Err(e) => Response::from_string(format!("{{\"error\":\"{e}\"}}"))
                    .with_status_code(503)
                    .with_header(json_ct),
            },
            _ => Response::from_string("{\"error\":\"unknown\"}")
                .with_status_code(404)
                .with_header(json_ct),
        };
        req.respond(resp).ok();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_to_payload_encodes_known_pixels() {
        // 800x600 white PNG with one black pixel at (0,0)
        let mut img = image::GrayImage::from_pixel(WIDTH, HEIGHT, image::Luma([255u8]));
        img.put_pixel(0, 0, image::Luma([0u8]));
        let mut png = Vec::new();
        image::DynamicImage::ImageLuma8(img)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let payload = Service::image_to_payload(&png, false).unwrap();
        assert_eq!(payload.len(), PAYLOAD);
        assert_eq!(payload[0] & 0x0F, 0x0); // even pixel 0 = low nibble 0 (black)
        assert_eq!(payload[0] >> 4, 0xF); // odd pixel 1 = high nibble 0xF (white)
    }

    #[test]
    fn paused_service_rejects_pushes_without_counting() {
        let svc = Service::new(Device::offline(), None, None);
        svc.set_paused(true);
        let err = svc.push_payload(&vec![0u8; PAYLOAD], None, false).unwrap_err();
        assert_eq!(err, "paused");
        assert_eq!(svc.pushes.load(Ordering::Relaxed), 0);
        assert_eq!(svc.last.lock().unwrap().as_str(), "paused");
        assert!(svc.health_json().contains("\"paused\":true"));
    }

    #[test]
    fn clear_payload_is_a_full_boot_frame() {
        assert_eq!(CLEAR_PAYLOAD.len(), PAYLOAD);
        let svc = Service::new(Device::offline(), None, None);
        svc.set_paused(true);
        assert_eq!(svc.clear_panel().unwrap_err(), "paused");
    }
}
