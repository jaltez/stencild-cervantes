//! Tray icon: real icon on Windows (feature `tray`), headless selftest everywhere.

pub const ICON_PNG: &[u8] = include_bytes!("../../../assets/tray-icon.png");

/// Validate the embedded icon assets without a display.
pub fn selftest() -> Result<(), String> {
    let img = image::load_from_memory(ICON_PNG).map_err(|e| format!("tray icon decode: {e}"))?;
    if img.width() < 16 || img.height() < 16 {
        return Err("tray icon too small".into());
    }
    println!("tray: icon {}x{} ok", img.width(), img.height());
    #[cfg(all(feature = "tray", windows))]
    {
        // Actual tray creation requires an event loop; the icon asset is what
        // the selftest can validate deterministically. Runtime tray uses
        // tray-icon with the same embedded image.
        println!("tray: windows build has tray support (feature enabled)");
    }
    #[cfg(not(all(feature = "tray", windows)))]
    {
        println!("tray: no runtime tray on this platform/build (assets ok)");
    }
    Ok(())
}

#[cfg(all(feature = "tray", windows))]
#[allow(dead_code)]
pub fn spawn() {
    // Kept minimal: create the icon and park the thread. Full menu/message
    // integration can be added without touching the service core.
    use std::time::Duration;
    std::thread::spawn(|| {
        let img = image::load_from_memory(ICON_PNG).expect("icon");
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        let icon = tray_icon::Icon::from_rgba(rgba.into_raw(), w, h).expect("tray icon rgba");
        let tray = tray_icon::TrayIconBuilder::new()
            .with_tooltip("stencild")
            .with_icon(icon)
            .build();
        if let Ok(_tray) = tray {
            loop {
                std::thread::sleep(Duration::from_secs(3600));
            }
        }
    });
}
