#![no_main]

use algesum::{
    CanonicalGraphDag, CanonicalGraphDagLimits, CanonicalGraphDocument, CanonicalSearchBudget,
    GraphSchemaId, Microcanon,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = CanonicalGraphDocument::from_bytes(data);
    let canonizer = Microcanon::new(GraphSchemaId::derive(b"rc7-fuzz-graph-v1"));
    let _ = CanonicalGraphDag::from_canonical_bytes(
        data,
        &canonizer,
        CanonicalSearchBudget::new(20_000),
        CanonicalGraphDagLimits::default(),
    );
});
