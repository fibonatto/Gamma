mod imports;
pub use imports::parse_imports;
use std::{env, fs, path::{Path, PathBuf}};

pub struct Project { pub root: PathBuf, pub target: PathBuf, pub search_paths: Vec<PathBuf>, pub source_roots: Vec<PathBuf> }
impl Project {
    pub fn discover(target: &Path) -> Result<Self, String> {
        let mut cur = target.parent().ok_or("target has no parent directory")?.to_path_buf();
        let root = loop {
            if cur.join("lakefile.lean").exists() || cur.join("lakefile.toml").exists() { break cur; }
            if !cur.pop() { break target.parent().unwrap().to_path_buf(); }
        };
        let mut search_paths = vec![root.join(".lake/build/lib/lean"), root.join(".lake/build/lib")];
        let mut source_roots = vec![root.clone()];
        for candidate in [root.join("src"), root.join("lean")] { if candidate.is_dir() { source_roots.push(candidate); } }
        if let Some(existing) = env::var_os("LEAN_PATH") {
            search_paths.extend(env::split_paths(&existing));
        }
        if let Some(sysroot) = option_env!("GAMMA_SYSROOT") {
            search_paths.push(PathBuf::from(sysroot).join("lib/lean"));
        }
        let packages = root.join(".lake/packages");
        if let Ok(entries) = fs::read_dir(packages) {
            for entry in entries.flatten() {
                let package_root = entry.path();
                search_paths.push(package_root.join(".lake/build/lib/lean"));
                source_roots.push(package_root);
            }
        }
        search_paths.retain(|p| p.is_dir());
        search_paths.sort(); search_paths.dedup();
        source_roots.retain(|p| p.is_dir());
        source_roots.sort(); source_roots.dedup();
        Ok(Self { root, target: target.to_path_buf(), search_paths, source_roots })
    }
    pub fn configure_lean_path(&self) {
        let joined = env::join_paths(&self.search_paths).expect("invalid LEAN_PATH entries");
        env::set_var("LEAN_PATH", joined);
    }
    pub fn resolve_source(&self, module: &str) -> Option<PathBuf> {
        let rel = format!("{}.lean", module.replace('.', "/"));
        for base in &self.source_roots {
            let path = base.join(&rel);
            if path.is_file() { return Some(path); }
        }
        None
    }
}
