use serde::{Deserialize, Serialize};

/// Input from a client for a single simulation frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputFrame {
    /// Frame number this input applies to.
    pub frame: u32,
    /// Opaque input payload.
    pub data: Vec<u8>,
}

/// Full state snapshot from the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    /// Frame number the snapshot represents.
    pub frame: u32,
    /// Raw snapshot payload.
    pub data: Vec<u8>,
}

/// Delta between two [`Snapshot`]s produced by [`delta_compress`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotDelta {
    /// Frame number of the produced snapshot.
    pub frame: u32,
    /// XOR diff of the snapshot bytes.
    pub delta: Vec<u8>,
}

/// Create a [`SnapshotDelta`] by XOR'ing the bytes of `base` and `current`.
pub fn delta_compress(base: &Snapshot, current: &Snapshot) -> SnapshotDelta {
    let delta = base
        .data
        .iter()
        .zip(&current.data)
        .map(|(a, b)| a ^ b)
        .collect();
    SnapshotDelta {
        frame: current.frame,
        delta,
    }
}

/// Apply a [`SnapshotDelta`] to `base` to reconstruct a [`Snapshot`].
pub fn apply_delta(base: &Snapshot, delta: &SnapshotDelta) -> Snapshot {
    let data = base
        .data
        .iter()
        .zip(&delta.delta)
        .map(|(a, d)| a ^ d)
        .collect();
    Snapshot {
        frame: delta.frame,
        data,
    }
}
