use std::ffi::CString;

use anyhow::{anyhow, Result};
use x11::xlib;

/// Ref: <https://xwindow.angelfire.com/page2.html>
/// Ref: <https://www.oreilly.com/library/view/xlib-reference-manual/9780937175262/14_appendix-f.html>
pub struct X11 {
    display_ptr: *mut xlib::Display,
}

impl X11 {
    pub fn open() -> Result<Self> {
        let display_ptr = unsafe { xlib::XOpenDisplay(std::ptr::null()) };
        if display_ptr.is_null() {
            Err(anyhow!("XOpenDisplay failed"))
        } else {
            Ok(Self { display_ptr })
        }
    }

    pub fn set_root_window_name(&self, name: &str) -> Result<()> {
        let screen = unsafe { xlib::XDefaultScreen(self.display_ptr) };
        let window = unsafe { xlib::XRootWindow(self.display_ptr, screen) };
        let name = CString::new(name)?;
        unsafe { xlib::XStoreName(self.display_ptr, window, name.as_ptr()) };
        Ok(())
    }
}

impl Drop for X11 {
    fn drop(&mut self) {
        unsafe {
            xlib::XCloseDisplay(self.display_ptr);
        }
    }
}
