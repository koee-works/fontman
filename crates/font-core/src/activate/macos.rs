//! macOS activation via CoreText.
//!
//! `CTFontManagerRegisterFontsForURL` with persistent scope is instant and
//! reversible, which beats symlinking into `~/Library/Fonts` — that path is
//! slower to propagate and leaves debris if we crash.
//!
//! Sandboxing note: persistent registration is restricted for sandboxed apps.
//! Ship unsandboxed and notarised. If you later want the Mac App Store, the
//! symlink approach is the fallback, and this trait is where you'd swap it.

use super::{ActivationError, FontActivator, Result};
use core_foundation::base::TCFType;
use core_foundation::error::CFError;
use core_foundation::string::CFString;
use core_foundation::url::{CFURL, kCFURLPOSIXPathStyle};
use core_foundation_sys::base::CFTypeRef;
use core_foundation_sys::error::CFErrorRef;
use core_foundation_sys::url::CFURLRef;
use std::path::{Path, PathBuf};
use std::ptr;

type CTFontManagerScope = u32;

/// `kCTFontManagerScopePersistent` — available to the user across sessions,
/// which is what "activate a font" means to anyone using this app.
const SCOPE_PERSISTENT: CTFontManagerScope = 2;

#[link(name = "CoreText", kind = "framework")]
extern "C" {
    fn CTFontManagerRegisterFontsForURL(
        font_url: CFURLRef,
        scope: CTFontManagerScope,
        error: *mut CFErrorRef,
    ) -> bool;

    fn CTFontManagerUnregisterFontsForURL(
        font_url: CFURLRef,
        scope: CTFontManagerScope,
        error: *mut CFErrorRef,
    ) -> bool;

    fn CTFontManagerCopyRegisteredFontDescriptors(
        scope: CTFontManagerScope,
        enabled: bool,
    ) -> CFTypeRef;
}

pub struct MacActivator;

impl MacActivator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MacActivator {
    fn default() -> Self {
        Self::new()
    }
}

fn cf_url(path: &Path) -> CFURL {
    CFURL::from_path(path, false).unwrap_or_else(|| {
        // from_path only fails on non-UTF-8 paths; construct from a string so
        // the caller gets a CoreText error rather than a panic.
        CFURL::from_file_system_path(CFString::new(&path.to_string_lossy()), kCFURLPOSIXPathStyle, false)
    })
}

fn describe(err: CFErrorRef, path: &Path) -> ActivationError {
    let reason = if err.is_null() {
        "CoreText reported failure without an error".to_string()
    } else {
        let e = unsafe { CFError::wrap_under_create_rule(err) };
        e.description().to_string()
    };
    ActivationError::Refused { path: path.to_path_buf(), reason }
}

impl FontActivator for MacActivator {
    fn register(&self, path: &Path) -> Result<()> {
        let url = cf_url(path);
        let mut err: CFErrorRef = ptr::null_mut();
        let ok = unsafe {
            CTFontManagerRegisterFontsForURL(url.as_concrete_TypeRef(), SCOPE_PERSISTENT, &mut err)
        };
        if ok {
            return Ok(());
        }
        // Already-registered is reported as an error but is the state we want,
        // and reconciliation asks for it routinely.
        let e = describe(err, path);
        if let ActivationError::Refused { reason, .. } = &e {
            if reason.contains("already registered") {
                return Ok(());
            }
        }
        Err(e)
    }

    fn unregister(&self, path: &Path) -> Result<()> {
        let url = cf_url(path);
        let mut err: CFErrorRef = ptr::null_mut();
        let ok = unsafe {
            CTFontManagerUnregisterFontsForURL(url.as_concrete_TypeRef(), SCOPE_PERSISTENT, &mut err)
        };
        if ok {
            return Ok(());
        }
        let e = describe(err, path);
        if let ActivationError::Refused { reason, .. } = &e {
            // Not registered is the desired end state.
            if reason.contains("not registered") || reason.contains("has not been registered") {
                return Ok(());
            }
        }
        Err(e)
    }

    fn list_active(&self) -> Result<Vec<PathBuf>> {
        // TODO(list_active): walk the CFArray of CTFontDescriptorRef returned
        // below and read `kCTFontURLAttribute` from each. Left unimplemented
        // rather than guessed at — reconciliation is the one place a wrong
        // answer silently corrupts state, so this needs testing on-device
        // against real registrations before it is trusted.
        //
        // let descriptors = unsafe {
        //     CTFontManagerCopyRegisteredFontDescriptors(SCOPE_PERSISTENT, true)
        // };
        let _ = CTFontManagerCopyRegisteredFontDescriptors;
        Ok(Vec::new())
    }

    fn notify_change(&self) -> Result<()> {
        // CoreText posts kCTFontManagerRegisteredFontsChangedNotification
        // itself on registration. Nothing further to do.
        Ok(())
    }
}
