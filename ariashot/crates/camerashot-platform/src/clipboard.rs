//! Clipboard operations for copying images and text.
//!
//! Uses `arboard` for cross-platform clipboard access.
//!
//! **Linux note**: `copy_rgba_image` calls `arboard::SetExtLinux::wait()`
//! which blocks the current thread until another application reads the clipboard.

use crate::traits::PlatformError;
use arboard::{Clipboard, ImageData};
use std::borrow::Cow;

/// Copy an RGBA image to the system clipboard.
///
/// `rgba` must be a tightly-packed RGBA8 buffer of length `width * height * 4`.
///
/// On Linux this function blocks until another application reads from the clipboard,
/// because Linux clipboard ownership is cooperative.
pub fn copy_rgba_image(width: u32, height: u32, rgba: &[u8]) -> Result<(), PlatformError> {
    let mut clipboard = Clipboard::new()
        .map_err(|e| PlatformError::CaptureFailed(format!("clipboard: {e}")))?;

    let img = ImageData {
        width: width as usize,
        height: height as usize,
        bytes: Cow::Borrowed(rgba),
    };

    clipboard
        .set_image(img)
        .map_err(|e| PlatformError::CaptureFailed(format!("clipboard set_image: {e}")))?;

    Ok(())
}

/// Copy a text string to the system clipboard.
pub fn copy_text(text: &str) -> Result<(), PlatformError> {
    let mut clipboard = Clipboard::new()
        .map_err(|e| PlatformError::CaptureFailed(format!("clipboard: {e}")))?;

    clipboard
        .set_text(text)
        .map_err(|e| PlatformError::CaptureFailed(format!("clipboard set_text: {e}")))?;

    Ok(())
}
