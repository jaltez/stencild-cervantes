//! Tray icon + menu: resident control of the Stencild service.
//!
//! Windows: real tray (icon, menu, live status) via `tray-icon`/`muda`.
//! The process detaches from its console (`FreeConsole`), so it survives
//! the launching terminal closing and works as a double-click target.
//! Other platforms: headless — the HTTP service still runs, the tray is a
//! no-op loop, so `stencild tray` behaves like `serve` there.
//!
//! Everything menu-independent (labels, status text, actions) lives in
//! plain functions so it is unit-testable without a display.

#[cfg(all(feature = "tray", windows))]
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use crate::service::Service;

pub const ICON_PNG: &[u8] = include_bytes!("../../../assets/tray-icon.png");

/// One-line tray status, also used as the tooltip.
#[cfg_attr(not(all(feature = "tray", windows)), allow(dead_code))]
pub fn status_text(online: bool, paused: bool, pushes: u64, last: &str) -> String {
    let state = if paused {
        "paused"
    } else if online {
        "online"
    } else {
        "offline"
    };
    if last.is_empty() {
        format!("stencild {state} · {pushes} pushes")
    } else {
        format!("stencild {state} · {pushes} pushes · last: {last}")
    }
}

/// Menu label for the pause/resume toggle.
#[cfg_attr(not(all(feature = "tray", windows)), allow(dead_code))]
pub fn pause_label(paused: bool) -> &'static str {
    if paused { "Resume" } else { "Pause" }
}

/// Validate the embedded icon assets without a display.
pub fn selftest() -> Result<(), String> {
    let img = image::load_from_memory(ICON_PNG).map_err(|e| format!("tray icon decode: {e}"))?;
    if img.width() < 16 || img.height() < 16 {
        return Err("tray icon too small".into());
    }
    println!("tray: icon {}x{} ok", img.width(), img.height());
    Ok(())
}

/// Report a fatal tray-mode error. On Windows this is a message box
/// (the console may already be gone); elsewhere it is plain stderr.
pub fn fatal(msg: &str) {
    #[cfg(all(feature = "tray", windows))]
    win::alert_error(msg);
    #[cfg(not(all(feature = "tray", windows)))]
    eprintln!("{msg}");
}

/// Run the tray loop until Quit/Restart. Blocks the calling thread.
pub fn run(service: Arc<Service>) {
    cfg_run(service);
}

#[cfg(all(feature = "tray", windows))]
mod win {
    /// Full argv, captured at first use for Restart (respawn with same
    /// arguments).
    pub static ARGS: std::sync::LazyLock<Vec<String>> =
        std::sync::LazyLock::new(|| std::env::args().collect());


    /// Detach from the launching console so closing the terminal (or the
    /// double-click console flash) does not kill the resident process.
    pub fn detach_from_console() {
        use windows_sys::Win32::System::Console::FreeConsole;
        unsafe { FreeConsole() };
    }

    pub fn alert_error(msg: &str) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            MessageBoxA, MB_ICONERROR, MB_OK,
        };
        let mut text = msg.to_string();
        text.push('\0');
        let title = b"stencild\0";
        unsafe {
            MessageBoxA(
                std::ptr::null_mut(),
                text.as_ptr(),
                title.as_ptr(),
                MB_OK | MB_ICONERROR,
            );
        }
    }

    /// Spawn a fresh detached copy of this program with the same arguments
    /// (used by Restart and by update-then-restart).
    pub fn spawn_detached() -> std::io::Result<()> {
        use std::os::windows::process::CommandExt;
        use windows_sys::Win32::System::Threading::DETACHED_PROCESS;
        let exe = std::env::current_exe()?;
        std::process::Command::new(exe)
            .args(ARGS.iter().skip(1))
            .creation_flags(DETACHED_PROCESS)
            .spawn()
            .map(|_| ())
    }
}

#[cfg(all(feature = "tray", windows))]
fn cfg_run(service: Arc<Service>) {
    use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
    use tray_icon::{TrayIcon, TrayIconBuilder};

    win::detach_from_console();

    let img = image::load_from_memory(ICON_PNG).expect("icon");
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let icon = tray_icon::Icon::from_rgba(rgba.into_raw(), w, h).expect("tray icon rgba");

    let status = MenuItem::with_id(MenuId::new("status"), "stencild", false, None);
    let pm = MenuItem::with_id(MenuId::new("pm"), pause_label(false), true, None);
    let clear = MenuItem::with_id(MenuId::new("clear"), "Clear panel (boot image)", true, None);
    let restart = MenuItem::with_id(MenuId::new("restart"), "Restart", true, None);
    let update = MenuItem::with_id(MenuId::new("update"), "Check for update…", true, None);
    let quit = MenuItem::with_id(MenuId::new("quit"), "Quit", true, None);

    let menu = Menu::new();
    menu.append(&status).expect("menu status");
    menu.append(&PredefinedMenuItem::separator()).expect("menu sep");
    menu.append(&pm).expect("menu pm");
    menu.append(&clear).expect("menu clear");
    menu.append(&PredefinedMenuItem::separator()).expect("menu sep2");
    menu.append(&restart).expect("menu restart");
    menu.append(&update).expect("menu update");
    menu.append(&quit).expect("menu quit");

    let refresh = |tray: &TrayIcon| {
        let online = service.device.is_open() && service.device.ping();
        let text = status_text(
            online,
            service.is_paused(),
            service.pushes.load(Ordering::Relaxed),
            &service.last.lock().unwrap(),
        );
        let _ = status.set_text(text.clone());
        let _ = pm.set_text(pause_label(service.is_paused()));
        tray.set_tooltip(Some(&text)).ok();
    };

    let tray = TrayIconBuilder::new()
        .with_tooltip("stencild")
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .build()
        .expect("tray icon");
    refresh(&tray);

    loop {
        // Menu events (crossbeam channel); 2 s tick refreshes status/tooltip.
        match MenuEvent::receiver().recv_timeout(Duration::from_secs(2)) {
            Ok(ev) => match ev.id().as_ref() {
                "pm" => {
                    service.set_paused(!service.is_paused());
                }
                "clear" => {
                    std::thread::spawn({
                        let svc = Arc::clone(&service);
                        move || {
                            let r = svc.clear_panel();
                            *svc.last.lock().unwrap() = r.unwrap_or_else(|e| e);
                        }
                    });
                }
                "restart" => {
                    drop(tray);
                    if let Err(e) = win::spawn_detached() {
                        win::alert_error(&format!("restart failed: {e}"));
                    }
                    std::process::exit(0);
                }
                "update" => {
                    std::thread::spawn({
                        let svc = Arc::clone(&service);
                        move || {
                            let (msg, applied) = check_for_update();
                            *svc.last.lock().unwrap() = msg.clone();
                            if applied {
                                std::thread::sleep(Duration::from_secs(3));
                                if let Err(e) = win::spawn_detached() {
                                    *svc.last.lock().unwrap() =
                                        format!("restart after update failed: {e}");
                                } else {
                                    std::process::exit(0);
                                }
                            }
                        }
                    });
                }
                "quit" => {
                    drop(tray);
                    std::process::exit(0);
                }
                _ => {}
            },
            Err(_) => {} // timeout: fall through to refresh
        }
        refresh(&tray);
    }
}

/// Look for `manifest.json` next to the running binary and apply it.
/// Returns (user message, whether a restart is needed to run the update).
#[cfg(all(feature = "tray", windows))]
fn check_for_update() -> (String, bool) {
    let Ok(exe) = std::env::current_exe() else {
        return ("update: cannot locate running exe".to_string(), false);
    };
    let manifest = exe.parent().map(|d| d.join("manifest.json"));
    match manifest {
        Some(p) if p.exists() => match crate::update::apply_to(&p, &exe, false) {
            Ok(msg) => {
                let applied = msg.starts_with("updated to");
                (msg, applied)
            }
            Err(e) => (format!("update: {e}"), false),
        },
        _ => ("update: no manifest.json next to exe".to_string(), false),
    }
}

#[cfg(not(all(feature = "tray", windows)))]
fn cfg_run(_service: Arc<Service>) {
    println!("tray: no tray UI on this platform/build; HTTP service stays up (Ctrl-C to quit)");
    loop {
        std::thread::sleep(Duration::from_secs(3600));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_text_covers_states() {
        assert_eq!(status_text(true, false, 3, ""), "stencild online · 3 pushes");
        assert_eq!(status_text(true, true, 3, ""), "stencild paused · 3 pushes");
        assert_eq!(
            status_text(false, false, 0, "ERR x"),
            "stencild offline · 0 pushes · last: ERR x"
        );
    }

    #[test]
    fn pause_label_toggles() {
        assert_eq!(pause_label(false), "Pause");
        assert_eq!(pause_label(true), "Resume");
    }
}
