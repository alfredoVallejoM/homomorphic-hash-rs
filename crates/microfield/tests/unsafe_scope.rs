//! Ensures H2.6 cannot silently expand its audited `unsafe` boundaries.

#![cfg(all(feature = "std", feature = "portable", feature = "builtin-fields"))]

use std::{fs, path::Path};

#[test]
fn unsafe_code_is_confined_to_isa_adapters_and_packed_storage() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut rust_files = Vec::new();
    collect_rust_files(&source, &mut rust_files);

    let pclmul = source.join("backend/x86_pclmul.rs");
    let pmull = source.join("backend/aarch64_pmull.rs");
    let packed_storage = source.join("engine/packed/storage.rs");
    let backend_module = source.join("backend/mod.rs");
    let packed_module = source.join("engine/packed/mod.rs");
    let mut unsafe_sites = 0;
    let mut allow_sites = 0;

    for path in rust_files {
        let text = fs::read_to_string(&path).expect("library source must be readable");
        for line in text.lines() {
            if [
                "unsafe fn",
                "unsafe {",
                "unsafe impl",
                "unsafe trait",
                "unsafe extern",
                "#[unsafe(",
            ]
            .iter()
            .any(|token| line.contains(token))
            {
                unsafe_sites += 1;
                assert!(
                    path == pclmul || path == pmull || path == packed_storage,
                    "unsafe code escaped the three audited boundaries: {}",
                    path.display()
                );
            }
            if line.contains("allow(unsafe_code)") {
                allow_sites += 1;
                assert!(
                    path == backend_module || path == packed_module,
                    "an additional module relaxed the unsafe lint: {}",
                    path.display()
                );
            }
        }
    }

    assert!(
        unsafe_sites > 0,
        "the gate must observe the audited wrapper"
    );
    assert_eq!(
        allow_sites, 3,
        "exactly three audited module exceptions are allowed"
    );
    let root = fs::read_to_string(source.join("lib.rs")).expect("crate root must be readable");
    assert!(root.contains("#![deny(unsafe_code)]"));
    let manifest = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .expect("crate manifest must be readable");
    assert!(manifest.contains("unsafe_code = \"deny\""));
}

fn collect_rust_files(directory: &Path, output: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(directory).expect("source directory must be readable") {
        let path = entry.expect("source entry must be readable").path();
        if path.is_dir() {
            collect_rust_files(&path, output);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            output.push(path);
        }
    }
}
