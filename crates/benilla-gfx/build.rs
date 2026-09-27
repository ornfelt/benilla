//! Finds the gfx library, only with the `gfx` feature on, the way `wc_clean_new_rs/build.rs`
//! does: `$GFX_DIR` names a gfx checkout (the folder holding `bin/<Configuration>_x64`), else
//! `$code_root_dir/Code2/General/gfx/gfx_dll/gfx_benilla` when it exists, else `.../gfx`. The
//! configuration is `$GFX_CONFIGURATION`, else `Release` when built, else `Debug`. The library is
//! copied beside the binaries; the `benilla` launcher sets the rpath that finds it there.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=GFX_DIR");
    println!("cargo:rerun-if-env-changed=GFX_CONFIGURATION");
    println!("cargo:rerun-if-env-changed=code_root_dir");
    if env::var_os("CARGO_FEATURE_GFX").is_none() {
        return;
    }

    let gfx_dir = gfx_dir();
    let configuration = env::var("GFX_CONFIGURATION").unwrap_or_else(|_| {
        if gfx_dir.join("bin").join("Release_x64").is_dir() {
            "Release".into()
        } else {
            "Debug".into()
        }
    });
    let bin_dir = gfx_dir.join("bin").join(format!("{configuration}_x64"));
    let lib_dir = gfx_dir.join("lib").join(format!("{configuration}_x64"));

    // `OUT_DIR` is `target/<profile>/build/<pkg>/out`; three up is `target/<profile>`, where the
    // binaries land and `cargo run`/`cargo test` put link-search paths on the loader path.
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let target_dir = out_dir
        .ancestors()
        .nth(3)
        .expect("the cargo target dir")
        .to_path_buf();

    let names: &[&str] = if cfg!(windows) {
        &["gfx.dll", "SDL2.dll"]
    } else if cfg!(target_os = "macos") {
        &["libgfx.dylib"]
    } else {
        &["libgfx.so"]
    };
    let mut found = false;
    for name in names {
        let src = bin_dir.join(name);
        if src.exists() {
            println!("cargo:rerun-if-changed={}", src.display());
            copy(&src, &target_dir.join(name));
            found |= name.contains("gfx");
        }
    }
    if cfg!(windows) {
        // The import library links; the DLL beside the exe loads.
        let import = lib_dir.join("gfx.lib");
        if import.exists() {
            println!("cargo:rerun-if-changed={}", import.display());
            copy(&import, &target_dir.join("gfx.lib"));
        }
    }
    if !found {
        panic!(
            "benilla-gfx: the gfx library is not built in {} (build it there, or set GFX_DIR / \
             GFX_CONFIGURATION)",
            bin_dir.display()
        );
    }
    println!("cargo:rustc-link-search=native={}", target_dir.display());
    // This crate's examples live one below the library, in `target/<profile>/examples`.
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        println!("cargo:rustc-link-arg-examples=-Wl,-rpath,$ORIGIN/..");
    }
}

fn gfx_dir() -> PathBuf {
    if let Some(dir) = env::var_os("GFX_DIR") {
        return PathBuf::from(dir);
    }
    let root = env::var("code_root_dir").unwrap_or_else(|_| {
        if cfg!(windows) {
            r"C:\Users\jonas".into()
        } else {
            "/home/jonas".into()
        }
    });
    let dll = PathBuf::from(root).join("Code2/General/gfx/gfx_dll");
    let benilla = dll.join("gfx_benilla");
    // The parent, not `gfx_benilla` itself: a watched path that does not exist reruns every build.
    println!("cargo:rerun-if-changed={}", dll.display());
    if benilla.is_dir() {
        benilla
    } else {
        dll.join("gfx")
    }
}

fn copy(src: &Path, dst: &Path) {
    // Skip an unchanged copy, so a rebuild does not touch a library a running binary has mapped.
    if let (Ok(a), Ok(b)) = (fs::metadata(src), fs::metadata(dst)) {
        if a.len() == b.len() && a.modified().ok() <= b.modified().ok() {
            return;
        }
    }
    fs::copy(src, dst)
        .unwrap_or_else(|e| panic!("copy {} -> {}: {e}", src.display(), dst.display()));
}
