use std::{env, path::PathBuf};

use microfield::generator::BinaryFieldFactory;

fn main() {
    println!("cargo:rerun-if-changed=field.toml");
    println!("cargo:rerun-if-changed=field_233.toml");
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo always defines OUT_DIR"));
    for manifest in ["field.toml", "field_233.toml"] {
        BinaryFieldFactory::from_manifest(manifest)
            .expect("fixture manifest must parse")
            .generate()
            .expect("fixture modulus must be irreducible")
            .emit_rust(&output)
            .expect("generated module must publish atomically");
    }
}
