//! RC.4 differential contract for fixed chunks and hierarchical summaries.

#![cfg(feature = "signatures")]

use homomorphic_hash_rs::{
    BinaryPolynomialEncoder, FileChunkProfile, HomomorphicSummaryTree, SummaryEditPath,
    SummaryTreeError, SummaryTreeLimits,
};
use microfield::{BinaryPolynomialField, Gf2_128V1};
use rand::{rngs::StdRng, Rng, SeedableRng};
use structural_field_fixture::Gf2_9StructuralFixture;

type Encoder = BinaryPolynomialEncoder;
type Tree = HomomorphicSummaryTree<Gf2_128V1, Encoder>;

fn encoder() -> Encoder {
    BinaryPolynomialEncoder::new(0x5243_0004)
}

fn base() -> Gf2_128V1 {
    Gf2_128V1::from_polynomial_bytes_mod(&[2])
}

fn rebuild(profile: FileChunkProfile, bytes: &[u8]) -> Tree {
    Tree::from_bytes(profile, encoder(), base(), bytes).unwrap()
}

#[test]
fn chunk_profile_binds_root_identity_and_all_boundary_sizes_rebuild() {
    let profile = FileChunkProfile::fixed(16).unwrap();
    let other = FileChunkProfile::fixed(8).unwrap();
    assert_ne!(profile.profile_id(), other.profile_id());

    for length in [0_usize, 1, 15, 16, 17, 31, 32, 33, 255] {
        let bytes = (0..length).map(|index| index as u8).collect::<Vec<_>>();
        let tree = rebuild(profile, &bytes);
        assert_eq!(tree.byte_len(), length);
        assert_eq!(tree.chunk_count(), length.div_ceil(16));
        assert_eq!(tree.to_file_bytes().unwrap(), bytes);
        assert_eq!(tree.root(), rebuild(profile, &bytes).root());
        assert_eq!(tree.root().profile_id(), profile.profile_id());
        assert_eq!(&tree.root().to_canonical_bytes()[..4], b"MFSR");

        let other_tree = rebuild(other, &bytes);
        assert_ne!(tree.root().profile_id(), other_tree.root().profile_id());
        assert_ne!(
            tree.root().to_canonical_bytes(),
            other_tree.root().to_canonical_bytes()
        );
    }
}

#[test]
fn local_replacement_touches_only_leaf_paths_and_matches_rebuild() {
    let profile = FileChunkProfile::fixed(64).unwrap();
    let mut exact = (0..4_096).map(|index| index as u8).collect::<Vec<_>>();
    let mut tree = rebuild(profile, &exact);
    let original_chunks = tree.chunk_count();

    exact[1_301..1_309].copy_from_slice(b"LOCALITY");
    let report = tree.replace_range(1_301..1_309, b"LOCALITY").unwrap();
    assert_eq!(report.path(), SummaryEditPath::LocalTree);
    assert_eq!(report.touched_leaves(), 1);
    assert!(report.recomputed_nodes() <= tree.chunk_count().ilog2() as usize + 2);
    assert_eq!(tree.chunk_count(), original_chunks);
    assert_eq!(tree.to_file_bytes().unwrap(), exact);
    assert_eq!(tree.root(), rebuild(profile, &exact).root());

    let replacement = vec![0xa5; 150];
    exact[60..210].copy_from_slice(&replacement);
    let report = tree.replace_range(60..210, &replacement).unwrap();
    assert_eq!(report.path(), SummaryEditPath::LocalTree);
    assert_eq!(report.touched_leaves(), 4);
    assert!(report.recomputed_nodes() < tree.chunk_count());
    assert_eq!(tree.root(), rebuild(profile, &exact).root());
}

#[test]
fn random_range_campaign_matches_exact_bytes_and_rebuild_after_every_edit() {
    let profile = FileChunkProfile::fixed(23).unwrap();
    let mut rng = StdRng::seed_from_u64(0x5243_4004);
    let mut exact = (0..257).map(|_| rng.gen()).collect::<Vec<u8>>();
    let mut tree = rebuild(profile, &exact);
    let mut committed = 0_u64;

    for step in 0..600 {
        let operation = rng.gen_range(0..5);
        let old_len = exact.len();
        let report = match operation {
            0 if !exact.is_empty() => {
                let start = rng.gen_range(0..exact.len());
                let end = rng.gen_range(start + 1..=exact.len());
                let replacement = (start..end).map(|_| rng.gen()).collect::<Vec<u8>>();
                exact[start..end].copy_from_slice(&replacement);
                tree.replace_range(start..end, &replacement).unwrap()
            }
            1 => {
                let offset = rng.gen_range(0..=exact.len());
                let inserted = (0..rng.gen_range(1..=12))
                    .map(|_| rng.gen())
                    .collect::<Vec<u8>>();
                exact.splice(offset..offset, inserted.iter().copied());
                tree.insert_range(offset, &inserted).unwrap()
            }
            2 if !exact.is_empty() => {
                let start = rng.gen_range(0..exact.len());
                let end = rng.gen_range(start + 1..=exact.len());
                exact.drain(start..end);
                tree.remove_range(start..end).unwrap()
            }
            3 => {
                let appended = (0..rng.gen_range(1..=12))
                    .map(|_| rng.gen())
                    .collect::<Vec<u8>>();
                exact.extend_from_slice(&appended);
                tree.append(&appended).unwrap()
            }
            _ => {
                let new_len = rng.gen_range(0..=exact.len());
                exact.truncate(new_len);
                tree.truncate(new_len).unwrap()
            }
        };
        if report.path() != SummaryEditPath::NoChange {
            committed += 1;
        }
        if exact.len() == old_len && report.path() != SummaryEditPath::NoChange {
            assert_eq!(report.path(), SummaryEditPath::LocalTree, "step {step}");
        } else if exact.len() != old_len {
            assert_eq!(
                report.path(),
                SummaryEditPath::BoundaryRebuild,
                "step {step}"
            );
        }
        assert_eq!(report.revision(), committed);
        assert_eq!(tree.revision(), committed);
        assert_eq!(tree.to_file_bytes().unwrap(), exact, "step {step}");
        assert_eq!(tree.root(), rebuild(profile, &exact).root(), "step {step}");
    }
}

#[test]
fn checkpoint_round_trips_revision_and_rejects_every_truncation() {
    let profile = FileChunkProfile::fixed(17).unwrap();
    let original = (0..333).map(|index| (index * 13) as u8).collect::<Vec<_>>();
    let mut tree = rebuild(profile, &original);
    tree.replace_range(30..34, b"tree").unwrap();
    tree.append(b"checkpoint").unwrap();
    let expected_file = tree.to_file_bytes().unwrap();
    let checkpoint = tree.to_checkpoint_bytes().unwrap();
    assert_eq!(&checkpoint[..4], b"MFST");

    let restored = Tree::from_checkpoint_bytes(
        profile,
        encoder(),
        base(),
        &checkpoint,
        SummaryTreeLimits::default(),
    )
    .unwrap();
    assert_eq!(restored.root(), tree.root());
    assert_eq!(restored.revision(), tree.revision());
    assert_eq!(restored.to_file_bytes().unwrap(), expected_file);

    for length in 0..checkpoint.len() {
        assert!(
            Tree::from_checkpoint_bytes(
                profile,
                encoder(),
                base(),
                &checkpoint[..length],
                SummaryTreeLimits::default(),
            )
            .is_err(),
            "accepted truncated checkpoint at byte {length}"
        );
    }
    let mut trailing = checkpoint.clone();
    trailing.push(0);
    assert!(Tree::from_checkpoint_bytes(
        profile,
        encoder(),
        base(),
        &trailing,
        SummaryTreeLimits::default()
    )
    .is_err());
    assert!(Tree::from_checkpoint_bytes(
        FileChunkProfile::fixed(19).unwrap(),
        encoder(),
        base(),
        &checkpoint,
        SummaryTreeLimits::default()
    )
    .is_err());

    let mut corrupted_root = checkpoint.clone();
    corrupted_root[64] ^= 1;
    assert!(Tree::from_checkpoint_bytes(
        profile,
        encoder(),
        base(),
        &corrupted_root,
        SummaryTreeLimits::default()
    )
    .is_err());
    let mut corrupted_file = checkpoint.clone();
    *corrupted_file.last_mut().unwrap() ^= 1;
    assert!(Tree::from_checkpoint_bytes(
        profile,
        encoder(),
        base(),
        &corrupted_file,
        SummaryTreeLimits::default()
    )
    .is_err());
}

#[test]
fn invalid_edits_and_limits_are_atomic() {
    let profile = FileChunkProfile::fixed(8).unwrap();
    let limits = SummaryTreeLimits {
        max_file_bytes: 32,
        max_chunks: 4,
        max_chunk_bytes: 8,
        max_checkpoint_bytes: 1_024,
    };
    let mut tree =
        Tree::from_bytes_with_limits(profile, encoder(), base(), b"0123456789", limits).unwrap();
    let before = tree.to_checkpoint_bytes().unwrap();

    let reversed_start = 9;
    let reversed_end = 4;
    assert_eq!(
        tree.replace_range(reversed_start..reversed_end, b"bad"),
        Err(SummaryTreeError::InvalidRange)
    );
    assert!(matches!(
        tree.append(&[0; 32]),
        Err(SummaryTreeError::LimitExceeded("file bytes"))
    ));
    assert_eq!(tree.revision(), 0);
    assert_eq!(tree.to_checkpoint_bytes().unwrap(), before);

    let restrictive = SummaryTreeLimits {
        max_checkpoint_bytes: before.len() - 1,
        ..limits
    };
    assert!(matches!(
        Tree::from_checkpoint_bytes(profile, encoder(), base(), &before, restrictive),
        Err(SummaryTreeError::LimitExceeded("checkpoint bytes"))
    ));
}

#[test]
fn generated_external_field_uses_the_same_tree_without_adapters() {
    let profile = FileChunkProfile::fixed(7).unwrap();
    let encoder = BinaryPolynomialEncoder::new(0x9004);
    let base = Gf2_9StructuralFixture::from_polynomial_bytes_mod(&[2]);
    let bytes = b"external generated field summary tree";
    let mut tree = HomomorphicSummaryTree::<Gf2_9StructuralFixture, _>::from_bytes(
        profile, encoder, base, bytes,
    )
    .unwrap();
    let report = tree.replace_range(9..18, b"GENERATED").unwrap();
    assert_eq!(report.path(), SummaryEditPath::LocalTree);
    let rebuilt = HomomorphicSummaryTree::<Gf2_9StructuralFixture, _>::from_bytes(
        profile,
        encoder,
        base,
        &tree.to_file_bytes().unwrap(),
    )
    .unwrap();
    assert_eq!(tree.root(), rebuilt.root());
}
