//! Self-update from a local file:// manifest with sha256 verification.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    /// `file:///absolute/path` or a plain absolute path.
    pub url: String,
    pub sha256: String,
}

pub fn parse_manifest(text: &str) -> Result<Manifest, String> {
    serde_json::from_str(text).map_err(|e| format!("invalid manifest: {e}"))
}

pub fn file_from_url(url: &str) -> Result<std::path::PathBuf, String> {
    if let Some(rest) = url.strip_prefix("file://") {
        // file:///C:/x strips to "/C:/x" — drop the leading slash only
        // when a drive spec follows, so POSIX paths keep their root.
        let b = rest.as_bytes();
        let is_drive =
            b.len() >= 4 && b[0] == b'/' && b[2] == b':' && (b[3] == b'/' || b[3] == b'\\');
        let path = if is_drive { &rest[1..] } else { rest };
        return Ok(std::path::PathBuf::from(path.replace('\\', "/")));
    }
    if url.starts_with('/') {
        return Ok(std::path::PathBuf::from(url));
    }
    // windows drive path like C:/...
    let bytes = url.as_bytes();
    if bytes.len() >= 3 && bytes[1] == b':' && (bytes[2] == b'/' || bytes[2] == b'\\') {
        return Ok(std::path::PathBuf::from(url.replace('\\', "/")));
    }
    Err(format!("only local file:// URLs are supported: {url}"))
}

pub fn sha256_file(path: &Path) -> Result<String, String> {
    let data = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut h = Sha256::new();
    h.update(&data);
    Ok(hex::encode(h.finalize()))
}

/// Install an update over `target` (the binary file path). When `selftest` is
/// set, the resulting binary is executed with `--version` and must report the
/// manifest version. Handles the Windows case where `target` is the running
/// executable (rename-the-live-exe dance).
pub fn apply_to(manifest_path: &Path, target: &Path, selftest: bool) -> Result<String, String> {
    let text = fs::read_to_string(manifest_path).map_err(|e| format!("manifest: {e}"))?;
    let m = parse_manifest(&text)?;
    let actual = env!("CARGO_PKG_VERSION");
    if m.version == actual {
        return Ok(format!("already up to date ({actual}); nothing to do"));
    }
    let src = file_from_url(&m.url)?;
    let digest = sha256_file(&src)?;
    if digest != m.sha256.to_lowercase() {
        return Err(format!("sha256 mismatch for {}: {digest} != {}", src.display(), m.sha256));
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
    }
    let staging = sibling(target, "new");
    fs::copy(&src, &staging).map_err(|e| format!("stage: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&staging, fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("chmod: {e}"))?;
    }

    // Windows cannot rename over an existing executable (locked while
    // running), but renaming the occupied name away IS allowed: park the
    // current target as *.old, then move the staged one into place. This
    // covers both self-update and updating another installed copy.
    #[cfg(windows)]
    if target.exists() {
        let old = sibling(target, "old");
        let _ = fs::remove_file(&old);
        fs::rename(target, &old).map_err(|e| format!("park current exe: {e}"))?;
    }

    fs::rename(&staging, target).map_err(|e| format!("atomic replace: {e}"))?;

    if selftest {
        let out = Command::new(target)
            .arg("--version")
            .output()
            .map_err(|e| format!("selftest exec: {e}"))?;
        let version_line = String::from_utf8_lossy(&out.stdout).to_string();
        if !out.status.success() || !version_line.contains(&m.version) {
            return Err(format!(
                "selftest failed: status {:?}, output {:?}",
                out.status.code(),
                version_line.trim()
            ));
        }
    }
    Ok(format!("updated to {} at {}", m.version, target.display()))
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let name = path.file_name().map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "stencild".into());
    path.with_file_name(format!("{name}.{suffix}"))
}

/// Apply an update into `prefix/bin/stencild[.exe]`.
pub fn apply(manifest_path: &Path, prefix: &Path, selftest: bool) -> Result<String, String> {
    let name = if cfg!(windows) { "stencild.exe" } else { "stencild" };
    apply_to(manifest_path, &prefix.join("bin").join(name), selftest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn manifest_requires_matching_sha256() {
        let dir = std::env::temp_dir().join(format!("stencild-upd-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let payload = dir.join("payload.bin");
        let mut f = fs::File::create(&payload).unwrap();
        f.write_all(b"new-binary").unwrap();
        drop(f);
        let digest = sha256_file(&payload).unwrap();

        let good = dir.join("good.json");
        fs::write(
            &good,
            format!(r#"{{"version":"999.0.0","url":"file://{}","sha256":"{}"}}"#, payload.display(), digest),
        )
        .unwrap();
        let prefix = dir.join("install");
        fs::create_dir_all(&prefix).unwrap();

        // sha mismatch must fail before touching the target
        let bad = dir.join("bad.json");
        fs::write(
            &bad,
            format!(r#"{{"version":"999.0.0","url":"file://{}","sha256":"{}"}}"#, payload.display(), "0".repeat(64)),
        )
        .unwrap();
        assert!(apply(&bad, &prefix, false).is_err());

        // matching sha applies atomically
        let msg = apply(&good, &prefix, false).unwrap();
        assert!(msg.contains("999.0.0"));
        assert_eq!(fs::read(prefix.join("bin/stencild")).unwrap(), b"new-binary");
    }

    #[test]
    fn rejects_non_local_urls() {
        assert!(file_from_url("https://example.com/x").is_err());
        assert!(file_from_url("file:///tmp/x").is_ok());
    }

    #[test]
    fn windows_file_urls_lose_only_the_extra_slash() {
        assert_eq!(
            file_from_url("file:///C:/Users/x/stencild.exe").unwrap(),
            std::path::PathBuf::from("C:/Users/x/stencild.exe")
        );
        assert_eq!(
            file_from_url("file:///tmp/x").unwrap(),
            std::path::PathBuf::from("/tmp/x")
        );
    }
}
