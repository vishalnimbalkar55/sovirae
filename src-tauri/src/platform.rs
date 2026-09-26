//! Platform adapters for behavior Tauri does not expose directly (spec §5.4).

use tauri::{Runtime, WebviewWindow};

/// Shows the floating player without activating SpeakIt or taking typing
/// focus from the application the user is working in.
pub fn show_passive<R: Runtime>(window: &WebviewWindow<R>) {
    #[cfg(target_os = "macos")]
    {
        let w = window.clone();
        let _ = window.run_on_main_thread(move || macos::order_front(&w));
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window.show();
    }
}

/// Lets the expanded player take keyboard focus when the user explicitly
/// asks for it (spec §10.3, §16).
pub fn set_player_keyable<R: Runtime>(window: &WebviewWindow<R>, keyable: bool) {
    #[cfg(target_os = "macos")]
    {
        if keyable {
            let w = window.clone();
            let _ = window.run_on_main_thread(move || macos::make_key(&w));
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        if keyable {
            let _ = window.set_focus();
        }
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use objc2_app_kit::NSWindow;
    use tauri::{Runtime, WebviewWindow};

    fn ns_window<R: Runtime>(w: &WebviewWindow<R>) -> Option<&'static NSWindow> {
        let ptr = w.ns_window().ok()? as *const NSWindow;
        // SAFETY: Tauri returns the live NSWindow for this webview window;
        // it outlives these main-thread calls.
        unsafe { ptr.as_ref() }
    }

    pub fn order_front<R: Runtime>(w: &WebviewWindow<R>) {
        if let Some(win) = ns_window(w) {
            win.orderFrontRegardless();
        }
    }

    pub fn make_key<R: Runtime>(w: &WebviewWindow<R>) {
        if let Some(win) = ns_window(w) {
            win.makeKeyAndOrderFront(None);
        }
    }
}
