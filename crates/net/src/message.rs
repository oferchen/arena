use anyhow::{Error, anyhow};
use bevy::prelude::Event;
use serde::{Deserialize, Serialize};

/// Input from a client for a single simulation frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Event)]
pub struct InputFrame {
    /// Frame number this input applies to.
    pub frame: u32,
    /// Opaque input payload.
    pub data: Vec<u8>,
}

/// Full state snapshot from the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Event)]
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

/// Messages from the server describing world state updates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerMessage {
    /// Full baseline snapshot.
    Baseline(Snapshot),
    /// Delta-compressed snapshot relative to the last baseline.
    Delta(SnapshotDelta),
}

/// Create a [`SnapshotDelta`] by XOR'ing the bytes of `base` and `current`.
pub fn delta_compress(base: &Snapshot, current: &Snapshot) -> Result<SnapshotDelta, Error> {
    if base.data.len() != current.data.len() {
        return Err(anyhow!(
            "snapshot length mismatch: {} != {}",
            base.data.len(),
            current.data.len()
        ));
    }
    let delta = base
        .data
        .iter()
        .zip(&current.data)
        .map(|(a, b)| a ^ b)
        .collect();
    Ok(SnapshotDelta {
        frame: current.frame,
        delta,
    })
}

/// Apply a [`SnapshotDelta`] to `base` to reconstruct a [`Snapshot`].
pub fn apply_delta(base: &Snapshot, delta: &SnapshotDelta) -> Result<Snapshot, Error> {
    if base.data.len() != delta.delta.len() {
        return Err(anyhow!(
            "snapshot length mismatch: {} != {}",
            base.data.len(),
            delta.delta.len()
        ));
    }
    let data = base
        .data
        .iter()
        .zip(&delta.delta)
        .map(|(a, d)| a ^ d)
        .collect();
    Ok(Snapshot {
        frame: delta.frame,
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delta_compress_and_apply_delta_happy_path() -> Result<(), Error> {
        let base = Snapshot {
            frame: 1,
            data: vec![1, 2, 3],
        };
        let current = Snapshot {
            frame: 2,
            data: vec![2, 4, 6],
        };
        let delta = delta_compress(&base, &current)?;
        let reconstructed = apply_delta(&base, &delta)?;
        assert_eq!(reconstructed, current);
        Ok(())
    }

    #[test]
    fn delta_compress_mismatched_lengths() {
        let base = Snapshot {
            frame: 1,
            data: vec![1, 2, 3],
        };
        let current = Snapshot {
            frame: 2,
            data: vec![1, 2],
        };
        assert!(delta_compress(&base, &current).is_err());
    }

    #[test]
    fn apply_delta_mismatched_lengths() {
        let base = Snapshot {
            frame: 1,
            data: vec![1, 2, 3],
        };
        let delta = SnapshotDelta {
            frame: 2,
            delta: vec![1, 2],
        };
        assert!(apply_delta(&base, &delta).is_err());
    }
}
