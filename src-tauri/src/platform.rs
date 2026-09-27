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
    #[cfg(windows)]
    windows::show_no_activate(window);
    #[cfg(not(any(target_os = "macos", windows)))]
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
    #[cfg(windows)]
    {
        windows::set_no_activate(window, !keyable);
        if keyable {
            let _ = window.set_focus();
        }
    }
    #[cfg(not(any(target_os = "macos", windows)))]
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

#[cfg(windows)]
pub mod windows {
    use tauri::{Runtime, WebviewWindow};
    use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND};
    use windows::Win32::Networking::WinSock::{WSAIoctl, SOCKET};
    use windows::Win32::Security::{GetLengthSid, GetTokenInformation, TokenUser, TOKEN_QUERY, TOKEN_USER};
    use windows::Win32::System::Threading::{
        GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE, HWND_TOPMOST, SWP_NOACTIVATE,
        SWP_NOMOVE, SWP_NOSIZE, SW_SHOWNOACTIVATE, WS_EX_NOACTIVATE,
    };

    /// Keeps clicks on the player from activating it, so typing stays in
    /// the user's application.
    pub fn set_no_activate<R: Runtime>(w: &WebviewWindow<R>, on: bool) {
        let Ok(hwnd) = w.hwnd() else { return };
        // SAFETY: `hwnd` is this live window; only the extended style changes.
        unsafe {
            let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            let flag = WS_EX_NOACTIVATE.0 as isize;
            let next = if on { style | flag } else { style & !flag };
            if next != style {
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next);
            }
        }
    }

    pub fn show_no_activate<R: Runtime>(w: &WebviewWindow<R>) {
        let Ok(hwnd) = w.hwnd() else { return };
        if !w.is_visible().unwrap_or(false) {
            set_no_activate(w, true);
        }
        show(hwnd);
    }

    fn show(hwnd: HWND) {
        // SAFETY: plain window calls on a live window handle.
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE);
        }
    }

    /// `SIO_AF_UNIX_GETPEERPID`: the process ID at the other end of an
    /// AF_UNIX socket (Windows 10 1803+).
    const SIO_AF_UNIX_GETPEERPID: u32 = 0x5800_0100;

    /// True when the process on the other end of the bridge socket runs as
    /// the same Windows user as Sovirae.
    pub fn peer_is_current_user(stream: &uds_windows::UnixStream) -> bool {
        use std::os::windows::io::AsRawSocket;
        let socket = SOCKET(stream.as_raw_socket() as usize);
        let mut pid: u32 = 0;
        let mut returned: u32 = 0;
        // SAFETY: valid socket; the output buffer is a u32 of the stated size.
        let rc = unsafe {
            WSAIoctl(
                socket,
                SIO_AF_UNIX_GETPEERPID,
                None,
                0,
                Some(&mut pid as *mut u32 as *mut _),
                std::mem::size_of::<u32>() as u32,
                &mut returned,
                None,
                None,
            )
        };
        if rc != 0 || pid == 0 {
            return false;
        }
        // SAFETY: handles are checked and closed; SIDs are copied out.
        unsafe {
            let Ok(peer) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return false };
            let theirs = user_sid(peer);
            let _ = CloseHandle(peer);
            match (theirs, user_sid(GetCurrentProcess())) {
                (Some(a), Some(b)) => a == b,
                _ => false,
            }
        }
    }

    /// The user SID of a process token, as bytes.
    unsafe fn user_sid(process: HANDLE) -> Option<Vec<u8>> {
        let mut token = HANDLE::default();
        OpenProcessToken(process, TOKEN_QUERY, &mut token).ok()?;
        let mut len = 0u32;
        let _ = GetTokenInformation(token, TokenUser, None, 0, &mut len);
        // u64 storage keeps TOKEN_USER suitably aligned.
        let mut buf = vec![0u64; (len as usize).div_ceil(8)];
        let ok = GetTokenInformation(token, TokenUser, Some(buf.as_mut_ptr().cast()), len, &mut len);
        let _ = CloseHandle(token);
        ok.ok()?;
        let sid = (*(buf.as_ptr() as *const TOKEN_USER)).User.Sid;
        let n = GetLengthSid(sid) as usize;
        Some(std::slice::from_raw_parts(sid.0 as *const u8, n).to_vec())
    }
}
