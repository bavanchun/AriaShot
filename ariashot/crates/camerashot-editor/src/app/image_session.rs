//! Pure-Rust image editing session — no Slint dependency, fully testable.
//!
//! Owns the base pixmap, annotation layer, and undo stack.
//! Compositing renders annotations onto a clone of the base image.

use camerashot_core::annotation::Annotation;
use camerashot_core::undo::{UndoAction, UndoStack};
use camerashot_core::AnnotationRenderer;
use chrono::Local;
use std::path::{Path, PathBuf};
use tiny_skia::Pixmap;
use uuid::Uuid;

/// Holds the state for a single image-editing session.
pub struct ImageSession {
    /// Original, unmodified base image.
    base: Pixmap,
    /// Current vector annotations on top of the base image.
    pub annotations: Vec<Annotation>,
    /// Undo/redo history.
    pub undo: UndoStack,
    /// The file path the image was loaded from, if any.
    /// `None` means the image was captured from screen.
    pub source_path: Option<PathBuf>,
}

impl ImageSession {
    /// Create a session from a loaded image file.
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let img = image::open(path).map_err(|e| format!("Failed to open image: {e}"))?;
        let rgba = img.to_rgba8();
        let (w, h) = (rgba.width(), rgba.height());
        let pixmap = Pixmap::from_vec(
            rgba.into_raw(),
            tiny_skia::IntSize::from_wh(w, h).ok_or("Invalid image dimensions")?,
        )
        .ok_or("Failed to create Pixmap from image data")?;

        Ok(Self {
            base: pixmap,
            annotations: Vec::new(),
            undo: UndoStack::default(),
            source_path: Some(path.to_path_buf()),
        })
    }

    /// Create a session from raw RGBA pixel data (e.g. screen capture).
    pub fn from_rgba(width: u32, height: u32, rgba_data: Vec<u8>) -> Result<Self, String> {
        let pixmap = Pixmap::from_vec(
            rgba_data,
            tiny_skia::IntSize::from_wh(width, height).ok_or("Invalid capture dimensions")?,
        )
        .ok_or("Failed to create Pixmap from capture data")?;

        Ok(Self {
            base: pixmap,
            annotations: Vec::new(),
            undo: UndoStack::default(),
            source_path: None,
        })
    }

    /// Width of the base image in pixels.
    #[inline]
    pub fn width(&self) -> u32 {
        self.base.width()
    }

    /// Height of the base image in pixels.
    #[inline]
    pub fn height(&self) -> u32 {
        self.base.height()
    }

    /// Composite the base image with all current annotations.
    /// Returns a new `Pixmap` — the base is never mutated.
    pub fn composite(&self) -> Pixmap {
        let mut out = self.base.clone();
        if !self.annotations.is_empty() {
            let mut pm = out.as_mut();
            AnnotationRenderer::render_annotations(&mut pm, &self.annotations);
        }
        out
    }

    /// Raw RGBA bytes of the base image (useful for OCR which operates on raw data).
    pub fn base_rgba(&self) -> &[u8] {
        self.base.data()
    }

    /// Apply a batch of redaction annotations with atomic undo.
    ///
    /// Returns the number of annotations added.
    pub fn apply_redaction_batch(&mut self, group_id: Uuid, anns: Vec<Annotation>) -> usize {
        let count = anns.len();
        if count == 0 {
            return 0;
        }

        // Build individual Add actions for the batch
        let mut batch_actions = Vec::with_capacity(count);
        for ann in &anns {
            batch_actions.push(UndoAction::Add {
                annotation: ann.clone(),
            });
        }

        // Push the batch action onto undo stack
        self.undo.push(UndoAction::Batch {
            group_id: Some(group_id),
            actions: batch_actions,
        });

        // Add annotations to the live list
        self.annotations.extend(anns);

        count
    }

    /// Undo the last action. Returns `true` if something was undone.
    pub fn undo(&mut self) -> bool {
        self.undo.undo(&mut self.annotations)
    }

    /// Redo the last undone action. Returns `true` if something was redone.
    pub fn redo(&mut self) -> bool {
        self.undo.redo(&mut self.annotations)
    }

    /// Compute the default save path based on the source.
    ///
    /// - If loaded from a file: `<stem>-edited.png` in the same directory.
    /// - If captured from screen: `~/Pictures/AriaShot/ariashot-YYYYMMDD-HHMMSS.png`.
    pub fn default_save_path(&self) -> PathBuf {
        if let Some(ref src) = self.source_path {
            let stem = src.file_stem().unwrap_or_default().to_string_lossy();
            let parent = src.parent().unwrap_or_else(|| Path::new("."));
            parent.join(format!("{stem}-edited.png"))
        } else {
            let now = Local::now();
            let ts = now.format("%Y%m%d-%H%M%S");
            let pictures = dirs_picture_ariashot();
            pictures.join(format!("ariashot-{ts}.png"))
        }
    }

    /// Save the composited image to the given path as PNG.
    pub fn save_png(&self, path: &Path) -> Result<PathBuf, String> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory {}: {e}", parent.display()))?;
        }

        let composited = self.composite();
        composited
            .save_png(path)
            .map_err(|e| format!("Failed to save PNG: {e}"))?;

        Ok(path.to_path_buf())
    }
}

/// Returns `~/Pictures/AriaShot/`, creating it if necessary.
fn dirs_picture_ariashot() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let dir = PathBuf::from(home).join("Pictures").join("AriaShot");
    let _ = std::fs::create_dir_all(&dir);
    dir
}
