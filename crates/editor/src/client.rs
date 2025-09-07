use crate::level::Level;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = storeLevel)]
    async fn store_level(id: &str, data: &str);
    #[wasm_bindgen(js_name = loadLevel)]
    async fn load_level(id: &str) -> JsValue;
}

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
        Self {
            mode: EditorMode::FirstPerson,
        }
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
        #[cfg(target_arch = "wasm32")]
        {
            let id = level.id.clone();
            let data = serde_json::to_string(level).expect("serialize level");
            spawn_local(async move {
                let _ = store_level(&id, &data).await;
            });
        }
    }

    #[allow(dead_code)]
    #[allow(unused_variables)]
    pub async fn load_level_locally(&self, id: &str) -> Option<Level> {
        #[cfg(target_arch = "wasm32")]
        {
            let data = load_level(id).await;
            if data.is_null() || data.is_undefined() {
                return None;
            }
            let s = data.as_string()?;
            return serde_json::from_str(&s).ok();
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = id;
            None
        }
    }
}
