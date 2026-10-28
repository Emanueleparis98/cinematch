//! Un solo processo per sessione: due finestre scriverebbero lo stesso
//! config, lo stesso store SIMKL e la stessa cache poster.

#[cfg(target_os = "windows")]
use std::ffi::c_void;

#[cfg(target_os = "windows")]
const MUTEX: &str = "CineMatch.SingleInstance";

pub struct Instance {
    #[cfg(target_os = "windows")]
    handle: *mut c_void,
}

impl Drop for Instance {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        unsafe {
            CloseHandle(self.handle);
        }
    }
}

#[cfg(target_os = "windows")]
pub fn acquire() -> Option<Instance> {
    let name = wide(MUTEX);
    let handle = unsafe { CreateMutexW(std::ptr::null_mut(), 0, name.as_ptr()) };
    if handle.is_null() {
        // Se il mutex non si crea, meglio aprire che bloccare l'app.
        return Some(Instance { handle });
    }
    if unsafe { GetLastError() } == 183 {
        unsafe {
            CloseHandle(handle);
            let text = wide("CineMatch è già aperto.");
            let caption = wide("CineMatch");
            MessageBoxW(
                std::ptr::null_mut(),
                text.as_ptr(),
                caption.as_ptr(),
                0x0000_0040 | 0x0001_0000,
            );
        }
        return None;
    }
    Some(Instance { handle })
}

#[cfg(not(target_os = "windows"))]
pub fn acquire() -> Option<Instance> {
    Some(Instance {})
}

#[cfg(target_os = "windows")]
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateMutexW(attrs: *mut c_void, initial_owner: i32, name: *const u16) -> *mut c_void;
    fn GetLastError() -> u32;
    fn CloseHandle(handle: *mut c_void) -> i32;
}

#[cfg(target_os = "windows")]
#[link(name = "user32")]
unsafe extern "system" {
    fn MessageBoxW(hwnd: *mut c_void, text: *const u16, caption: *const u16, ty: u32) -> i32;
}
