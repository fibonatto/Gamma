mod backend;
mod cache;
mod diagnostics;
mod graph;
mod output;
mod project;

use std::{env, path::PathBuf, process::ExitCode, time::Instant};
use crate::{backend::Backend, cache::Cache, graph::ModuleGraph, output::print_output, project::Project};

fn run() -> Result<bool, String> {
    let started = Instant::now();
    let mut args = env::args_os().skip(1);
    let target = args.next().map(PathBuf::from).ok_or_else(|| "Usage: gamma <file.lean>".to_owned())?;
    if args.next().is_some() { return Err("Usage: gamma <file.lean>".into()); }
    let target = target.canonicalize().map_err(|e| format!("cannot resolve target: {e}"))?;
    let project = Project::discover(&target)?;
    project.configure_lean_path();

    let graph = ModuleGraph::discover(&project, &target)?;
    let fingerprint = graph.fingerprint(&project)?;
    let mut cache = Cache::open(&project.root);
    let json = if let Some(hit) = cache.get(&fingerprint) {
        hit
    } else {
        let backend = Backend::initialize()?;
        let modules = graph.ordered_modules().join(",");
        let json = backend.inspect(&target, &modules)?;
        cache.put(fingerprint, &json)?;
        json
    };
    let parsed: diagnostics::InspectOutput = serde_json::from_str(&json)
        .map_err(|e| format!("Lean backend returned invalid JSON: {e}\n{json}"))?;
    print_output(&parsed, started.elapsed());
    Ok(parsed.has_errors)
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::from(1),
        Ok(false) => ExitCode::SUCCESS,
        Err(e) => { eprintln!("gamma: {e}"); ExitCode::from(2) }
    }
}
