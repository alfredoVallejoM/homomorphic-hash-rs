//! Bounded set reconciliation built from characteristic-polynomial sketches.
//!
//! This is a validation protocol, not yet a public product API. It makes the
//! missing step explicit: a product signature alone cannot recover elements;
//! enough independent evaluations and a bounded decoder are required.

use microfield::{Field, Fp251V1, Invert, Pow};

use crate::model::{ReconciliationReport, ValidationManifest};

#[derive(Clone, Debug)]
struct Sketch {
    cardinality: usize,
    evaluations: Vec<Fp251V1>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RecoveredDifference {
    only_left: Vec<u8>,
    only_right: Vec<u8>,
}

pub fn run_campaign(manifest: &ValidationManifest) -> Result<ReconciliationReport, String> {
    let exhaustive_universe = manifest.signature.reconciliation_universe.min(8);
    let maximum = manifest.signature.reconciliation_max_difference;
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
            let actual = exact_difference(&left, &right);
            let decoded = reconcile(
                &left,
                &right,
                manifest.signature.reconciliation_universe,
                maximum,
            )?;
            attempted += 1;
            if decoded != actual {
                return Err(format!(
                    "bounded reconciliation mismatch: left={left:?} right={right:?} decoded={decoded:?}"
                ));
            }
            recovered += 1;
        }
    }

    let over_bound_left: Vec<u8> = (0..=maximum as u8).collect();
    let over_bound = reconcile(
        &over_bound_left,
        &[],
        manifest.signature.reconciliation_universe,
        maximum,
    );
    if over_bound.is_ok() {
        return Err("reconciliation accepted a difference beyond its declared bound".into());
    }

    Ok(ReconciliationReport {
        exhaustive_pairs: attempted,
        recovered_pairs: recovered,
        rejected_over_bound: 1,
        maximum_symmetric_difference: maximum,
        classification: "ValidatedPrimitive: end-to-end bounded recovery with unknown distance over F251; public API, multiset multiplicity and production-scale factorization remain pending".into(),
    })
}

fn reconcile(
    left: &[u8],
    right: &[u8],
    universe: u8,
    maximum_difference: usize,
) -> Result<RecoveredDifference, String> {
    validate_set(left, universe)?;
    validate_set(right, universe)?;
    let points: Vec<_> = (0..maximum_difference)
        .map(|index| fp(u64::from(universe) + 17 + index as u64))
        .collect();
    let left_sketch = sketch(left, &points);
    let right_sketch = sketch(right, &points);
    decode(
        &left_sketch,
        &right_sketch,
        right,
        &points,
        universe,
        maximum_difference,
    )
}

fn decode(
    left: &Sketch,
    right: &Sketch,
    receiver_set: &[u8],
    points: &[Fp251V1],
    universe: u8,
    maximum_difference: usize,
) -> Result<RecoveredDifference, String> {
    if left.cardinality == right.cardinality && left.evaluations == right.evaluations {
        return Ok(RecoveredDifference {
            only_left: Vec::new(),
            only_right: Vec::new(),
        });
    }
    let cardinality_delta = left.cardinality as isize - right.cardinality as isize;
    let ratios: Result<Vec<_>, _> = left
        .evaluations
        .iter()
        .zip(&right.evaluations)
        .map(|(&numerator, &denominator)| {
            denominator
                .invert()
                .map(|inverse| numerator.mul(inverse))
                .ok_or("evaluation point was a denominator root")
        })
        .collect();
    let ratios = ratios?;

    for difference in 1..=maximum_difference {
        for positive_degree in 0..=difference {
            let negative_degree = difference - positive_degree;
            if positive_degree as isize - negative_degree as isize != cardinality_delta {
                continue;
            }
            let Some(coefficients) = solve_candidate(
                &points[..difference],
                &ratios[..difference],
                positive_degree,
                negative_degree,
            ) else {
                continue;
            };
            let numerator = monic_polynomial(&coefficients[..positive_degree]);
            let denominator = monic_polynomial(&coefficients[positive_degree..]);
            let only_left = roots_in_universe(&numerator, universe);
            let only_right = roots_in_universe(&denominator, universe);
            if only_left.len() != positive_degree
                || only_right.len() != negative_degree
                || only_left.iter().any(|value| receiver_set.contains(value))
                || only_right.iter().any(|value| !receiver_set.contains(value))
                || !sketch_equation_holds(left, right, &numerator, &denominator, points)
            {
                continue;
            }
            let mut reconstructed: Vec<_> = receiver_set
                .iter()
                .copied()
                .filter(|value| !only_right.contains(value))
                .chain(only_left.iter().copied())
                .collect();
            reconstructed.sort_unstable();
            if sketch(&reconstructed, points).evaluations != left.evaluations {
                continue;
            }
            return Ok(RecoveredDifference {
                only_left,
                only_right,
            });
        }
    }
    Err(format!(
        "no reconciliation solution within symmetric-difference bound {maximum_difference}"
    ))
}

fn solve_candidate(
    points: &[Fp251V1],
    ratios: &[Fp251V1],
    positive_degree: usize,
    negative_degree: usize,
) -> Option<Vec<Fp251V1>> {
    let difference = positive_degree + negative_degree;
    let mut matrix = Vec::with_capacity(difference);
    for (&point, &ratio) in points.iter().zip(ratios) {
        let mut row = Vec::with_capacity(difference + 1);
        for power in 0..positive_degree {
            row.push(point.pow(&[power as u64]));
        }
        for power in 0..negative_degree {
            row.push(ratio.neg().mul(point.pow(&[power as u64])));
        }
        row.push(
            ratio
                .mul(point.pow(&[negative_degree as u64]))
                .sub(point.pow(&[positive_degree as u64])),
        );
        matrix.push(row);
    }
    solve_unique(matrix).ok()
}

fn sketch(values: &[u8], points: &[Fp251V1]) -> Sketch {
    Sketch {
        cardinality: values.len(),
        evaluations: points
            .iter()
            .map(|&point| {
                values.iter().fold(Fp251V1::ONE, |product, &value| {
                    product.mul(point.sub(fp(u64::from(value))))
                })
            })
            .collect(),
    }
}

fn solve_unique(mut matrix: Vec<Vec<Fp251V1>>) -> Result<Vec<Fp251V1>, String> {
    let size = matrix.len();
    if matrix.iter().any(|row| row.len() != size + 1) {
        return Err("non-square reconciliation system".into());
    }
    for column in 0..size {
        let pivot = (column..size)
            .find(|&row| !matrix[row][column].is_zero())
            .ok_or("singular reconciliation system")?;
        matrix.swap(column, pivot);
        let inverse = matrix[column][column]
            .invert()
            .ok_or("zero reconciliation pivot")?;
        for value in &mut matrix[column][column..=size] {
            *value = value.mul(inverse);
        }
        let pivot_row = matrix[column].clone();
        for (row_index, row) in matrix.iter_mut().enumerate() {
            if row_index == column {
                continue;
            }
            let factor = row[column];
            if factor.is_zero() {
                continue;
            }
            for entry in column..=size {
                row[entry] = row[entry].sub(factor.mul(pivot_row[entry]));
            }
        }
    }
    Ok(matrix.into_iter().map(|row| row[size]).collect())
}

fn monic_polynomial(lower_coefficients: &[Fp251V1]) -> Vec<Fp251V1> {
    let mut coefficients = lower_coefficients.to_vec();
    coefficients.push(Fp251V1::ONE);
    coefficients
}

fn evaluate_polynomial(coefficients: &[Fp251V1], point: Fp251V1) -> Fp251V1 {
    coefficients
        .iter()
        .rev()
        .fold(Fp251V1::ZERO, |value, &coefficient| {
            value.mul(point).add(coefficient)
        })
}

fn roots_in_universe(coefficients: &[Fp251V1], universe: u8) -> Vec<u8> {
    (0..universe)
        .filter(|&value| evaluate_polynomial(coefficients, fp(u64::from(value))).is_zero())
        .collect()
}

fn sketch_equation_holds(
    left: &Sketch,
    right: &Sketch,
    numerator: &[Fp251V1],
    denominator: &[Fp251V1],
    points: &[Fp251V1],
) -> bool {
    points.iter().enumerate().all(|(index, &point)| {
        left.evaluations[index].mul(evaluate_polynomial(denominator, point))
            == right.evaluations[index].mul(evaluate_polynomial(numerator, point))
    })
}

fn exact_difference(left: &[u8], right: &[u8]) -> RecoveredDifference {
    RecoveredDifference {
        only_left: left
            .iter()
            .copied()
            .filter(|value| !right.contains(value))
            .collect(),
        only_right: right
            .iter()
            .copied()
            .filter(|value| !left.contains(value))
            .collect(),
    }
}

fn validate_set(values: &[u8], universe: u8) -> Result<(), String> {
    if values.iter().any(|&value| value >= universe)
        || values.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(
            "reconciliation inputs must be sorted unique values inside the universe".into(),
        );
    }
    Ok(())
}

fn mask_set(mask: u64, universe: u8) -> Vec<u8> {
    (0..universe)
        .filter(|bit| mask & (1_u64 << bit) != 0)
        .collect()
}

fn fp(value: u64) -> Fp251V1 {
    Fp251V1::from_u64_mod(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovers_both_sides_and_rejects_bound_violation() {
        let recovered = reconcile(&[1, 2, 7, 9], &[1, 3, 7, 10], 32, 4).unwrap();
        assert_eq!(recovered.only_left, [2, 9]);
        assert_eq!(recovered.only_right, [3, 10]);
        assert!(reconcile(&[0, 1, 2], &[3, 4, 5], 32, 5).is_err());
    }

    #[test]
    fn exhaustive_small_universe_recovers_every_pair_within_bound() {
        for left in 0_u64..256 {
            for right in 0_u64..256 {
                if (left ^ right).count_ones() > 6 {
                    continue;
                }
                let left_set = mask_set(left, 8);
                let right_set = mask_set(right, 8);
                assert_eq!(
                    reconcile(&left_set, &right_set, 32, 6).unwrap(),
                    exact_difference(&left_set, &right_set)
                );
            }
        }
    }
}
