use camerashot_core::annotation::AnnotationTool;
use camerashot_core::geometry::Rect;
use camerashot_core::undo::{UndoAction, UndoStack};
use camerashot_ocr::{PiiCategory, PiiDetector, PiiRedactor};

#[test]
fn test_pii_detector_11_patterns() {
    let detector = PiiDetector::new();

    // 1. Email
    let email_matches = detector.scan_text("Contact me at user.name+dev@example.co.uk please");
    assert!(email_matches
        .iter()
        .any(|m| m.category == PiiCategory::Email));

    // 2. Phone
    let phone_matches = detector.scan_text("Call +1 415-555-2671 or 555-1234");
    assert!(phone_matches
        .iter()
        .any(|m| m.category == PiiCategory::Phone));

    // 3. SSN
    let ssn_matches = detector.scan_text("SSN: 123-45-6789 confidential");
    assert!(ssn_matches.iter().any(|m| m.category == PiiCategory::Ssn));

    // 4. Credit Card
    let cc_matches = detector.scan_text("Card: 4532 1123 4567 8901 visa");
    assert!(cc_matches
        .iter()
        .any(|m| m.category == PiiCategory::CreditCard));

    // 5. CVV
    let cvv_matches = detector.scan_text("Security code CVV: 789 on back");
    assert!(cvv_matches.iter().any(|m| m.category == PiiCategory::Cvv));

    // 6. Expiry
    let exp_matches = detector.scan_text("Valid thru 12/28 or 2026-09");
    assert!(exp_matches
        .iter()
        .any(|m| m.category == PiiCategory::Expiry));

    // 7. IPv4
    let ip_matches = detector.scan_text("Server listening on 192.168.1.105:8080");
    assert!(ip_matches.iter().any(|m| m.category == PiiCategory::Ipv4));

    // 8. AWS Key
    let aws_matches = detector.scan_text("export AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE");
    assert!(aws_matches
        .iter()
        .any(|m| m.category == PiiCategory::AwsKey));

    // 9. Secret Assignment
    let secret_matches = detector.scan_text("api_key: secret_token_99182a17z");
    assert!(secret_matches
        .iter()
        .any(|m| m.category == PiiCategory::SecretAssignment));

    // 10. Hex Key
    let hex_matches = detector
        .scan_text("Hash: 5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8");
    assert!(hex_matches
        .iter()
        .any(|m| m.category == PiiCategory::HexKey));

    // 11. Bearer Token
    let bearer_matches =
        detector.scan_text("Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.xyz");
    assert!(bearer_matches
        .iter()
        .any(|m| m.category == PiiCategory::BearerToken));
}

#[test]
fn test_pii_redactor_batch_undo_integration() {
    let redactor = PiiRedactor::new();
    let text = "Contact support@example.com or call 123-45-6789";
    let block_rect = Rect::new(100.0, 200.0, 400.0, 30.0);

    let (batch_id, annotations) =
        redactor.build_redactions_for_blocks(&[(text, block_rect)], AnnotationTool::Pixelate, 2.0);

    assert_eq!(annotations.len(), 2); // 1 email + 1 ssn
    for ann in &annotations {
        assert_eq!(ann.group_id, Some(batch_id));
        assert_eq!(ann.tool, AnnotationTool::Pixelate);
    }

    // Verify integration with UndoStack
    let mut undo_stack = UndoStack::new(50);
    let mut canvas_annotations = Vec::new();

    // Push batch action
    let add_actions: Vec<UndoAction> = annotations
        .iter()
        .map(|ann| UndoAction::Add {
            annotation: ann.clone(),
        })
        .collect();
    undo_stack.push(UndoAction::Batch {
        group_id: Some(batch_id),
        actions: add_actions,
    });
    canvas_annotations.extend(annotations);

    assert_eq!(canvas_annotations.len(), 2);
    assert!(undo_stack.can_undo());

    // 1-click Ctrl+Z rollback
    let success = undo_stack.undo(&mut canvas_annotations);
    assert!(success);

    // All redaction blocks removed atomically in one undo!
    assert!(canvas_annotations.is_empty());

    // 1-click Ctrl+Shift+Z redo restoration
    assert!(undo_stack.can_redo());
    let redo_success = undo_stack.redo(&mut canvas_annotations);
    assert!(redo_success);
    assert_eq!(canvas_annotations.len(), 2);
}
