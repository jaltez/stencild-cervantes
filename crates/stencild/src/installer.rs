//! Installer artifact generation (Linux shell installer + Windows notes).

use std::fs;
use std::path::Path;

const INSTALL_SH: &str = r#"#!/bin/sh
# stencild installer (generated). Usage: sh install.sh --prefix DIR
set -e
PREFIX="${HOME}/.local"
while [ $# -gt 0 ]; do
  case "$1" in
    --prefix) PREFIX="$2"; shift 2 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done
HERE="$(cd "$(dirname "$0")" && pwd)"
mkdir -p "$PREFIX/bin" "$PREFIX/share/stencild"
cp "$HERE/stencild" "$PREFIX/bin/stencild"
chmod +x "$PREFIX/bin/stencild"
cp "$HERE/tray-icon.png" "$PREFIX/share/stencild/tray-icon.png"
echo "installed: $PREFIX/bin/stencild"
"#;

const INSTALL_PS1: &str = r#"# stencild installer (generated, Windows). Run in PowerShell:
#   .\install.ps1 -Prefix C:\Users\you\AppData\Local\stencild
param([string]$Prefix = "$env:LOCALAPPDATA\stencild")
New-Item -ItemType Directory -Force "$Prefix\bin" | Out-Null
Copy-Item "$PSScriptRoot\stencild.exe" "$Prefix\bin\stencild.exe" -Force
Copy-Item "$PSScriptRoot\tray-icon.png" "$Prefix\bin\tray-icon.png" -Force
Write-Host "installed: $Prefix\bin\stencild.exe"
"#;

/// Emit installer files into `out`. Never executes anything.
pub fn emit(out: &Path) -> Result<(), String> {
    fs::create_dir_all(out).map_err(|e| format!("mkdir {}: {e}", out.display()))?;
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    let dest_name = if cfg!(windows) { "stencild.exe" } else { "stencild" };
    fs::copy(&exe, out.join(dest_name)).map_err(|e| format!("copy binary: {e}"))?;
    fs::write(out.join("install.sh"), INSTALL_SH).map_err(|e| format!("install.sh: {e}"))?;
    fs::write(out.join("install.ps1"), INSTALL_PS1).map_err(|e| format!("install.ps1: {e}"))?;
    fs::write(out.join("tray-icon.png"), crate::tray::ICON_PNG)
        .map_err(|e| format!("tray icon: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(out.join("install.sh"), fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("chmod install.sh: {e}"))?;
    }
    Ok(())
}
