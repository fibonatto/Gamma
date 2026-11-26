use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
#[derive(Serialize, Deserialize, Default)]
struct DiskCache { version: u32, entries: std::collections::HashMap<String, String> }
pub struct Cache { path: std::path::PathBuf, data: DiskCache }
impl Cache {
    pub fn open(root: &Path) -> Self {
        let dir = root.join(".gamma"); let path = dir.join("cache.json");
        let data = fs::read(&path).ok().and_then(|b| serde_json::from_slice::<DiskCache>(&b).ok()).filter(|c| c.version == 2).unwrap_or(DiskCache { version: 2, entries: Default::default() });
        Self { path, data }
    }
    pub fn get(&self, key: &str) -> Option<String> { self.data.entries.get(key).cloned() }
    pub fn put(&mut self, key: String, json: &str) -> Result<(), String> {
        self.data.entries.insert(key, json.to_owned());
        let parent = self.path.parent().unwrap(); fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let tmp = self.path.with_extension("json.tmp");
        let bytes = serde_json::to_vec(&self.data).map_err(|e| e.to_string())?;
        fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
        fs::rename(tmp, &self.path).map_err(|e| e.to_string())?;
        Ok(())
    }
}
