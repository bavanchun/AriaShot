/// Fast 64-bit non-cryptographic content hash for settlement detection.
#[inline]
pub fn compute_frame_hash(data: &[u8]) -> u64 {
    // 64-bit FNV-1a hash
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET_BASIS;
    // Stride by 8 for high throughput
    let chunks = data.chunks_exact(8);
    let remainder = chunks.remainder();

    for chunk in chunks {
        let val = u64::from_le_bytes(chunk.try_into().unwrap());
        hash ^= val;
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    for &byte in remainder {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    hash
}

/// Settlement detector that determines whether scrolling animation has stopped
/// by verifying consecutive identical content hashes.
#[derive(Debug, Default, Clone)]
pub struct SettlementDetector {
    last_hash: Option<u64>,
    consecutive_matches: usize,
    required_matches: usize,
}

impl SettlementDetector {
    /// Create detector where `required_identical_frames` is the total number of identical
    /// frames needed to declare settlement (e.g. 2 for two consecutive identical frames).
    pub fn new(required_identical_frames: usize) -> Self {
        Self {
            last_hash: None,
            consecutive_matches: 0,
            required_matches: required_identical_frames.saturating_sub(1).max(1),
        }
    }

    pub fn reset(&mut self) {
        self.last_hash = None;
        self.consecutive_matches = 0;
    }

    /// Feed a new frame buffer. Returns true if the frame is settled (identical content).
    pub fn update(&mut self, frame_data: &[u8]) -> bool {
        let current_hash = compute_frame_hash(frame_data);
        if let Some(prev) = self.last_hash {
            if prev == current_hash {
                self.consecutive_matches += 1;
                if self.consecutive_matches >= self.required_matches {
                    return true;
                }
            } else {
                self.consecutive_matches = 0;
            }
        }
        self.last_hash = Some(current_hash);
        false
    }
}
