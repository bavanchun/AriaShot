//! Unit tests for `ImageSession` — pure Rust, no Slint dependency.

use camerashot_core::annotation::{Annotation, AnnotationTool};
use camerashot_core::geometry::Point;
use camerashot_editor::app::image_session::ImageSession;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Helper: create a tiny 2×2 white RGBA Pixmap session.
fn tiny_session() -> ImageSession {
    let w = 2u32;
    let h = 2u32;
    let rgba = vec![255u8; (w * h * 4) as usize]; // white
    ImageSession::from_rgba(w, h, rgba).expect("failed to create tiny session")
}

/// Helper: create a dummy pixelate annotation.
fn dummy_annotation(x: f64, y: f64, w: f64, h: f64) -> Annotation {
    Annotation::new(
        AnnotationTool::Pixelate,
        Point::new(x, y),
        Point::new(x + w, y + h),
        [0, 0, 0, 255],
        4.0,
    )
}

#[test]
fn test_composite_dimensions_match_base() {
    let session = tiny_session();
    let composited = session.composite();
    assert_eq!(composited.width(), 2);
    assert_eq!(composited.height(), 2);
}

#[test]
fn test_composite_without_annotations_equals_base() {
    let session = tiny_session();
    let composited = session.composite();
    assert_eq!(composited.data(), session.base_rgba());
}

#[test]
fn test_apply_redaction_batch_then_undo_redo() {
    let mut session = tiny_session();
    let group = Uuid::new_v4();

    let ann1 = dummy_annotation(0.0, 0.0, 1.0, 1.0);
    let ann2 = dummy_annotation(1.0, 0.0, 1.0, 1.0);
    let id1 = ann1.id;
    let id2 = ann2.id;

    // Apply batch of 2 annotations
    let count = session.apply_redaction_batch(group, vec![ann1, ann2]);
    assert_eq!(count, 2);
    assert_eq!(session.annotations.len(), 2);

    // Undo removes all annotations in one step
    assert!(session.undo());
    assert_eq!(session.annotations.len(), 0);

    // Redo restores all annotations
    assert!(session.redo());
    assert_eq!(session.annotations.len(), 2);

    // Check IDs preserved
    let ids: Vec<Uuid> = session.annotations.iter().map(|a| a.id).collect();
    assert!(ids.contains(&id1));
    assert!(ids.contains(&id2));
}

#[test]
fn test_apply_empty_batch_returns_zero() {
    let mut session = tiny_session();
    let count = session.apply_redaction_batch(Uuid::new_v4(), vec![]);
    assert_eq!(count, 0);
    assert_eq!(session.annotations.len(), 0);
    // Nothing to undo
    assert!(!session.undo());
}

#[test]
fn test_undo_on_empty_returns_false() {
    let mut session = tiny_session();
    assert!(!session.undo());
    assert!(!session.redo());
}

#[test]
fn test_default_save_path_with_source() {
    let mut session = tiny_session();
    session.source_path = Some(PathBuf::from("/tmp/photos/screenshot.png"));

    let path = session.default_save_path();
    assert_eq!(path, PathBuf::from("/tmp/photos/screenshot-edited.png"));
}

#[test]
fn test_default_save_path_without_source() {
    let session = tiny_session();
    assert!(session.source_path.is_none());

    let path = session.default_save_path();
    let path_str = path.to_string_lossy();

    // Should be in ~/Pictures/AriaShot/ariashot-YYYYMMDD-HHMMSS.png
    assert!(
        path_str.contains("Pictures/AriaShot/ariashot-"),
        "path = {path_str}"
    );
    assert!(path_str.ends_with(".png"), "path = {path_str}");
}

#[test]
fn test_save_png_roundtrip() {
    let session = tiny_session();
    let dir = std::env::temp_dir().join("camerashot-test-save");
    let _ = std::fs::remove_dir_all(&dir);
    let out_path = dir.join("test-output.png");

    let result = session.save_png(&out_path);
    assert!(result.is_ok(), "save_png failed: {:?}", result.err());

    let saved = result.unwrap();
    assert_eq!(saved, out_path);
    assert!(out_path.exists());

    // Verify the saved PNG can be loaded back and has correct dimensions
    let loaded = image::open(&out_path).expect("Failed to load saved PNG");
    assert_eq!(loaded.width(), 2);
    assert_eq!(loaded.height(), 2);

    // Cleanup
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_from_file_with_fixture() {
    // Use the OCR test fixture if it exists
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../camerashot-ocr/tests/fixtures/pii-sample.png");

    if fixture.exists() {
        let session = ImageSession::from_file(&fixture);
        assert!(
            session.is_ok(),
            "Failed to load fixture: {:?}",
            session.err()
        );
        let session = session.unwrap();
        assert!(session.width() > 0);
        assert!(session.height() > 0);
        assert!(session.source_path.is_some());
    }
}

#[test]
fn test_multiple_undo_redo_cycles() {
    let mut session = tiny_session();

    // Apply 3 separate batches
    for i in 0..3 {
        let ann = dummy_annotation(i as f64, 0.0, 1.0, 1.0);
        session.apply_redaction_batch(Uuid::new_v4(), vec![ann]);
    }
    assert_eq!(session.annotations.len(), 3);

    // Undo all 3
    assert!(session.undo());
    assert_eq!(session.annotations.len(), 2);
    assert!(session.undo());
    assert_eq!(session.annotations.len(), 1);
    assert!(session.undo());
    assert_eq!(session.annotations.len(), 0);
    assert!(!session.undo()); // nothing left

    // Redo all 3
    assert!(session.redo());
    assert_eq!(session.annotations.len(), 1);
    assert!(session.redo());
    assert_eq!(session.annotations.len(), 2);
    assert!(session.redo());
    assert_eq!(session.annotations.len(), 3);
    assert!(!session.redo()); // nothing left
}
