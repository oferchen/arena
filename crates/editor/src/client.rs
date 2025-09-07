use crate::level::Level;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorMode {
    FirstPerson,
    TopDown,
    PrefabPalette,
    CsgBrush,
    SplineVolume,
}

pub struct EditorClient {
    pub mode: EditorMode,
}

impl EditorClient {
    pub fn new() -> Self {
        Self { mode: EditorMode::FirstPerson }
    }

    pub fn set_mode(&mut self, mode: EditorMode) {
        self.mode = mode;
    }

    /// Store level data using the browser's OPFS/IndexedDB.
    ///
    /// This is a placeholder implementation. The actual
    /// Web APIs would be invoked from WASM.
    #[allow(unused_variables)]
    pub fn store_level_locally(&self, level: &Level) {
        // TODO: implement OPFS/IndexedDB persistence
    }
}
