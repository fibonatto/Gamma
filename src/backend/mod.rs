use std::{ffi::{CStr, CString}, path::Path, sync::{Mutex, OnceLock}};

unsafe extern "C" {
    fn gamma_inspect_json(path: *const std::ffi::c_char, modules: *const std::ffi::c_char, out: *mut *mut std::ffi::c_char) -> i32;
    fn gamma_free(ptr: *mut std::ffi::c_char);
}

static LEAN_GATE: OnceLock<Mutex<()>> = OnceLock::new();
pub struct Backend;

impl Backend {
    pub fn initialize() -> Result<Self, String> { Ok(Self) }

    pub fn inspect(&self, path: &Path, modules: &str) -> Result<String, String> {
        let _guard = LEAN_GATE.get_or_init(|| Mutex::new(())).lock().map_err(|_| "Lean runtime lock poisoned".to_owned())?;
        let path = CString::new(path.to_string_lossy().as_bytes()).map_err(|_| "path contains NUL".to_owned())?;
        let modules = CString::new(modules).map_err(|_| "module list contains NUL".to_owned())?;
        let mut out = std::ptr::null_mut();
        let status = unsafe { gamma_inspect_json(path.as_ptr(), modules.as_ptr(), &mut out) };
        if status != 0 { return Err(format!("Lean FFI failed (status {status})")); }
        if out.is_null() { return Err("Lean FFI returned a null result".into()); }
        let json = unsafe { CStr::from_ptr(out).to_string_lossy().into_owned() };
        unsafe { gamma_free(out) };
        Ok(json)
    }
}
