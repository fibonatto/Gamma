mod backend;
mod cache;
mod diagnostics;
mod graph;
mod output;
mod project;

use std::{
    env,
    path::PathBuf,
    process::ExitCode,
    time::{Duration, Instant},
};

use crate::{
    backend::Backend,
    cache::Cache,
    graph::ModuleGraph,
    output::{print_output, RunStats},
    project::Project,
};

#[derive(Clone, Copy)]
struct RusageSnapshot {
    user: Duration,
    sys: Duration,
    peak_rss_bytes: u64,
}

impl RusageSnapshot {
    #[cfg(unix)]
    fn capture() -> Self {
        let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
        if unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) } != 0 {
            return Self {
                user: Duration::ZERO,
                sys: Duration::ZERO,
                peak_rss_bytes: 0,
            };
        }

        let to_duration = |tv: libc::timeval| -> Duration {
            let secs = tv.tv_sec.max(0) as u64;
            let nanos = (tv.tv_usec.max(0) as u32).saturating_mul(1_000);
            Duration::new(secs, nanos)
        };

        #[cfg(any(target_os = "macos", target_os = "ios"))]
        let peak_rss_bytes = usage.ru_maxrss.max(0) as u64;

        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        let peak_rss_bytes = (usage.ru_maxrss.max(0) as u64).saturating_mul(1024);

        Self {
            user: to_duration(usage.ru_utime),
            sys: to_duration(usage.ru_stime),
            peak_rss_bytes,
        }
    }

    #[cfg(not(unix))]
    fn capture() -> Self {
        Self {
            user: Duration::ZERO,
            sys: Duration::ZERO,
            peak_rss_bytes: 0,
        }
    }
}

fn run() -> Result<bool, String> {
    let wall_start = Instant::now();
    let rusage_start = RusageSnapshot::capture();

    let mut args = env::args_os().skip(1);
    let target = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "Usage: gamma <file.lean>".to_owned())?;
    if args.next().is_some() {
        return Err("Usage: gamma <file.lean>".into());
    }
    let target = target
        .canonicalize()
        .map_err(|e| format!("cannot resolve target: {e}"))?;

    let graph_start = Instant::now();
    let project = Project::discover(&target)?;
    project.configure_lean_path();
    let graph = ModuleGraph::discover(&project, &target)?;
    let fingerprint = graph.fingerprint(&project)?;
    let ordered_modules = graph.ordered_modules();
    let module_count = ordered_modules.len() + 1;
    let graph_time = graph_start.elapsed();

    let inspect_start = Instant::now();
    let mut cache = Cache::open(&project.root);
    let (json, cache_hit) = if let Some(hit) = cache.get(&fingerprint) {
        (hit, true)
    } else {
        let backend = Backend::initialize()?;
        let modules = ordered_modules.join(",");
        let json = backend.inspect(&target, &modules)?;
        cache.put(fingerprint, &json)?;
        (json, false)
    };
    let parsed: diagnostics::InspectOutput = serde_json::from_str(&json)
        .map_err(|e| format!("Lean backend returned invalid JSON: {e}\n{json}"))?;
    let inspect_time = inspect_start.elapsed();

    let rusage_end = RusageSnapshot::capture();
    let stats = RunStats {
        wall: wall_start.elapsed(),
        graph_time,
        inspect_time,
        user_cpu: rusage_end.user.saturating_sub(rusage_start.user),
        sys_cpu: rusage_end.sys.saturating_sub(rusage_start.sys),
        peak_rss_bytes: rusage_end.peak_rss_bytes,
        cache_hit,
        module_count,
    };

    print_output(&parsed, &stats);
    Ok(parsed.has_errors)
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::from(1),
        Ok(false) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("gamma: {e}");
            ExitCode::from(2)
        }
    }
}
