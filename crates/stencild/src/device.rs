//! Serial device client (EPD1 over CDC-ACM) with single-try reconnect.

use std::io::{Read, Write};
use std::sync::Mutex;
use std::time::Duration;

use serialport::SerialPort;

use crate::frame::{encode, PAYLOAD};

pub struct Device {
    pub name: String,
    port: Option<Mutex<Box<dyn SerialPort>>>,
    pub waveform: u8,
}

impl Device {
    pub fn open(name: &str, waveform: u8) -> Self {
        let port = serialport::new(name, 115_200)
            .timeout(Duration::from_secs(1))
            .open()
            .ok()
            .map(Mutex::new);
        Device { name: name.to_string(), port, waveform }
    }

    pub fn offline() -> Self {
        Device { name: "none".into(), port: None, waveform: 2 }
    }

    pub fn is_open(&self) -> bool {
        self.port.is_some()
    }

    /// Send a frame; returns the device reply ("OK", "PONG", ...) or an error
    /// string. Never blocks longer than one short reopen attempt.
    pub fn send_frame(&self, payload: &[u8], waveform: u8, invert: bool) -> Result<String, String> {
        if payload.len() != PAYLOAD {
            return Err(format!("payload must be {PAYLOAD} bytes"));
        }
        let frame = encode(payload, waveform, invert as u8);
        if let Some(lock) = &self.port {
            let mut p = lock.lock().unwrap();
            if let Ok(r) = exchange(&mut **p, &frame) {
                return Ok(r);
            }
        }
        // One bounded reopen attempt; failures return immediately (never hang).
        match serialport::new(&self.name, 115_200)
            .timeout(Duration::from_secs(2))
            .open()
        {
            Ok(mut p) => {
                let r = exchange(&mut *p, &frame);
                if let Some(lock) = &self.port {
                    *lock.lock().unwrap() = p;
                }
                r.map_err(|e| format!("offline: {e}"))
            }
            Err(e) => Err(format!("offline: {e}")),
        }
    }

    pub fn ping(&self) -> bool {
        let Some(lock) = &self.port else { return false };
        let mut p = match lock.lock() {
            Ok(p) => p,
            Err(_) => return false,
        };
        p.clear(serialport::ClearBuffer::Input).ok();
        let frame = encode(&[], 0, 0);
        if p.write_all(&frame).and_then(|_| p.flush()).is_err() {
            return false;
        }
        let mut line = String::new();
        let mut b = [0u8; 1];
        for _ in 0..16 {
            match p.read(&mut b) {
                Ok(1) if b[0] == b'\n' => break,
                Ok(1) => line.push(b[0] as char),
                _ => break,
            }
        }
        line.trim() == "PONG"
    }
}

fn exchange(port: &mut dyn SerialPort, frame: &[u8]) -> std::io::Result<String> {
    port.clear(serialport::ClearBuffer::Input).ok();
    port.write_all(frame)?;
    port.flush()?;
    let mut line = String::new();
    let mut b = [0u8; 1];
    for _ in 0..64 {
        match port.read(&mut b) {
            Ok(1) if b[0] == b'\n' => break,
            Ok(1) => line.push(b[0] as char),
            Ok(_) => continue,
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => break,
            Err(e) => return Err(e),
        }
    }
    if line.is_empty() {
        return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "no reply"));
    }
    Ok(line.trim().to_string())
}

/// Probe every serial port with a PING; returns the first that answers PONG.
pub fn autodetect() -> Option<String> {
    let ports = serialport::available_ports().ok()?;
    for p in ports {
        let Ok(mut s) = serialport::new(&p.port_name, 115_200)
            .timeout(Duration::from_millis(700))
            .open()
        else {
            continue;
        };
        s.clear(serialport::ClearBuffer::Input).ok();
        let frame = encode(&[], 0, 0);
        if s.write_all(&frame).and_then(|_| s.flush()).is_err() {
            continue;
        }
        let mut line = String::new();
        let mut b = [0u8; 1];
        for _ in 0..16 {
            match s.read(&mut b) {
                Ok(1) if b[0] == b'\n' => break,
                Ok(1) => line.push(b[0] as char),
                _ => break,
            }
        }
        if line.trim() == "PONG" {
            return Some(p.port_name);
        }
    }
    None
}
