use std::{env, fs, path::PathBuf, process::Command};

fn run(cmd: &mut Command, what: &str) -> String {
    let out = cmd.output().unwrap_or_else(|e| panic!("{what}: {e}"));
    if !out.status.success() {
        panic!(
            "{what} failed:\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let lean_dir = manifest.join("lean");
    let target = env::var("TARGET").unwrap();
    if target != "aarch64-apple-darwin" {
        panic!("gamma currently supports only aarch64-apple-darwin; got {target}");
    }

    println!("cargo:rerun-if-changed=lean");
    println!("cargo:rerun-if-changed=c/gamma_shim.c");
    println!("cargo:rerun-if-env-changed=LEAN_SYSROOT");
    println!("cargo:rerun-if-env-changed=LAKE");

    let lake = env::var("LAKE").unwrap_or_else(|_| "lake".into());
    let native_dir = lean_dir.join(".lake/build/lib");

    // Do not accidentally link a stale dylib produced with the old, mismatched
    // explicit libName configuration.
    if native_dir.exists() {
        for entry in fs::read_dir(&native_dir).expect("reading Lean native library directory") {
            let path = entry.expect("reading Lean native library entry").path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            if name.starts_with("lib")
                && name.ends_with("LeanChecker.dylib")
                && name != "libgamma_x2dbackend_LeanChecker.dylib"
            {
                fs::remove_file(&path)
                    .unwrap_or_else(|e| panic!("removing stale {}: {e}", path.display()));
            }
        }
    }


    run(
        Command::new(&lake)
            .arg("build")
            .arg("LeanChecker:shared")
            .current_dir(&lean_dir),
        "building Lean FFI backend with Lake",
    );

    let sysroot = if let Ok(p) = env::var("LEAN_SYSROOT") {
        PathBuf::from(p)
    } else {
        PathBuf::from(run(
            Command::new(&lake)
                .args(["env", "lean", "--print-prefix"])
                .current_dir(&lean_dir),
            "locating Lean sysroot",
        ))
    };
    if !sysroot.join("include/lean/lean.h").exists() {
        panic!("Lean headers not found under {}. Set LEAN_SYSROOT.", sysroot.display());
    }

    let checker_dylib = fs::read_dir(&native_dir)
        .unwrap_or_else(|e| panic!("reading {}: {e}", native_dir.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            name.starts_with("lib") && name.ends_with("LeanChecker.dylib")
        })
        .unwrap_or_else(|| {
            panic!("Lake did not produce lib*LeanChecker.dylib in {}", native_dir.display())
        });
    let checker_file = checker_dylib
        .file_name()
        .and_then(|n| n.to_str())
        .expect("UTF-8 Lean library filename");
    let checker_stem = checker_file
        .strip_prefix("lib")
        .and_then(|s| s.strip_suffix(".dylib"))
        .expect("Lean library filename must be libNAME.dylib");

    let init_symbol = format!("initialize_{checker_stem}");
    let lean_dylib = sysroot.join("lib/lean/libleanshared.dylib");
    if !lean_dylib.exists() {
        panic!(
            "expected {}. This build currently links the Lean shared runtime; bundle it for redistribution.",
            lean_dylib.display()
        );
    }

    cc::Build::new()
        .file("c/gamma_shim.c")
        .include(sysroot.join("include"))
        .define("LEAN_CHECKER_INIT_SYMBOL", Some(init_symbol.as_str()))
        .warnings(true)
        .flag_if_supported("-std=c11")
        .compile("gamma_shim");

    println!("cargo:rustc-link-search=native={}", native_dir.display());
    println!("cargo:rustc-link-search=native={}", sysroot.join("lib/lean").display());
    println!("cargo:rustc-link-lib=dylib={checker_stem}");
    println!("cargo:rustc-link-lib=dylib=leanshared");
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", native_dir.display());
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", sysroot.join("lib/lean").display());
    println!("cargo:rustc-env=GAMMA_SYSROOT={}", sysroot.display());
}
