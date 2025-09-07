use crate::level::Level;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, prelude::*};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;
#[cfg(target_arch = "wasm32")]
use web_sys::{
    FileSystemDirectoryHandle, FileSystemFileHandle, FileSystemGetFileOptions,
    FileSystemWritableFileStream, IdbDatabase, IdbTransactionMode, StorageManager,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorMode {
    FirstPerson,
    /// Orthographic top-down style editing.
    Orthographic,
    PrefabPalette,
    CsgBrush,
    SplineTool,
    Volume,
    NavMesh,
    Validation,
}

pub struct EditorClient {
    pub mode: EditorMode,
    /// Whether transform gizmos are visible.
    pub gizmos: bool,
    /// Optional grid snapping increment.
    pub grid_snap: Option<f32>,
    /// Control points for the in-progress spline tool.
    pub spline: Vec<[f32; 3]>,
    /// Points defining editor volumes.
    pub volumes: Vec<[f32; 3]>,
    /// Whether the baked navigation mesh is visible.
    pub navmesh_visible: bool,
    /// History stack for undo operations.
    pub undo_stack: Vec<Level>,
    /// History stack for redo operations.
    pub redo_stack: Vec<Level>,
}

impl EditorClient {
    /// Create a new editor client with default settings.
    pub fn new() -> Self {
        Self {
            mode: EditorMode::FirstPerson,
            gizmos: true,
            grid_snap: None,
            spline: Vec::new(),
            volumes: Vec::new(),
            navmesh_visible: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn set_mode(&mut self, mode: EditorMode) {
        self.mode = mode;
    }

    /// Toggle the visibility of editing gizmos.
    pub fn toggle_gizmos(&mut self) {
        self.gizmos = !self.gizmos;
    }

    /// Enable grid snapping with the provided step size or disable it with `None`.
    pub fn set_grid_snap(&mut self, step: Option<f32>) {
        self.grid_snap = step;
    }

    /// Apply grid snapping to the provided coordinate if enabled.
    pub fn snap_value(&self, v: f32) -> f32 {
        if let Some(step) = self.grid_snap {
            (v / step).round() * step
        } else {
            v
        }
    }

    /// Append a control point to the active spline tool.
    pub fn add_spline_point(&mut self, point: [f32; 3]) {
        self.spline.push(point);
    }

    /// Clear any active spline being edited.
    pub fn clear_spline(&mut self) {
        self.spline.clear();
    }

    /// Add a point to the active volume being edited.
    pub fn add_volume_point(&mut self, point: [f32; 3]) {
        self.volumes.push(point);
    }

    /// Clear any temporary volume definition.
    pub fn clear_volumes(&mut self) {
        self.volumes.clear();
    }

    /// Toggle visibility of the navigation mesh overlay.
    pub fn toggle_navmesh(&mut self) {
        self.navmesh_visible = !self.navmesh_visible;
    }

    /// Capture the current level state for undo operations.
    pub fn snapshot(&mut self, level: &Level) {
        self.undo_stack.push(level.clone());
        self.redo_stack.clear();
    }

    /// Undo to the previous captured level state.
    pub fn undo(&mut self, level: &mut Level) -> bool {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(level.clone());
            *level = prev;
            true
        } else {
            false
        }
    }

    /// Redo the last undone level state.
    pub fn redo(&mut self, level: &mut Level) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(level.clone());
            *level = next;
            true
        } else {
            false
        }
    }

    /// Persist the level locally using OPFS or IndexedDB.
    pub async fn store_level_locally(&self, level: &Level) -> Result<(), String> {
        #[cfg(target_arch = "wasm32")]
        {
            let data = serde_json::to_string(level).map_err(|e| e.to_string())?;
            save_level(&level.id, &data)
                .await
                .map_err(|e| format!("{e:?}"))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = level;
            Ok(())
        }
    }

    /// Load a previously stored level.
    pub async fn load_level_locally(&self, id: &str) -> Result<Option<Level>, String> {
        #[cfg(target_arch = "wasm32")]
        {
            match load_level(id).await {
                Ok(Some(data)) => serde_json::from_str(&data)
                    .map(Some)
                    .map_err(|e| e.to_string()),
                Ok(None) => Ok(None),
                Err(e) => Err(format!("{e:?}")),
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = id;
            Ok(None)
        }
    }
}

// --- wasm helpers ---

#[cfg(target_arch = "wasm32")]
const DB_NAME: &str = "editor-levels";
#[cfg(target_arch = "wasm32")]
const STORE_NAME: &str = "levels";
#[cfg(target_arch = "wasm32")]
const DB_VERSION: u32 = 1;

#[cfg(target_arch = "wasm32")]
async fn save_level(id: &str, data: &str) -> Result<(), JsValue> {
    if save_opfs(id, data).await.is_err() {
        save_idb(id, data).await?
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
async fn load_level(id: &str) -> Result<Option<String>, JsValue> {
    if let Ok(Some(data)) = load_opfs(id).await {
        return Ok(Some(data));
    }
    load_idb(id).await
}

#[cfg(target_arch = "wasm32")]
async fn save_opfs(id: &str, data: &str) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let navigator = window.navigator();
    let storage: StorageManager = navigator
        .storage()
        .ok_or_else(|| JsValue::from_str("no storage"))?;
    let dir_js = JsFuture::from(storage.get_directory()).await?;
    let dir: FileSystemDirectoryHandle = dir_js.dyn_into()?;
    let mut opts = FileSystemGetFileOptions::new();
    opts.create(true);
    let file_js = JsFuture::from(dir.get_file_handle_with_options(id, &opts)).await?;
    let file: FileSystemFileHandle = file_js.dyn_into()?;
    let writable_js = JsFuture::from(file.create_writable()).await?;
    let writable: FileSystemWritableFileStream = writable_js.dyn_into()?;
    JsFuture::from(writable.write_with_str(data)).await?;
    JsFuture::from(writable.close()).await?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
async fn load_opfs(id: &str) -> Result<Option<String>, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let navigator = window.navigator();
    let storage: StorageManager = navigator
        .storage()
        .ok_or_else(|| JsValue::from_str("no storage"))?;
    let dir_js = JsFuture::from(storage.get_directory()).await?;
    let dir: FileSystemDirectoryHandle = dir_js.dyn_into()?;
    let file_js = JsFuture::from(dir.get_file_handle(id)).await;
    let file_handle = match file_js {
        Ok(v) => v.dyn_into::<FileSystemFileHandle>()?,
        Err(_) => return Ok(None),
    };
    let file_js = JsFuture::from(file_handle.get_file()).await?;
    let file: web_sys::File = file_js.dyn_into()?;
    let text_js = JsFuture::from(file.text()).await?;
    Ok(text_js.as_string())
}

#[cfg(target_arch = "wasm32")]
async fn open_db() -> Result<IdbDatabase, JsValue> {
    use std::{cell::RefCell, rc::Rc};

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let factory = window
        .indexed_db()?
        .ok_or_else(|| JsValue::from_str("no indexeddb"))?;
    let open = factory.open_with_u32(DB_NAME, DB_VERSION)?;

    let error: Rc<RefCell<Option<JsValue>>> = Rc::new(RefCell::new(None));
    let error_clone = error.clone();
    let open_clone = open.clone();
    let on_upgrade = Closure::once_into_js(move |_evt: web_sys::IdbVersionChangeEvent| {
        let res = (|| -> Result<(), JsValue> {
            let db_js = open_clone.result()?;
            let db: IdbDatabase = db_js.dyn_into()?;
            db.create_object_store(STORE_NAME)?;
            Ok(())
        })();
        if let Err(e) = res {
            *error_clone.borrow_mut() = Some(e);
        }
    });
    open.set_onupgradeneeded(Some(on_upgrade.as_ref().unchecked_ref()));
    on_upgrade.forget();
    let db_js = JsFuture::from(open).await?;
    if let Some(err) = error.borrow_mut().take() {
        Err(err)
    } else {
        db_js.dyn_into()
    }
}

#[cfg(target_arch = "wasm32")]
async fn save_idb(id: &str, data: &str) -> Result<(), JsValue> {
    let db = open_db().await?;
    let tx = db.transaction_with_str_and_mode(STORE_NAME, IdbTransactionMode::Readwrite)?;
    let store = tx.object_store(STORE_NAME)?;
    let req = store.put_with_key(&JsValue::from_str(data), &JsValue::from_str(id))?;
    JsFuture::from(req).await?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
async fn load_idb(id: &str) -> Result<Option<String>, JsValue> {
    let db = open_db().await?;
    let tx = db.transaction_with_str_and_mode(STORE_NAME, IdbTransactionMode::Readonly)?;
    let store = tx.object_store(STORE_NAME)?;
    let req = store.get(&JsValue::from_str(id))?;
    let val = JsFuture::from(req).await?;
    if val.is_undefined() {
        Ok(None)
    } else {
        Ok(val.as_string())
    }
}
