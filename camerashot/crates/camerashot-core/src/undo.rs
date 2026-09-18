use crate::annotation::Annotation;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Atomic action stored in the undo/redo history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UndoAction {
    /// An annotation was added to the canvas.
    Add { annotation: Annotation },
    /// An annotation was deleted from the canvas at the given index.
    Delete {
        annotation: Annotation,
        index: usize,
    },
    /// An existing annotation's properties or points were modified.
    Modify {
        previous: Annotation,
        current: Annotation,
    },
    /// A composite batch of actions (e.g. auto-redact batch or multi-select deletion).
    Batch {
        group_id: Option<Uuid>,
        actions: Vec<UndoAction>,
    },
}

/// Event-sourced, crash-safe undo and redo stack for annotations.
/// Replaces AppKit's fragile ScopedUndoTextView architecture with deterministic state changes.
#[derive(Debug, Clone)]
pub struct UndoStack {
    undo_stack: Vec<UndoAction>,
    redo_stack: Vec<UndoAction>,
    max_history: usize,
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new(100)
    }
}

impl UndoStack {
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history: max_history.max(10),
        }
    }

    #[inline]
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    #[inline]
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Push a newly performed action onto the undo stack, clearing any existing redo history.
    pub fn push(&mut self, action: UndoAction) {
        self.redo_stack.clear();
        if self.undo_stack.len() >= self.max_history {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(action);
    }

    /// Undo the most recent action and apply changes to the annotations slice.
    /// Returns true if an action was undone, false if the stack was empty.
    pub fn undo(&mut self, annotations: &mut Vec<Annotation>) -> bool {
        if let Some(action) = self.undo_stack.pop() {
            Self::apply_undo(&action, annotations);
            self.redo_stack.push(action);
            true
        } else {
            false
        }
    }

    /// Redo the most recently undone action and re-apply changes to the annotations slice.
    /// Returns true if an action was redone, false if the redo stack was empty.
    pub fn redo(&mut self, annotations: &mut Vec<Annotation>) -> bool {
        if let Some(action) = self.redo_stack.pop() {
            Self::apply_redo(&action, annotations);
            self.undo_stack.push(action);
            true
        } else {
            false
        }
    }

    fn apply_undo(action: &UndoAction, annotations: &mut Vec<Annotation>) {
        match action {
            UndoAction::Add { annotation } => {
                annotations.retain(|a| a.id != annotation.id);
            }
            UndoAction::Delete { annotation, index } => {
                let safe_idx = (*index).min(annotations.len());
                annotations.insert(safe_idx, annotation.clone());
            }
            UndoAction::Modify { previous, .. } => {
                if let Some(slot) = annotations.iter_mut().find(|a| a.id == previous.id) {
                    *slot = previous.clone();
                }
            }
            UndoAction::Batch { actions, .. } => {
                // Batch undo must execute sub-actions in reverse order
                for sub_action in actions.iter().rev() {
                    Self::apply_undo(sub_action, annotations);
                }
            }
        }
    }

    fn apply_redo(action: &UndoAction, annotations: &mut Vec<Annotation>) {
        match action {
            UndoAction::Add { annotation } => {
                if !annotations.iter().any(|a| a.id == annotation.id) {
                    annotations.push(annotation.clone());
                }
            }
            UndoAction::Delete { annotation, .. } => {
                annotations.retain(|a| a.id != annotation.id);
            }
            UndoAction::Modify { current, .. } => {
                if let Some(slot) = annotations.iter_mut().find(|a| a.id == current.id) {
                    *slot = current.clone();
                }
            }
            UndoAction::Batch { actions, .. } => {
                // Batch redo must execute sub-actions in original order
                for sub_action in actions.iter() {
                    Self::apply_redo(sub_action, annotations);
                }
            }
        }
    }
}
