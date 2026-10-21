//! Compiles the C shim and links libghostty-vt.
//!
//! Set `GHOSTTY_VT_DIR` to a `zig build -Demit-lib-vt` output directory
//! (the one holding `include/` and `lib/`). Defaults to `~/tools/ghostty/zig-out`.

use std::{env, path::PathBuf};

fn main() {
    let dir = env::var("GHOSTTY_VT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = env::var("USERPROFILE")
                .or_else(|_| env::var("HOME"))
                .expect("no home dir");
            PathBuf::from(home)
                .join("tools")
                .join("ghostty")
                .join("zig-out")
        });
    let include = dir.join("include");
    let lib = dir.join("lib");
    assert!(
        include.join("ghostty").join("vt.h").exists(),
        "libghostty-vt headers not found in {}",
        include.display()
    );

    cc::Build::new()
        .file("shim/hv.c")
        .include(&include)
        .include("shim")
        .define("GHOSTTY_STATIC", None)
        .warnings(false)
        .compile("hv");

    println!("cargo:rustc-link-search=native={}", lib.display());
    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-lib=static=ghostty-vt-static");
        println!("cargo:rustc-link-lib=ntdll");
        println!("cargo:rustc-link-lib=kernel32");
    } else {
        println!("cargo:rustc-link-lib=static=ghostty-vt");
    }
    println!("cargo:rerun-if-changed=shim/hv.c");
    println!("cargo:rerun-if-changed=shim/hv.h");
    println!("cargo:rerun-if-env-changed=GHOSTTY_VT_DIR");
}
