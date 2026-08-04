//! Build-time generated binary field used only by structural integration tests.

#![no_std]

include!(concat!(env!("OUT_DIR"), "/gf2_9_structural_fixture.rs"));
