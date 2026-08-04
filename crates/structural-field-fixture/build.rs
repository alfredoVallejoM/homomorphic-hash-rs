use std::{env, path::PathBuf};

use microfield::generator::BinaryFieldFactory;

fn main() {
    println!("cargo:rerun-if-changed=field.toml");
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo always defines OUT_DIR"));
    BinaryFieldFactory::from_manifest("field.toml")
        .expect("fixture manifest must parse")
        .generate()
        .expect("fixture modulus must be irreducible")
        .emit_rust(output)
        .expect("generated module must publish atomically");
}
