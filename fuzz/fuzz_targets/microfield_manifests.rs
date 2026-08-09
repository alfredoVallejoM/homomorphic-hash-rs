#![no_main]

use libfuzzer_sys::fuzz_target;
use microfield::spec::{model::FieldManifest, PrimeFieldManifest};

fuzz_target!(|data: &[u8]| {
    let Ok(source) = core::str::from_utf8(data) else {
        return;
    };

    if let Ok(manifest) = FieldManifest::parse_toml(source) {
        if let Ok(normalized) = manifest.normalize() {
            let reparsed = FieldManifest::parse_toml(normalized.canonical_toml())
                .expect("normalized binary manifest must parse")
                .normalize()
                .expect("normalized binary manifest must remain valid");
            assert_eq!(reparsed, normalized);
        }
    }

    if let Ok(manifest) = PrimeFieldManifest::parse_toml(source) {
        if let Ok(normalized) = manifest.normalize() {
            let reparsed = PrimeFieldManifest::parse_toml(normalized.canonical_toml())
                .expect("normalized prime manifest must parse")
                .normalize()
                .expect("normalized prime manifest must remain valid");
            assert_eq!(reparsed, normalized);
        }
    }
});
