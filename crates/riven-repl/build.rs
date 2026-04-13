use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let runtime_c = PathBuf::from(&crate_dir)
        .parent()
        .unwrap()
        .join("riven-core")
        .join("runtime")
        .join("runtime.c");

    if !runtime_c.exists() {
        panic!("Runtime source not found at {:?}", runtime_c);
    }

    cc::Build::new()
        .file(&runtime_c)
        .opt_level(2)
        .warnings(true)
        .compile("rivenrt");

    println!("cargo:rerun-if-changed={}", runtime_c.display());
}
