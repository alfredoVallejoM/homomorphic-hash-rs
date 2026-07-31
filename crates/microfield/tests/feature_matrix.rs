//! Feature-independent layout and thread-safety checks.

use microfield::{ArtifactBundleDigest, ArtifactId, F2, FieldId};

fn assert_send_sync_static<T: Send + Sync + 'static>() {}

#[test]
fn public_value_objects_are_thread_safe() {
    assert_send_sync_static::<F2>();
    assert_send_sync_static::<FieldId>();
    assert_send_sync_static::<ArtifactId>();
    assert_send_sync_static::<ArtifactBundleDigest>();
}

#[test]
fn identifiers_have_stable_layout() {
    assert_eq!(core::mem::size_of::<FieldId>(), 32);
    assert_eq!(core::mem::size_of::<ArtifactId>(), 32);
    assert_eq!(core::mem::size_of::<ArtifactBundleDigest>(), 32);
}
