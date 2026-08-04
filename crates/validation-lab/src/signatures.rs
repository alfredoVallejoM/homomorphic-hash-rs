use std::collections::{BTreeMap, BTreeSet};

use homomorphic_hash_rs::{
    AdditiveSignature, BidirectionalSequenceSignature, CanonicalElementEncoder,
    MultiEvaluationMultisetSignature, MultisetSignature, SequenceSignature,
};
use microfield::Fp251V1;

use crate::model::{
    CollisionExample, CollisionProfile, SignatureApplicationResult, SignatureCampaignReport,
    ValidationManifest,
};

pub fn run_campaign(manifest: &ValidationManifest) -> Result<SignatureCampaignReport, String> {
    let words = enumerate_words(
        manifest.signature.alphabet_size,
        manifest.signature.collision_max_length,
    );
    let metamorphic_words = enumerate_words(
        manifest.signature.alphabet_size,
        manifest.signature.exhaustive_max_length,
    );
    let mut checks = 0_u64;
    for word in &metamorphic_words {
        check_word_laws(word, &mut checks)?;
    }

    let mut profiles = Vec::new();
    let mut examples = Vec::new();
    for (name, key) in [
        ("additive-f251-v1", SignatureKind::Additive),
        ("sequence-f251-v1", SignatureKind::Sequence),
        (
            "bidirectional-sequence-f251-v1",
            SignatureKind::Bidirectional,
        ),
        ("multiset-k1-f251-v1", SignatureKind::Multiset1),
        ("multiset-k2-f251-v1", SignatureKind::Multiset2),
        ("multiset-k4-f251-v1", SignatureKind::Multiset4),
    ] {
        let (profile, example) = collision_profile(name, key, &words)?;
        profiles.push(profile);
        if let Some(example) = example {
            examples.push(example);
        }
    }

    // Mandatory negative control: the algebraic residual validates its own
    // equation for an assumed suffix, not membership in the original input.
    let mut sequence = sequence();
    sequence.push(&[1]).map_err(debug_error)?;
    sequence.push(&[2]).map_err(debug_error)?;
    let false_candidate = [3];
    let derived = sequence
        .residual_assuming_last(&false_candidate)
        .map_err(debug_error)?;
    if !sequence
        .verify_residual(&false_candidate, &derived)
        .map_err(debug_error)?
    {
        return Err(
            "residual negative control no longer demonstrates equation-only semantics".into(),
        );
    }

    Ok(SignatureCampaignReport {
        enumerated_words: u64::try_from(words.len()).map_err(|_| "word count overflow")?,
        metamorphic_checks: checks,
        collision_profiles: profiles,
        minimum_examples: examples,
        residual_membership_control:
            "confirmed: a fabricated last item can derive a recomposing residual; this is not membership"
                .into(),
        applications: vec![
            SignatureApplicationResult {
                application: "distributed additive and multiset aggregation".into(),
                classification: "ValidatedPrimitive".into(),
                evidence: format!(
                    "{checks} exhaustive split/merge equations passed across canonical words"
                ),
                required_confirmation: "exact state or domain-specific store after candidate filtering"
                    .into(),
            },
            SignatureApplicationResult {
                application: "parallel sequence chunk composition".into(),
                classification: "ValidatedPrimitive".into(),
                evidence: "every split through length six equals direct Horner evaluation; performance runner compares O(1) merge against reread and SHA-256"
                    .into(),
                required_confirmation: "exact bytes or cryptographic digest when identity assurance is required"
                    .into(),
            },
            SignatureApplicationResult {
                application: "multi-evaluation candidate index".into(),
                classification: "Experimental".into(),
                evidence: "K=2 and K=4 remove the K=1 collision in the frozen small-domain campaign"
                    .into(),
                required_confirmation: "exact multiset comparison; fixed evaluation points provide no universal probability guarantee"
                    .into(),
            },
        ],
    })
}

#[derive(Clone, Copy)]
enum SignatureKind {
    Additive,
    Sequence,
    Bidirectional,
    Multiset1,
    Multiset2,
    Multiset4,
}

fn collision_profile(
    name: &str,
    kind: SignatureKind,
    words: &[Vec<u8>],
) -> Result<(CollisionProfile, Option<CollisionExample>), String> {
    let mut inputs = BTreeSet::new();
    for word in words {
        inputs.insert(semantic_input(kind, word));
    }
    let mut buckets: BTreeMap<Vec<u8>, Vec<Vec<u8>>> = BTreeMap::new();
    for input in &inputs {
        buckets
            .entry(signature_key(kind, input)?)
            .or_default()
            .push(input.clone());
    }
    let collisions: Vec<_> = buckets.values().filter(|bucket| bucket.len() > 1).collect();
    let minimum = collisions
        .iter()
        .flat_map(|bucket| bucket.iter().map(Vec::len))
        .min();
    let example = collisions
        .iter()
        .flat_map(|bucket| bucket.windows(2).next())
        .min_by_key(|pair| pair[0].len().max(pair[1].len()))
        .map(|pair| CollisionExample {
            signature: name.into(),
            left: pair[0].clone(),
            right: pair[1].clone(),
            classification: match kind {
                SignatureKind::Additive => "finite-field sum alias after commutative normalization",
                SignatureKind::Sequence | SignatureKind::Bidirectional => {
                    "fixed-point polynomial evaluation alias"
                }
                _ => "fixed-point characteristic-polynomial evaluation alias",
            }
            .into(),
        });
    Ok((
        CollisionProfile {
            signature: name.into(),
            semantic_inputs: inputs.len() as u64,
            distinct_outputs: buckets.len() as u64,
            collision_buckets: collisions.len() as u64,
            colliding_inputs: collisions.iter().map(|bucket| bucket.len() as u64).sum(),
            minimum_colliding_size: minimum,
        },
        example,
    ))
}

fn semantic_input(kind: SignatureKind, word: &[u8]) -> Vec<u8> {
    match kind {
        SignatureKind::Additive
        | SignatureKind::Multiset1
        | SignatureKind::Multiset2
        | SignatureKind::Multiset4 => {
            let mut normalized = word.to_vec();
            normalized.sort_unstable();
            normalized
        }
        SignatureKind::Sequence | SignatureKind::Bidirectional => word.to_vec(),
    }
}

fn signature_key(kind: SignatureKind, input: &[u8]) -> Result<Vec<u8>, String> {
    match kind {
        SignatureKind::Additive => {
            let mut value = AdditiveSignature::<Fp251V1, _>::new(CanonicalElementEncoder);
            for item in input {
                value.absorb(&[*item]).map_err(debug_error)?;
            }
            Ok(value.to_canonical_bytes())
        }
        SignatureKind::Sequence => {
            let mut value = sequence();
            for item in input {
                value.push(&[*item]).map_err(debug_error)?;
            }
            Ok(value.to_canonical_bytes())
        }
        SignatureKind::Bidirectional => {
            let mut value = bidirectional();
            for item in input {
                value.push(&[*item]).map_err(debug_error)?;
            }
            Ok(value.to_canonical_bytes())
        }
        SignatureKind::Multiset1 => multiset_key::<1>(input, [fp(17)]),
        SignatureKind::Multiset2 => multiset_key::<2>(input, [fp(17), fp(53)]),
        SignatureKind::Multiset4 => multiset_key::<4>(input, [fp(17), fp(53), fp(101), fp(199)]),
    }
}

fn multiset_key<const K: usize>(input: &[u8], offsets: [Fp251V1; K]) -> Result<Vec<u8>, String> {
    let mut value = MultiEvaluationMultisetSignature::new(CanonicalElementEncoder, offsets)
        .map_err(debug_error)?;
    for item in input {
        value.insert(&[*item]).map_err(debug_error)?;
    }
    Ok(value.to_canonical_bytes())
}

fn check_word_laws(word: &[u8], checks: &mut u64) -> Result<(), String> {
    let mut direct_sequence = sequence();
    let mut direct_bidirectional = bidirectional();
    let mut direct_multiset = MultisetSignature::new(CanonicalElementEncoder, fp(17));
    let mut direct_additive = AdditiveSignature::<Fp251V1, _>::new(CanonicalElementEncoder);
    for item in word {
        direct_sequence.push(&[*item]).map_err(debug_error)?;
        direct_bidirectional.push(&[*item]).map_err(debug_error)?;
        direct_multiset.insert(&[*item]).map_err(debug_error)?;
        direct_additive.absorb(&[*item]).map_err(debug_error)?;
    }
    for split in 0..=word.len() {
        let mut left_sequence = sequence();
        let mut right_sequence = sequence();
        let mut left_bidirectional = bidirectional();
        let mut right_bidirectional = bidirectional();
        let mut left_multiset = MultisetSignature::new(CanonicalElementEncoder, fp(17));
        let mut right_multiset = MultisetSignature::new(CanonicalElementEncoder, fp(17));
        let mut left_additive = AdditiveSignature::<Fp251V1, _>::new(CanonicalElementEncoder);
        let mut right_additive = AdditiveSignature::<Fp251V1, _>::new(CanonicalElementEncoder);
        for item in &word[..split] {
            left_sequence.push(&[*item]).map_err(debug_error)?;
            left_bidirectional.push(&[*item]).map_err(debug_error)?;
            left_multiset.insert(&[*item]).map_err(debug_error)?;
            left_additive.absorb(&[*item]).map_err(debug_error)?;
        }
        for item in &word[split..] {
            right_sequence.push(&[*item]).map_err(debug_error)?;
            right_bidirectional.push(&[*item]).map_err(debug_error)?;
            right_multiset.insert(&[*item]).map_err(debug_error)?;
            right_additive.absorb(&[*item]).map_err(debug_error)?;
        }
        if left_sequence
            .concatenate(&right_sequence)
            .map_err(debug_error)?
            != direct_sequence
            || left_bidirectional
                .concatenate(&right_bidirectional)
                .map_err(debug_error)?
                != direct_bidirectional
            || left_multiset
                .combine(&right_multiset)
                .map_err(debug_error)?
                != direct_multiset
            || left_additive
                .combine(&right_additive)
                .map_err(debug_error)?
                != direct_additive
        {
            return Err(format!("partition law failed for {word:?} at {split}"));
        }
        *checks += 4;
    }
    Ok(())
}

fn sequence() -> SequenceSignature<Fp251V1, CanonicalElementEncoder> {
    SequenceSignature::new(CanonicalElementEncoder, fp(11)).expect("non-zero base")
}

fn bidirectional() -> BidirectionalSequenceSignature<Fp251V1, CanonicalElementEncoder> {
    BidirectionalSequenceSignature::new(CanonicalElementEncoder, fp(11)).expect("non-zero base")
}

fn fp(value: u64) -> Fp251V1 {
    Fp251V1::from_u64_mod(value)
}

fn enumerate_words(alphabet_size: u8, maximum_length: usize) -> Vec<Vec<u8>> {
    let mut result = vec![Vec::new()];
    let mut level = vec![Vec::new()];
    for _ in 0..maximum_length {
        let mut next = Vec::with_capacity(level.len() * usize::from(alphabet_size));
        for prefix in &level {
            for symbol in 0..alphabet_size {
                let mut word = prefix.clone();
                word.push(symbol);
                next.push(word);
            }
        }
        result.extend(next.iter().cloned());
        level = next;
    }
    result
}

fn debug_error(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use microfield::{CanonicalEncoding, Field, Gf2_256HhV1};

    #[test]
    fn exhaustive_partition_laws_cover_every_split_through_length_six() {
        let mut checks = 0;
        for word in enumerate_words(4, 6) {
            check_word_laws(&word, &mut checks).unwrap();
        }
        assert_eq!(checks, 145_636);
    }

    #[test]
    fn more_evaluation_points_do_not_increase_observed_multiset_collisions() {
        let words = enumerate_words(5, 5);
        let one = collision_profile("k1", SignatureKind::Multiset1, &words)
            .unwrap()
            .0;
        let two = collision_profile("k2", SignatureKind::Multiset2, &words)
            .unwrap()
            .0;
        let four = collision_profile("k4", SignatureKind::Multiset4, &words)
            .unwrap()
            .0;
        assert!(two.colliding_inputs <= one.colliding_inputs);
        assert!(four.colliding_inputs <= two.colliding_inputs);
    }

    #[test]
    fn binary_field_partition_laws_hold_exhaustively_on_small_words() {
        let elements: Vec<_> = (0_u8..3)
            .map(|value| {
                let mut repr = [0_u8; 32];
                repr[0] = value;
                Gf2_256HhV1::from_canonical(&repr).unwrap()
            })
            .collect();
        for word in enumerate_words(3, 4) {
            let field_word: Vec<_> = word.iter().map(|&value| elements[value as usize]).collect();
            let mut direct_add = AdditiveSignature::<Gf2_256HhV1, _>::new(CanonicalElementEncoder);
            let mut direct_sequence =
                SequenceSignature::new(CanonicalElementEncoder, elements[2]).unwrap();
            let mut direct_multiset =
                MultisetSignature::new(CanonicalElementEncoder, Gf2_256HhV1::ONE);
            direct_add
                .absorb_elements(field_word.iter().copied())
                .unwrap();
            direct_sequence
                .push_elements(field_word.iter().copied())
                .unwrap();
            direct_multiset
                .insert_elements(field_word.iter().copied())
                .unwrap();
            for split in 0..=field_word.len() {
                let mut left_add =
                    AdditiveSignature::<Gf2_256HhV1, _>::new(CanonicalElementEncoder);
                let mut right_add =
                    AdditiveSignature::<Gf2_256HhV1, _>::new(CanonicalElementEncoder);
                let mut left_sequence =
                    SequenceSignature::new(CanonicalElementEncoder, elements[2]).unwrap();
                let mut right_sequence =
                    SequenceSignature::new(CanonicalElementEncoder, elements[2]).unwrap();
                let mut left_multiset =
                    MultisetSignature::new(CanonicalElementEncoder, Gf2_256HhV1::ONE);
                let mut right_multiset =
                    MultisetSignature::new(CanonicalElementEncoder, Gf2_256HhV1::ONE);
                left_add
                    .absorb_elements(field_word[..split].iter().copied())
                    .unwrap();
                right_add
                    .absorb_elements(field_word[split..].iter().copied())
                    .unwrap();
                left_sequence
                    .push_elements(field_word[..split].iter().copied())
                    .unwrap();
                right_sequence
                    .push_elements(field_word[split..].iter().copied())
                    .unwrap();
                left_multiset
                    .insert_elements(field_word[..split].iter().copied())
                    .unwrap();
                right_multiset
                    .insert_elements(field_word[split..].iter().copied())
                    .unwrap();
                assert_eq!(left_add.combine(&right_add).unwrap(), direct_add);
                assert_eq!(
                    left_sequence.concatenate(&right_sequence).unwrap(),
                    direct_sequence
                );
                assert_eq!(
                    left_multiset.combine(&right_multiset).unwrap(),
                    direct_multiset
                );
            }
        }
    }
}
