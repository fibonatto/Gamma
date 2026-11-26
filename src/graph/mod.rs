use std::{collections::{BTreeMap, BTreeSet}, fs, path::Path};
use rayon::prelude::*;
use crate::project::{Project, parse_imports};

#[derive(Default)]
pub struct ModuleGraph { deps: BTreeMap<String, Vec<String>>, source: BTreeMap<String, std::path::PathBuf>, target_name: String }
impl ModuleGraph {
    pub fn discover(project: &Project, target: &Path) -> Result<Self, String> {
        let root_name = target.file_stem().and_then(|s| s.to_str()).unwrap_or("Main").to_owned();
        let mut graph = Self { target_name: root_name.clone(), ..Self::default() };
        graph.visit(project, root_name, target.to_path_buf(), &mut BTreeSet::new())?;
        // The target itself is omitted from the imported-module list passed to Lean.
        Ok(graph)
    }
    fn visit(&mut self, project: &Project, name: String, path: std::path::PathBuf, active: &mut BTreeSet<String>) -> Result<(), String> {
        if self.source.contains_key(&name) { return Ok(()); }
        if !active.insert(name.clone()) { return Err(format!("import cycle detected at {name}")); }
        let source = fs::read_to_string(&path).map_err(|e| format!("reading {}: {e}", path.display()))?;
        let mut deps = Vec::new();
        for imp in parse_imports(&source) {
            if let Some(dep_path) = project.resolve_source(&imp) {
                deps.push(imp.clone());
                self.visit(project, imp, dep_path, active)?;
            }
        }
        active.remove(&name);
        deps.sort(); deps.dedup();
        self.deps.insert(name.clone(), deps);
        self.source.insert(name, path);
        Ok(())
    }
    pub fn ordered_modules(&self) -> Vec<String> {
        let mut done = BTreeSet::new(); let mut out = Vec::new();
        while done.len() < self.deps.len() {
            let mut ready: Vec<_> = self.deps.iter().filter(|(n, ds)| !done.contains(*n) && ds.iter().all(|d| done.contains(d))).map(|(n, _)| n.clone()).collect();
            if ready.is_empty() { break; }
            ready.sort();
            for n in ready { done.insert(n.clone()); if n != self.target_name { out.push(n); } }
        }
        out
    }
    pub fn fingerprint(&self, project: &Project) -> Result<String, String> {
        let mut h = blake3::Hasher::new();
        h.update(b"gamma-cache-v3\0leanprover/lean4:v4.36.0\0");
        let files: Vec<_> = self.source.iter().collect();
        let parts: Result<Vec<_>, String> = files.par_iter().map(|(name, path)| {
            let bytes = fs::read(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
            let mut part = blake3::Hasher::new();
            part.update(name.as_bytes()); part.update(b"\0"); part.update(&bytes);
            Ok(*part.finalize().as_bytes())
        }).collect();
        for part in parts? { h.update(&part); }
        for p in &project.search_paths { h.update(p.as_os_str().to_string_lossy().as_bytes()); h.update(b"\0"); }
        Ok(h.finalize().to_hex().to_string())
    }
}
