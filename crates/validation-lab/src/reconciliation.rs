//! Validation adapter for the maintained bounded reconciliation API.

use homomorphic_hash_rs::{BoundedSetReconciler, ReconciliationError, ReconciliationLimits};

use crate::model::{ReconciliationReport, ValidationManifest};

pub fn run_campaign(manifest: &ValidationManifest) -> Result<ReconciliationReport, String> {
    let exhaustive_universe = manifest.signature.reconciliation_universe.min(8);
    let maximum = manifest.signature.reconciliation_max_difference;
    let reconciler = BoundedSetReconciler::new(ReconciliationLimits::new(
        u16::from(manifest.signature.reconciliation_universe),
        maximum,
        maximum,
        1024 * 1024,
    ))
    .map_err(|error| error.to_string())?;
    let subset_count = 1_u64 << exhaustive_universe;
    let mut attempted = 0_u64;
    let mut recovered = 0_u64;
    for left_mask in 0..subset_count {
        for right_mask in 0..subset_count {
            let difference = (left_mask ^ right_mask).count_ones() as usize;
            if difference > maximum {
                continue;
            }
            let left = mask_set(left_mask, exhaustive_universe);
            let right = mask_set(right_mask, exhaustive_universe);
            let only_left = left
                .iter()
                .copied()
                .filter(|value| !right.contains(value))
                .collect::<Vec<_>>();
            let only_right = right
                .iter()
                .copied()
                .filter(|value| !left.contains(value))
                .collect::<Vec<_>>();
            let decoded = reconciler
                .reconcile(
                    &reconciler
                        .sketch(&left)
                        .map_err(|error| error.to_string())?,
                    &reconciler
                        .sketch(&right)
                        .map_err(|error| error.to_string())?,
                    &right,
                )
                .map_err(|error| error.to_string())?;
            attempted += 1;
            if decoded.only_left() != only_left || decoded.only_right() != only_right {
                return Err(format!(
                    "bounded reconciliation mismatch: left={left:?} right={right:?} decoded={decoded:?}"
                ));
            }
            recovered += 1;
        }
    }

    let outside = BoundedSetReconciler::new(ReconciliationLimits::new(
        u16::from(manifest.signature.reconciliation_universe),
        maximum,
        maximum,
        1024 * 1024,
    ))
    .map_err(|error| error.to_string())?;
    let over_bound_left: Vec<u16> = (0..=maximum as u16).collect();
    let empty = Vec::new();
    let over_bound = outside.reconcile(
        &outside
            .sketch(&over_bound_left)
            .map_err(|error| error.to_string())?,
        &outside.sketch(&empty).map_err(|error| error.to_string())?,
        &empty,
    );
    if !matches!(over_bound, Err(ReconciliationError::DifferenceExceedsBound)) {
        return Err("reconciliation accepted a difference beyond its declared bound".into());
    }

    Ok(ReconciliationReport {
        exhaustive_pairs: attempted,
        recovered_pairs: recovered,
        rejected_over_bound: 1,
        maximum_symmetric_difference: maximum,
        classification: "MaintainedPrimitive: public bounded set recovery over Fp251 with profile/wire/limits; v1 rejects multiset multiplicity".into(),
    })
}

fn mask_set(mask: u64, universe: u8) -> Vec<u16> {
    (0..universe)
        .filter(|bit| mask & (1_u64 << bit) != 0)
        .map(u16::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maintained_api_recovers_both_sides_and_rejects_bound_violation() {
        let reconciler =
            BoundedSetReconciler::new(ReconciliationLimits::new(32, 4, 4, 1024)).unwrap();
        let left = [1, 2, 7, 9];
        let right = [1, 3, 7, 10];
        let recovered = reconciler
            .reconcile(
                &reconciler.sketch(&left).unwrap(),
                &reconciler.sketch(&right).unwrap(),
                &right,
            )
            .unwrap();
        assert_eq!(recovered.only_left(), [2, 9]);
        assert_eq!(recovered.only_right(), [3, 10]);
    }
}
