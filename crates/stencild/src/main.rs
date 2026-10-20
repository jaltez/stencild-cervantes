//! stencild — interface service for Stencild e-ink panels.

mod device;
mod frame;
mod installer;
mod service;
mod tray;
mod update;

use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "stencild", version, about = "Stencild e-ink panel interface service")]
struct Cli {
    /// Listen address for the HTTP API (default command).
    #[arg(long, default_value = "127.0.0.1:8787", global = true)]
    listen: String,

    /// Serial device: `auto`, `COM13`, or `/dev/ttyACM0`.
    #[arg(long, default_value = "auto", global = true)]
    device: String,

    /// Directory whose new images are pushed automatically.
    #[arg(long, global = true)]
    inbox: Option<PathBuf>,

    /// Default waveform: 0 INIT, 1 MU, 2 GU, 3 GC, 4 NOP.
    #[arg(long, default_value_t = 2, global = true)]
    waveform: u8,

    /// Never touch a serial port; write frames to --frame-out instead.
    #[arg(long, global = true)]
    no_device: bool,

    /// Write each EPD1 frame to this file (dry-run / debugging).
    #[arg(long, global = true)]
    frame_out: Option<PathBuf>,

    /// Validate tray icon assets and exit (no display required).
    #[arg(long, global = true)]
    tray_selftest: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the HTTP service (default).
    Serve,
    /// Emit installer artifacts (binary + install scripts).
    Installer {
        /// Only generate files; never execute anything.
        #[arg(long)]
        dry_run: bool,
        /// Output directory.
        #[arg(long, default_value = "dist")]
        out: PathBuf,
    },
    /// Apply a self-update from a local manifest.
    Update {
        /// Manifest URL or path (file://... supported; sha256 required).
        #[arg(long)]
        manifest: String,
        /// Installation prefix to update (bin/stencild inside).
        #[arg(long, default_value = ".")]
        prefix: PathBuf,
        /// Apply the update (otherwise only validate).
        #[arg(long)]
        apply: bool,
        /// Execute the new binary --version and require the manifest version.
        #[arg(long)]
        selftest: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    if cli.tray_selftest {
        match tray::selftest() {
            Ok(()) => std::process::exit(0),
            Err(e) => {
                eprintln!("tray selftest: {e}");
                std::process::exit(1);
            }
        }
    }

    match cli.command {
        Some(Command::Installer { dry_run: _, out }) => {
            match installer::emit(&out) {
                Ok(()) => {
                    println!("installer artifacts written to {}", out.display());
                }
                Err(e) => {
                    eprintln!("installer: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some(Command::Update { manifest, prefix, apply, selftest }) => {
            let manifest_path = update::file_from_url(&manifest)
                .unwrap_or_else(|e| fail(&e));
            if !apply {
                match std::fs::read_to_string(&manifest_path)
                    .map_err(|e| e.to_string())
                    .and_then(|t| update::parse_manifest(&t))
                {
                    Ok(m) => println!(
                        "update available: {} (sha256 {}) — pass --apply to install",
                        m.version, m.sha256
                    ),
                    Err(e) => fail(&e),
                }
                return;
            }
            match update::apply(&manifest_path, &prefix, selftest) {
                Ok(msg) => println!("{msg}"),
                Err(e) => fail(&e),
            }
        }
        Some(Command::Serve) | None => {
            let device = if cli.no_device {
                device::Device::offline()
            } else {
                let name = if cli.device == "auto" {
                    match device::autodetect() {
                        Some(p) => p,
                        None => {
                            eprintln!("no device found (PING/PONG); continuing offline");
                            String::from("none")
                        }
                    }
                } else {
                    cli.device.clone()
                };
                if name == "none" {
                    device::Device::offline()
                } else {
                    device::Device::open(&name, cli.waveform)
                }
            };

            println!("[stencild] device: {} (open: {})", device.name, device.is_open());
            let svc = service::Service::new(device, cli.frame_out.clone(), cli.inbox.clone());
            svc.spawn_inbox();

            cfg_if_tray();

            if let Err(e) = service::serve(Arc::clone(&svc), &cli.listen) {
                fail(&e);
            }
        }
    }
}

fn cfg_if_tray() {
    #[cfg(all(feature = "tray", windows))]
    tray::spawn();
}

fn fail(msg: &str) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1)
}
