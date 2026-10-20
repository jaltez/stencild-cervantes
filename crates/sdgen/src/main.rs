//! sdgen — build the Stencild init SD card tree (no device access).

use std::fs;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use sha2::{Digest, Sha256};

const RUNNER: &[u8] = include_bytes!("../../../assets/scripts/runner.sh");
const ORDEN20: &[u8] = include_bytes!("../../../assets/scripts/orden20.sh");
const ORDEN21: &[u8] = include_bytes!("../../../assets/scripts/orden21.sh");
const EPDRAW: &[u8] = include_bytes!("../../../assets/firmware/epdraw");
const INKD: &[u8] = include_bytes!("../../../assets/firmware/inkd");
const G_SERIAL: &[u8] = include_bytes!("../../../assets/firmware/g_serial.ko");
const G_ETHER: &[u8] = include_bytes!("../../../assets/firmware/g_ether.ko");
const BOOT_IMAGE: &[u8] = include_bytes!("../../../assets/templates/boot.4bpp");

#[derive(Parser)]
#[command(name = "sdgen", version, about = "Build the Stencild init SD card")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Write the SD tree into --out (directory; no device is written).
    Build {
        /// Output directory.
        #[arg(long)]
        out: PathBuf,
        /// Target card size (recorded in SD_SIZE.txt; no image is created).
        #[arg(long, default_value = "2G")]
        size: String,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Build { out, size } => {
            if let Err(e) = build(&out, &size) {
                eprintln!("sdgen: {e}");
                std::process::exit(1);
            }
        }
    }
}

fn build(out: &Path, size: &str) -> Result<(), String> {
    fs::create_dir_all(out).map_err(|e| format!("mkdir {}: {e}", out.display()))?;

    write(out.join("NetronixInc_MFG.sh"), RUNNER)?;
    write(out.join("ordenes/20_panel.sh"), ORDEN20)?;
    // orden 21 references the device daemon by its shipped name: inkd.
    let orden21 = String::from_utf8_lossy(ORDEN21).replace("epdserver", "inkd").replace("stencild", "inkd");
    write(out.join("ordenes/21_usb.sh"), orden21.as_bytes())?;
    write(out.join("panel/epdraw"), EPDRAW)?;
    write(out.join("panel/imagen.4bpp"), BOOT_IMAGE)?;
    write(out.join("panel/usb/inkd"), INKD)?;
    write(out.join("panel/usb/g_serial.ko"), G_SERIAL)?;
    write(out.join("panel/usb/g_ether.ko"), G_ETHER)?;

    fs::write(out.join("SD_SIZE.txt"), format!("target card size: {size}\n"))
        .map_err(|e| format!("SD_SIZE.txt: {e}"))?;
    fs::write(
        out.join("README.txt"),
        "Stencild init SD\n\
         ===============\n\
         Boot the reader with this card inserted: the MFG hook runs the orderes in\n\
         ordenes/ (paints panel/imagen.4bpp, loads g_serial and starts inkd).\n\
         Create a file named LEER in the card root to boot the normal reader UI.\n\
         Card must be FAT32.\n",
    )
    .map_err(|e| format!("README.txt: {e}"))?;

    let mut entries: Vec<(String, String)> = Vec::new();
    collect(out, out, &mut entries)?;
    entries.sort();
    let manifest: String = entries
        .iter()
        .map(|(sha, rel)| format!("{sha}  {rel}\n"))
        .collect();
    fs::write(out.join("MANIFEST.sha256"), manifest).map_err(|e| format!("manifest: {e}"))?;

    println!("SD tree written to {}", out.display());
    Ok(())
}

fn write(path: PathBuf, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
    }
    fs::write(&path, bytes).map_err(|e| format!("write {}: {e}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if path.extension().is_none() || path.extension().is_some_and(|e| e == "sh") {
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
                .map_err(|e| format!("chmod {}: {e}", path.display()))?;
        }
    }
    Ok(())
}

fn collect(base: &Path, dir: &Path, out: &mut Vec<(String, String)>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("read {}: {e}", dir.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "MANIFEST.sha256" || name == "System Volume Information" {
            continue;
        }
        if path.is_dir() {
            collect(base, &path, out)?;
        } else {
            let data = fs::read(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
            let mut h = Sha256::new();
            h.update(&data);
            let rel = path
                .strip_prefix(base)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            out.push((hex::encode(h.finalize()), rel));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_layout_and_manifest_are_consistent() {
        let out = std::env::temp_dir().join(format!("sdgen-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&out);
        build(&out, "2G").unwrap();
        for rel in [
            "NetronixInc_MFG.sh",
            "ordenes/20_panel.sh",
            "ordenes/21_usb.sh",
            "panel/imagen.4bpp",
            "panel/epdraw",
            "panel/usb/inkd",
            "panel/usb/g_serial.ko",
        ] {
            assert!(out.join(rel).exists(), "missing {rel}");
        }
        assert_eq!(fs::metadata(out.join("panel/imagen.4bpp")).unwrap().len(), 240_000);

        let manifest = fs::read_to_string(out.join("MANIFEST.sha256")).unwrap();
        assert!(manifest.lines().count() >= 8);
        for line in manifest.lines() {
            let (sha, rel) = line.split_once("  ").unwrap();
            let data = fs::read(out.join(rel)).unwrap();
            let mut h = Sha256::new();
            h.update(&data);
            assert_eq!(hex::encode(h.finalize()), sha, "hash mismatch for {rel}");
        }
        let orden = fs::read_to_string(out.join("ordenes/21_usb.sh")).unwrap();
        assert!(orden.contains("inkd") && !orden.contains("stencild"));
        fs::remove_dir_all(&out).ok();
    }
}
