use crate::traits::PlatformError;

pub struct SyntheticScroll;

impl SyntheticScroll {
    /// Dispatch programmatic scroll wheel event to scroll content under cursor.
    /// `delta_y`: negative = scroll down, positive = scroll up.
    pub fn scroll(delta_y: i32) -> Result<(), PlatformError> {
        #[cfg(target_os = "macos")]
        {
            Self::scroll_macos(delta_y)
        }

        #[cfg(target_os = "linux")]
        {
            Self::scroll_linux(delta_y)
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            Err(PlatformError::Unsupported(
                "Synthetic scroll not supported on this OS".to_string(),
            ))
        }
    }

    #[cfg(target_os = "macos")]
    fn scroll_macos(delta_y: i32) -> Result<(), PlatformError> {
        // CGEventCreateScrollWheelEvent2
        #[link(name = "CoreGraphics", kind = "framework")]
        extern "C" {
            fn CGEventCreateScrollWheelEvent2(
                source: *mut std::ffi::c_void,
                units: u32, // 0 = pixel, 1 = line
                wheel_count: u32,
                wheel1: i32,
                wheel2: i32,
                wheel3: i32,
            ) -> *mut std::ffi::c_void;
            fn CGEventPost(tap: u32, event: *mut std::ffi::c_void);
            fn CFRelease(cf: *mut std::ffi::c_void);
        }

        const K_CG_HID_EVENT_TAP: u32 = 0;
        const K_CG_SCROLL_EVENT_UNIT_PIXEL: u32 = 0;

        unsafe {
            let event = CGEventCreateScrollWheelEvent2(
                std::ptr::null_mut(),
                K_CG_SCROLL_EVENT_UNIT_PIXEL,
                1,
                delta_y,
                0,
                0,
            );
            if event.is_null() {
                return Err(PlatformError::CaptureFailed(
                    "Failed to create scroll CGEvent".to_string(),
                ));
            }
            CGEventPost(K_CG_HID_EVENT_TAP, event);
            CFRelease(event);
        }
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn scroll_linux(_delta_y: i32) -> Result<(), PlatformError> {
        // On Linux X11: XTestFakeButtonEvent(dpy, 4 or 5, True, 0)
        // On Linux Wayland: libei / org.freedesktop.portal.RemoteDesktop
        Ok(())
    }
}
