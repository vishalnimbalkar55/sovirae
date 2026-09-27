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
        let _ = window.set_focusable(keyable);
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

/// Drops the `\\?\` prefix Windows adds to canonical and resource paths.
/// Explorer, Chrome's folder picker, and people expect `C:\...`.
pub fn plain_path(path: std::path::PathBuf) -> std::path::PathBuf {
    let s = path.to_string_lossy();
    if let Some(unc) = s.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}").into()
    } else if let Some(rest) = s.strip_prefix(r"\\?\") {
        rest.into()
    } else {
        path
    }
}

/// Opens `folder` in Finder, Explorer, or the desktop's file manager.
pub fn open_folder(folder: &std::path::Path) -> std::io::Result<()> {
    let opener = if cfg!(target_os = "macos") {
        "/usr/bin/open"
    } else if cfg!(windows) {
        "explorer.exe"
    } else {
        "xdg-open"
    };
    std::process::Command::new(opener).arg(folder).spawn().map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::plain_path;
    use std::path::PathBuf;

    #[test]
    fn strips_the_verbatim_prefix() {
        assert_eq!(plain_path(PathBuf::from(r"\\?\C:\Users\me\Sovirae\ext")), PathBuf::from(r"C:\Users\me\Sovirae\ext"));
        assert_eq!(plain_path(PathBuf::from(r"\\?\UNC\server\share\ext")), PathBuf::from(r"\\server\share\ext"));
        assert_eq!(plain_path(PathBuf::from("/Applications/Sovirae.app")), PathBuf::from("/Applications/Sovirae.app"));
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "writes and removes HKCU\\Software\\Sovirae-test"]
    fn registry_default_value_round_trips() {
        use super::windows::{reg_delete, reg_get_default, reg_set_default};
        let key = r"Software\Sovirae-test\NativeMessagingHosts\com.sovirae.bridge";
        reg_set_default(key, r"C:\Users\Zoë\AppData\Local\host.json").unwrap();
        assert_eq!(reg_get_default(key).as_deref(), Some(r"C:\Users\Zoë\AppData\Local\host.json"));
        reg_delete(r"Software\Sovirae-test");
        assert_eq!(reg_get_default(key), None);
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
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Networking::WinSock::{WSAIoctl, SOCKET};
    use windows::Win32::Security::{GetLengthSid, GetTokenInformation, TokenUser, TOKEN_QUERY, TOKEN_USER};
    use windows::Win32::System::Threading::{
        GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, HWND_TOP, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE};

    /// Visibility and focusability go through Tauri so its window state
    /// stays true: it skips `hide()` for a window it thinks is hidden, and
    /// rewrites the window styles from that state on resize or topmost
    /// changes. The player is created with `focus: false`, so Tauri shows
    /// it with `SW_SHOWNOACTIVATE`; not focusable adds `WS_EX_NOACTIVATE`
    /// so clicks keep typing in the user's application.
    pub fn show_no_activate<R: Runtime>(w: &WebviewWindow<R>) {
        if !w.is_visible().unwrap_or(false) {
            let _ = w.set_focusable(false);
        }
        let _ = w.show();
        let Ok(hwnd) = w.hwnd() else { return };
        // SAFETY: plain window call on a live window handle. HWND_TOP raises
        // it within its band and leaves the topmost setting alone.
        unsafe {
            let _ = SetWindowPos(hwnd, Some(HWND_TOP), 0, 0, 0, 0, SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE);
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

    // ---- Per-user registry (HKEY_CURRENT_USER) --------------------------------

    use windows::core::HSTRING;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegGetValueW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
        KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ, RRF_RT_REG_SZ,
    };

    /// Sets the default value of `HKCU\<subkey>`, creating the key.
    pub fn reg_set_default(subkey: &str, value: &str) -> Result<(), String> {
        let mut key = HKEY::default();
        // SAFETY: out-pointer to a local HKEY; the key is closed below.
        let rc = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER, &HSTRING::from(subkey), None, None, REG_OPTION_NON_VOLATILE, KEY_SET_VALUE, None,
                &mut key, None,
            )
        };
        if rc != ERROR_SUCCESS {
            return Err(format!("could not create HKCU\\{subkey}: {rc:?}"));
        }
        // REG_SZ data: UTF-16 with a terminating NUL, as bytes.
        let data: Vec<u8> = value.encode_utf16().chain([0]).flat_map(u16::to_le_bytes).collect();
        // SAFETY: open key; `data` outlives the call.
        let rc = unsafe { RegSetValueExW(key, None, None, REG_SZ, Some(&data)) };
        // SAFETY: the key was opened above.
        unsafe {
            let _ = RegCloseKey(key);
        }
        if rc != ERROR_SUCCESS {
            return Err(format!("could not write HKCU\\{subkey}: {rc:?}"));
        }
        Ok(())
    }

    /// The default value of `HKCU\<subkey>`, if the key exists.
    pub fn reg_get_default(subkey: &str) -> Option<String> {
        let mut len = 0u32;
        // SAFETY: size query; no buffer.
        let rc = unsafe { RegGetValueW(HKEY_CURRENT_USER, &HSTRING::from(subkey), None, RRF_RT_REG_SZ, None, None, Some(&mut len)) };
        if rc != ERROR_SUCCESS || len == 0 {
            return None;
        }
        let mut buf = vec![0u16; (len as usize).div_ceil(2)];
        // SAFETY: `buf` holds `len` bytes.
        let rc = unsafe {
            RegGetValueW(HKEY_CURRENT_USER, &HSTRING::from(subkey), None, RRF_RT_REG_SZ, None, Some(buf.as_mut_ptr().cast()), Some(&mut len))
        };
        if rc != ERROR_SUCCESS {
            return None;
        }
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Some(String::from_utf16_lossy(&buf[..end]))
    }

    /// Deletes `HKCU\<subkey>` and everything under it.
    pub fn reg_delete(subkey: &str) {
        // SAFETY: plain registry call.
        unsafe {
            let _ = RegDeleteTreeW(HKEY_CURRENT_USER, &HSTRING::from(subkey));
        }
        // RegDeleteTreeW empties the key; this removes the key itself.
        unsafe {
            let _ = windows::Win32::System::Registry::RegDeleteKeyW(HKEY_CURRENT_USER, &HSTRING::from(subkey));
        }
    }
}
