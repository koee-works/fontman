//! Windows activation.
//!
//! Two steps, and skipping either produces a half-working result:
//!
//! 1. `AddFontResourceExW` without `FR_PRIVATE` makes the font visible to the
//!    whole session. With `FR_PRIVATE` it would be visible only to our own
//!    process, which is useless for a font manager.
//! 2. A value under `HKCU\...\Fonts` makes it survive a reboot. Per-user font
//!    installation without admin rights landed in Windows 10 1809; below that
//!    this key is ignored and activation is session-only.
//!
//! Then broadcast `WM_FONTCHANGE` or nothing notices until relaunch.

use super::{ActivationError, FontActivator, Result};
use std::ffi::OsStr;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Gdi::{
    AddFontResourceExW, RemoveFontResourceExW, FONT_RESOURCE_CHARACTERISTICS, FR_NOT_ENUM,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegEnumValueW, RegOpenKeyExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_SZ,
};
use windows::Win32::UI::WindowsAndMessaging::{
    SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_FONTCHANGE,
};

const FONTS_KEY: &str = r"Software\Microsoft\Windows NT\CurrentVersion\Fonts";

fn wide(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(std::iter::once(0)).collect()
}

pub struct WindowsActivator;

impl WindowsActivator {
    pub fn new() -> Self {
        Self
    }

    fn open_fonts_key(&self, write: bool) -> Result<HKEY> {
        let path = wide(OsStr::new(FONTS_KEY));
        let access = if write { KEY_READ | KEY_WRITE } else { KEY_READ };
        let mut key = HKEY::default();
        unsafe {
            RegOpenKeyExW(HKEY_CURRENT_USER, PCWSTR(path.as_ptr()), 0, access, &mut key)
        }
        .ok()
        .map_err(|e| ActivationError::Refused {
            path: PathBuf::from(FONTS_KEY),
            reason: format!("cannot open HKCU Fonts key: {e}"),
        })?;
        Ok(key)
    }

    /// Registry value name. Windows conventionally uses the font's display
    /// name; we use the file stem, which is unique in our cache and avoids
    /// clobbering a system-installed font's value.
    fn value_name(path: &Path) -> String {
        let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        format!("{stem} (FontManager)")
    }
}

impl Default for WindowsActivator {
    fn default() -> Self {
        Self::new()
    }
}

impl FontActivator for WindowsActivator {
    fn register(&self, path: &Path) -> Result<()> {
        let wpath = wide(path.as_os_str());

        // 0 flags = session-wide and enumerable. FR_NOT_ENUM would hide it
        // from font pickers, which is the opposite of what we want.
        let count = unsafe { AddFontResourceExW(PCWSTR(wpath.as_ptr()), FONT_RESOURCE_CHARACTERISTICS(0), None) };
        let _ = FR_NOT_ENUM;
        if count == 0 {
            return Err(ActivationError::Refused {
                path: path.to_path_buf(),
                reason: "AddFontResourceExW added no faces".into(),
            });
        }

        // Persist across reboot. Value data is the full path (allowed since
        // per-user fonts; system-wide entries store a bare filename).
        let key = self.open_fonts_key(true)?;
        let name = wide(OsStr::new(&Self::value_name(path)));
        let data: Vec<u16> = wpath.clone();
        let bytes = unsafe {
            std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 2)
        };
        let result = unsafe { RegSetValueExW(key, PCWSTR(name.as_ptr()), 0, REG_SZ, Some(bytes)) };
        unsafe { RegCloseKey(key) }.ok().ok();
        result.ok().map_err(|e| ActivationError::Refused {
            path: path.to_path_buf(),
            reason: format!("registry write failed: {e}"),
        })?;

        Ok(())
    }

    fn unregister(&self, path: &Path) -> Result<()> {
        let wpath = wide(path.as_os_str());

        // GDI reference-counts registrations; loop until it reports none left,
        // otherwise a font registered twice stays active after one removal.
        let mut guard = 0;
        while unsafe { RemoveFontResourceExW(PCWSTR(wpath.as_ptr()), 0, None) }.as_bool() {
            guard += 1;
            if guard > 64 {
                break;
            }
        }

        let key = self.open_fonts_key(true)?;
        let name = wide(OsStr::new(&Self::value_name(path)));
        unsafe { RegDeleteValueW(key, PCWSTR(name.as_ptr())) }.ok().ok();
        unsafe { RegCloseKey(key) }.ok().ok();

        Ok(())
    }

    fn list_active(&self) -> Result<Vec<PathBuf>> {
        let key = self.open_fonts_key(false)?;
        let mut out = Vec::new();
        let mut index = 0u32;

        loop {
            let mut name = vec![0u16; 512];
            let mut name_len = name.len() as u32;
            let mut data = vec![0u8; 2048];
            let mut data_len = data.len() as u32;
            let mut kind = REG_SZ;

            let status = unsafe {
                RegEnumValueW(
                    key,
                    index,
                    windows::core::PWSTR(name.as_mut_ptr()),
                    &mut name_len,
                    None,
                    Some(&mut kind.0),
                    Some(data.as_mut_ptr()),
                    Some(&mut data_len),
                )
            };
            if status.is_err() {
                break; // ERROR_NO_MORE_ITEMS, or a read we cannot use
            }

            let value_name = String::from_utf16_lossy(&name[..name_len as usize]);
            if value_name.ends_with("(FontManager)") {
                let words: Vec<u16> = data[..data_len as usize]
                    .chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .take_while(|&w| w != 0)
                    .collect();
                out.push(PathBuf::from(std::ffi::OsString::from_wide(&words)));
            }
            index += 1;
        }

        unsafe { RegCloseKey(key) }.ok().ok();
        Ok(out)
    }

    fn notify_change(&self) -> Result<()> {
        // Timeout, not a plain SendMessage: a single hung top-level window
        // would otherwise block the whole activation batch indefinitely.
        unsafe {
            SendMessageTimeoutW(
                HWND_BROADCAST,
                WM_FONTCHANGE,
                WPARAM(0),
                LPARAM(0),
                SMTO_ABORTIFHUNG,
                1000,
                None,
            );
        }
        let _ = HWND::default();
        Ok(())
    }
}
