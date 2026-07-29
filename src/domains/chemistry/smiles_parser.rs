use crate::algebra::galois_256::GaloisSignature256;
use crate::algebra::traits::FiniteField;
use crate::topology::traits::HomomorphicAggregator;
use crate::topology::symmetric_difference::SymmetricDifferenceAggregator as SymDiff;
use crate::engine::canonizer::TopologyProvider;
use crate::harness::mapper::DomainMapper;

use purr::graph::Builder;
use purr::feature::{AtomKind, BondKind};
use purr::read::read;

use std::sync::atomic::{AtomicU64, Ordering};

static PERMUTATION_SEED_COUNTER: AtomicU64 = AtomicU64::new(1);

struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self { state: if seed == 0 { 1 } else { seed } }
    }

    fn next_usize(&mut self, bound: usize) -> usize {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        (x % (bound as u64)) as usize
    }
}

fn deterministic_shuffle<T>(slice: &mut [T], rng: &mut DeterministicRng) {
    let len = slice.len();
    if len < 2 { return; }
    for i in (1..len).rev() {
        let j = rng.next_usize(i + 1);
        slice.swap(i, j);
    }
}

#[derive(Clone)]
pub struct MolecularComplex {
    pub var_count: usize,
    pub clauses: Vec<Vec<usize>>,
    pub var_to_clauses: Vec<Vec<usize>>,
    pub seeds: Vec<GaloisSignature256>,
}

impl MolecularComplex {
    pub fn new(var_count: usize, seeds: Vec<GaloisSignature256>) -> Self {
        Self {
            var_count,
            clauses: Vec::new(),
            var_to_clauses: vec![vec![]; var_count],
            seeds,
        }
    }

    pub fn add_hyperedge(&mut self, vars: &[usize]) {
        let c_idx = self.clauses.len();
        self.clauses.push(vars.to_vec());
        for &v in vars {
            if v < self.var_count {
                self.var_to_clauses[v].push(c_idx);
            }
        }
    }

    pub fn generate_isomorphic_permutation(&self) -> Self {
        let seed = PERMUTATION_SEED_COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut rng = DeterministicRng::new(seed);

        let mut var_map: Vec<usize> = (0..self.var_count).collect();
        deterministic_shuffle(&mut var_map, &mut rng);

        let mut new_seeds = vec![GaloisSignature256::zero(); self.var_count];
        for i in 0..self.var_count {
            new_seeds[var_map[i]] = self.seeds[i];
        }

        let mut new_clauses: Vec<Vec<usize>> = self.clauses.iter().map(|clause| {
            clause.iter().map(|&v| var_map[v]).collect()
        }).collect();

        deterministic_shuffle(&mut new_clauses, &mut rng);

        let mut new_var_to_clauses = vec![vec![]; self.var_count];
        for (c_idx, vars) in new_clauses.iter().enumerate() {
            for &v in vars {
                new_var_to_clauses[v].push(c_idx);
            }
        }

        Self {
            var_count: self.var_count,
            clauses: new_clauses,
            var_to_clauses: new_var_to_clauses,
            seeds: new_seeds,
        }
    }
}

impl TopologyProvider for MolecularComplex {
    fn num_variables(&self) -> usize { self.var_count }
    fn num_clauses(&self) -> usize { self.clauses.len() }
    fn variables_in_clause(&self, idx: usize) -> Vec<usize> { self.clauses[idx].clone() }
    fn clauses_for_variable(&self, idx: usize) -> Vec<usize> { self.var_to_clauses[idx].clone() }

    fn initial_state(&self, variable_index: usize) -> Option<GaloisSignature256> {
        if variable_index < self.seeds.len() { Some(self.seeds[variable_index]) } else { None }
    }
}

pub struct SmilesParser;

impl SmilesParser {
    /// Safe parsing method. Returns None if the SMILES string is mathematically or syntactically corrupt.
    pub fn try_parse_to_complex(raw_smiles: &str) -> Option<MolecularComplex> {
        let smiles = raw_smiles.trim().trim_matches(|c| c == '\u{feff}' || c == '"' || c == '\'');
        if smiles.is_empty() { return None; }

        let mut builder = Builder::new();
        if read(smiles, &mut builder, None).is_err() { return None; }

        // Graceful handling: If the atom graph cannot be built (e.g. broken ring closures), abort safely.
        let atoms = match builder.build() {
            Ok(a) => a,
            Err(_) => return None,
        };

        let mut seed_states: Vec<GaloisSignature256> = Vec::with_capacity(atoms.len());

        for atom in &atoms {
            let symbol = match atom.kind {
                AtomKind::Aliphatic(ref el) => format!("{:?}", el),
                AtomKind::Aromatic(ref el) => format!("{:?}", el),
                AtomKind::Bracket { ref symbol, .. } => format!("{:?}", symbol),
                AtomKind::Star => "*".to_string(),
            };
            seed_states.push(SymDiff::embed_to_field(symbol.as_bytes()));
        }

        let mut complex = MolecularComplex::new(seed_states.len(), seed_states.clone());

        for (from_idx, atom) in atoms.iter().enumerate() {
            for bond in &atom.bonds {
                let to_idx = bond.tid;
                if from_idx < to_idx {
                    let multiplicity = match bond.kind {
                        BondKind::Double => 2,
                        BondKind::Triple => 3,
                        _ => 1,
                    };
                    for _ in 0..multiplicity {
                        complex.add_hyperedge(&[from_idx, to_idx]);
                    }
                }
            }
        }
        Some(complex)
    }

    /// Legacy method for hardcoded tests where syntax is guaranteed to be 100% correct.
    pub fn parse_to_complex(raw_smiles: &str) -> MolecularComplex {
        Self::try_parse_to_complex(raw_smiles).expect("CRITICAL: Hardcoded valid SMILES failed to parse.")
    }
}

impl DomainMapper for SmilesParser {
    type RawInput = String;
    fn map_to_topology(smiles: &Self::RawInput) -> (Box<dyn TopologyProvider + Send + Sync>, Vec<GaloisSignature256>) {
        let complex = Self::parse_to_complex(smiles);
        let seeds = complex.seeds.clone();
        (Box::new(complex), seeds)
    }
}
