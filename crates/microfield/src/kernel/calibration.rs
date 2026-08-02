//! Versioned, compile-time backend selection decisions.
//!
//! These values are deliberately separate from CPU feature detection. A
//! backend can be safe and available while remaining ineligible for automatic
//! selection until representative measurements justify a threshold.

/// Version of the maintained selection table.
pub(crate) const SELECTION_TABLE_VERSION: u32 = 1;

const _: () = assert!(SELECTION_TABLE_VERSION == 1);

/// Immutable calibration decision compiled into kernel metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SelectionCalibration {
    minimum_batch: usize,
    automatic_selection: bool,
}

impl SelectionCalibration {
    #[cfg(any(target_arch = "x86_64", test))]
    const fn automatic(minimum_batch: usize) -> Self {
        Self {
            minimum_batch,
            automatic_selection: true,
        }
    }

    #[cfg(any(feature = "builtin-fields", target_arch = "aarch64", test))]
    const fn explicit_only(candidate_minimum_batch: usize) -> Self {
        Self {
            minimum_batch: candidate_minimum_batch,
            automatic_selection: false,
        }
    }

    pub(crate) const fn minimum_batch(self) -> usize {
        self.minimum_batch
    }

    pub(crate) const fn automatic_selection(self) -> bool {
        self.automatic_selection
    }
}

/// PCLMUL is conservatively faster from one element for every maintained
/// field on the calibrated x86-64 baseline.
#[cfg(any(all(feature = "builtin-fields", target_arch = "x86_64"), test))]
pub(crate) const X86_PCLMUL: SelectionCalibration = SelectionCalibration::automatic(1);

/// VPCLMUL has a local candidate crossover at 64 elements for GF(2^128), but
/// remains explicit until at least two x86-64 CPU families confirm it.
#[cfg(any(all(feature = "builtin-fields", target_arch = "x86_64"), test))]
pub(crate) const X86_VPCLMUL_128: SelectionCalibration = SelectionCalibration::explicit_only(64);

/// No favorable VPCLMUL region was observed for either maintained GF(2^256)
/// field. `usize::MAX` is metadata, never an execution precondition.
#[cfg(any(all(feature = "builtin-fields", target_arch = "x86_64"), test))]
pub(crate) const X86_VPCLMUL_256: SelectionCalibration =
    SelectionCalibration::explicit_only(usize::MAX);

/// Goldilocks AVX2 clears the conservative 20% gate for multiplication and
/// square at four elements without degrading addition by more than 3%.
#[cfg(any(all(feature = "prime-fields", target_arch = "x86_64"), test))]
pub(crate) const X86_PRIME_AVX2_GOLDILOCKS: SelectionCalibration =
    SelectionCalibration::automatic(4);

/// PMULL is functionally certified on native `AArch64` hardware but stays
/// explicit until reproducible performance profiles cover two CPU families.
#[cfg(any(target_arch = "aarch64", test))]
pub(crate) const AARCH64_PMULL: SelectionCalibration = SelectionCalibration::explicit_only(1);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_one_selection_contract_matches_the_committed_table() {
        assert_eq!(SELECTION_TABLE_VERSION, 1);
        assert_eq!(X86_PCLMUL, SelectionCalibration::automatic(1));
        assert_eq!(X86_VPCLMUL_128, SelectionCalibration::explicit_only(64));
        assert_eq!(
            X86_VPCLMUL_256,
            SelectionCalibration::explicit_only(usize::MAX)
        );
        assert_eq!(AARCH64_PMULL, SelectionCalibration::explicit_only(1));

        let table = include_str!("../../calibration/selection-table-v1.csv");
        assert!(table.starts_with("selection_table_version,field,backend,"));
        assert_eq!(table.lines().skip(1).count(), 9);
        assert!(table.contains("1,gf2_128_v1,x86_pclmul,1,true"));
        assert!(table.contains("1,gf2_128_v1,x86_vpclmul,64,false"));
        assert!(table.contains("1,gf2_256_hh_v1,x86_vpclmul,none,false"));
        assert!(table.contains("1,gf2_256_alt_v1,x86_vpclmul,none,false"));
        assert_eq!(table.matches(",aarch64_pmull,1,false,").count(), 3);
    }

    #[test]
    fn phase46_goldilocks_evidence_matches_the_compiled_threshold() {
        assert_eq!(
            X86_PRIME_AVX2_GOLDILOCKS,
            SelectionCalibration::automatic(4)
        );

        let evidence = include_str!("../../calibration/phase46-simd-i7-13700hx-2026-08-02.csv");
        let mut arithmetic_rows = 0;
        let mut add_rows = 0;
        for line in evidence.lines().skip(1) {
            let mut parts = line.split(',');
            let columns: [&str; 10] = core::array::from_fn(|_| parts.next().unwrap());
            assert!(parts.next().is_none());
            assert_eq!(columns[0], "1");
            assert_eq!(columns[1], "fp_goldilocks64_v1");
            assert_eq!(columns[2], "x86_prime_avx2");
            assert_eq!(columns[4], "4");

            let portable_lower: f64 = columns[5].parse().unwrap();
            let portable_upper: f64 = columns[6].parse().unwrap();
            let simd_lower: f64 = columns[7].parse().unwrap();
            let simd_upper: f64 = columns[8].parse().unwrap();
            assert!(portable_lower > 0.0 && portable_upper >= portable_lower);
            assert!(simd_lower > 0.0 && simd_upper >= simd_lower);

            match columns[3] {
                "mul" | "square" => {
                    let conservative_gain = 100.0 * (portable_lower - simd_upper) / portable_lower;
                    assert!(conservative_gain >= 20.0);
                    arithmetic_rows += 1;
                }
                "add" => {
                    let conservative_regression =
                        100.0 * (simd_upper - portable_lower) / portable_lower;
                    assert!(conservative_regression <= 3.0);
                    add_rows += 1;
                }
                operation => panic!("unexpected phase 4.6 operation: {operation}"),
            }
        }
        assert_eq!(arithmetic_rows, 2);
        assert_eq!(add_rows, 1);
    }
}
