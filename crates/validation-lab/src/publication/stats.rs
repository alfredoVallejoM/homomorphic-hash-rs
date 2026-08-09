use super::model::ConfidenceInterval;

#[derive(Clone, Debug)]
pub struct SplitMix64(u64);

impl SplitMix64 {
    pub const fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    pub fn index(&mut self, upper: usize) -> usize {
        debug_assert!(upper > 0);
        (self.next_u64() % upper as u64) as usize
    }

    pub fn shuffle<T>(&mut self, values: &mut [T]) {
        for upper in (1..values.len()).rev() {
            let index = self.index(upper + 1);
            values.swap(upper, index);
        }
    }
}

pub fn derive_seed(root: u64, label: &str, index: usize) -> u64 {
    let hash = label
        .as_bytes()
        .iter()
        .fold(0xcbf2_9ce4_8422_2325, |state, byte| {
            (state ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
        });
    let mut rng = SplitMix64::new(root ^ hash ^ index as u64);
    rng.next_u64()
}

pub fn quantile(values: &[f64], probability: f64) -> f64 {
    assert!(!values.is_empty());
    assert!((0.0..=1.0).contains(&probability));
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    if sorted.len() == 1 {
        return sorted[0];
    }
    let position = (sorted.len() - 1) as f64 * probability;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    let fraction = position - lower as f64;
    sorted[lower] + (sorted[upper] - sorted[lower]) * fraction
}

pub fn median(values: &[f64]) -> f64 {
    quantile(values, 0.5)
}

pub fn mad(values: &[f64]) -> f64 {
    let center = median(values);
    let deviations = values
        .iter()
        .map(|value| (value - center).abs())
        .collect::<Vec<_>>();
    median(&deviations)
}

pub fn bootstrap_quantile_ci(
    clusters: &[Vec<f64>],
    probability: f64,
    resamples: usize,
    seed: u64,
) -> ConfidenceInterval {
    assert!(!clusters.is_empty());
    assert!(clusters.iter().all(|cluster| !cluster.is_empty()));
    assert!(resamples > 0);
    let mut rng = SplitMix64::new(seed);
    let total = clusters.iter().map(Vec::len).sum();
    let mut estimates = Vec::with_capacity(resamples);
    let mut sample = Vec::with_capacity(total);
    for _ in 0..resamples {
        sample.clear();
        for _ in 0..clusters.len() {
            let cluster = &clusters[rng.index(clusters.len())];
            for _ in 0..cluster.len() {
                sample.push(cluster[rng.index(cluster.len())]);
            }
        }
        estimates.push(quantile(&sample, probability));
    }
    ConfidenceInterval {
        confidence_level: 0.95,
        lower: quantile(&estimates, 0.025),
        upper: quantile(&estimates, 0.975),
    }
}

pub fn bootstrap_statistic_ci(
    values: &[f64],
    resamples: usize,
    seed: u64,
    statistic: impl Fn(&[f64]) -> f64,
) -> ConfidenceInterval {
    assert!(!values.is_empty());
    let mut rng = SplitMix64::new(seed);
    let mut sample = vec![0.0; values.len()];
    let mut estimates = Vec::with_capacity(resamples);
    for _ in 0..resamples {
        for value in &mut sample {
            *value = values[rng.index(values.len())];
        }
        estimates.push(statistic(&sample));
    }
    ConfidenceInterval {
        confidence_level: 0.95,
        lower: quantile(&estimates, 0.025),
        upper: quantile(&estimates, 0.975),
    }
}

pub fn relative_half_width(center: f64, interval: ConfidenceInterval) -> f64 {
    if center == 0.0 {
        return f64::INFINITY;
    }
    ((interval.upper - interval.lower) / 2.0) / center.abs()
}

pub fn log_log_slope(points: &[(f64, f64)]) -> Option<f64> {
    if points.len() < 2
        || points
            .iter()
            .any(|(x, y)| !x.is_finite() || !y.is_finite() || *x <= 0.0 || *y <= 0.0)
    {
        return None;
    }
    let transformed = points
        .iter()
        .map(|(x, y)| (x.ln(), y.ln()))
        .collect::<Vec<_>>();
    let mean_x = transformed.iter().map(|(x, _)| x).sum::<f64>() / transformed.len() as f64;
    let mean_y = transformed.iter().map(|(_, y)| y).sum::<f64>() / transformed.len() as f64;
    let numerator = transformed
        .iter()
        .map(|(x, y)| (x - mean_x) * (y - mean_y))
        .sum::<f64>();
    let denominator = transformed
        .iter()
        .map(|(x, _)| (x - mean_x).powi(2))
        .sum::<f64>();
    (denominator > 0.0).then_some(numerator / denominator)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantile_and_mad_cover_even_and_odd_samples() {
        assert_eq!(median(&[1.0, 3.0, 2.0]), 2.0);
        assert_eq!(median(&[1.0, 2.0, 3.0, 4.0]), 2.5);
        assert_eq!(mad(&[1.0, 2.0, 3.0, 100.0]), 1.0);
    }

    #[test]
    fn bootstrap_is_deterministic_and_contains_constant() {
        let clusters = vec![vec![7.0; 5]; 4];
        let left = bootstrap_quantile_ci(&clusters, 0.95, 200, 91);
        let right = bootstrap_quantile_ci(&clusters, 0.95, 200, 91);
        assert_eq!(left.lower, 7.0);
        assert_eq!(left.upper, 7.0);
        assert_eq!(left.lower, right.lower);
        assert_eq!(left.upper, right.upper);
    }

    #[test]
    fn shuffle_and_seed_are_reproducible() {
        let mut left = (0..16).collect::<Vec<_>>();
        let mut right = left.clone();
        SplitMix64::new(derive_seed(7, "cell", 3)).shuffle(&mut left);
        SplitMix64::new(derive_seed(7, "cell", 3)).shuffle(&mut right);
        assert_eq!(left, right);
        assert_ne!(left, (0..16).collect::<Vec<_>>());
    }

    #[test]
    fn slope_recovers_linear_and_quadratic_curves() {
        assert!(
            (log_log_slope(&[(1.0, 2.0), (2.0, 4.0), (4.0, 8.0)]).unwrap() - 1.0).abs() < 1e-12
        );
        assert!(
            (log_log_slope(&[(1.0, 1.0), (2.0, 4.0), (4.0, 16.0)]).unwrap() - 2.0).abs() < 1e-12
        );
    }
}
