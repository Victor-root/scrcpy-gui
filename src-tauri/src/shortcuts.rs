//! Cross-platform global OS shortcuts. `tauri_plugin_global_shortcut` only
//! supports one plugin instance (and so one dispatch handler) per app, so this
//! module owns the shared plugin/handler and dispatches by shortcut identity,
//! rather than each feature registering its own plugin.
//!
//! Windows' borderless-window drag (Ctrl+Alt+Shift+W, see `grab.rs`) is the
//! other shortcut sharing this plugin; its own toggle is exposed as a plain
//! function so this module can dispatch into it without owning any of its
//! drag-follow implementation.

use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::ScrcpyState;

/// Ctrl+Alt+Shift+C: snap the active device's mirror window back to the
/// centre of the screen. A true OS-level global shortcut, so it works
/// regardless of which window (if any) currently has keyboard focus -- unlike
/// a webview `keydown` listener, which only fires while the app's own window
/// is focused.
fn recenter_shortcut() -> Shortcut {
    Shortcut::new(
        Some(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT),
        Code::KeyC,
    )
}

/// Ctrl+Alt+Shift+S: screenshot the active device's mirror window (client
/// area only, no window borders) straight to the clipboard, ready to paste
/// elsewhere. Same true OS-level global shortcut as the others in this file.
fn screenshot_shortcut() -> Shortcut {
    Shortcut::new(
        Some(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT),
        Code::KeyS,
    )
}

/// The global-shortcut plugin, wired to dispatch every registered shortcut by
/// identity. Add this on the Tauri builder (its setup wires the OS hotkey
/// manager on the main thread).
pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            if shortcut == &recenter_shortcut() {
                recenter_active_device(app);
            }
            if shortcut == &screenshot_shortcut() {
                screenshot_active_device(app);
            }
            #[cfg(target_os = "windows")]
            if shortcut == &crate::grab::toggle_shortcut() {
                crate::grab::toggle();
            }
        })
        .build()
}

/// Register every global shortcut. Call from the app's `setup` (main thread).
pub fn register<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    app.global_shortcut().register(recenter_shortcut())?;
    app.global_shortcut().register(screenshot_shortcut())?;
    #[cfg(target_os = "windows")]
    app.global_shortcut().register(crate::grab::toggle_shortcut())?;
    Ok(())
}

fn recenter_active_device<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<ScrcpyState>();
    let device = state.active_device.lock().unwrap().clone();
    if let Some(device) = device {
        crate::commands::recenter_device(&state, &device);
    }
}

fn screenshot_active_device<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<ScrcpyState>();
    let device = state.active_device.lock().unwrap().clone();
    let Some(device) = device else { return };
    let ok = crate::commands::screenshot_device(&state, &device);
    let message = if ok {
        "[SYSTEM] Screenshot copied to clipboard"
    } else {
        "[SYSTEM] Screenshot failed"
    };
    let _ = app.emit("scrcpy-log", message);
}
