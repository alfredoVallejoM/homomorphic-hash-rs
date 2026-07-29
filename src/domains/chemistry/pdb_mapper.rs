use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::collections::HashMap;
use std::time::Instant;

use crate::algebra::galois_256::GaloisSignature256;
use crate::topology::symmetric_difference::SymmetricDifferenceAggregator as SymDiff;
use crate::domains::chemistry::smiles_parser::MolecularComplex;

// =========================================================================
// PDB MAPPER: 3D SPATIAL HASHING & TOPOLOGY INFERENCE
// Objective: Parse raw crystallographic data, sanitize biological noise
// (water, AltLocs), and use an O(N) Cell-List algorithm to infer the
// exact 3D empirical adjacency matrix (including Disulfide Bridges).
// =========================================================================

struct Atom3D {
    id: usize,
    element: String,
    x: f32,
    y: f32,
    z: f32,
}

pub struct PdbGraphMapper;

impl PdbGraphMapper {
    /// O(N) Spatial Hashing Algorithm to infer covalent bonds based on van der Waals radii
    fn infer_covalent_bonds_spatial_hash(atoms: &[Atom3D], cutoff: f32) -> Vec<(usize, usize)> {
        let cell_size = cutoff;
        let mut grid: HashMap<(i32, i32, i32), Vec<usize>> = HashMap::new();

        // 1. Map atoms to spatial buckets
        for (i, atom) in atoms.iter().enumerate() {
            let cx = (atom.x / cell_size).floor() as i32;
            let cy = (atom.y / cell_size).floor() as i32;
            let cz = (atom.z / cell_size).floor() as i32;
            grid.entry((cx, cy, cz)).or_insert_with(Vec::new).push(i);
        }

        let mut edges = Vec::new();
        let cutoff_sq = cutoff * cutoff;

        // 2. Local neighborhood search (O(N) amortized)
        for (i, atom) in atoms.iter().enumerate() {
            let cx = (atom.x / cell_size).floor() as i32;
            let cy = (atom.y / cell_size).floor() as i32;
            let cz = (atom.z / cell_size).floor() as i32;

            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        if let Some(neighbors) = grid.get(&(cx + dx, cy + dy, cz + dz)) {
                            for &j in neighbors {
                                if i < j { // Prevent duplicate edges
                                    let neighbor = &atoms[j];
                                    let dist_sq = (atom.x - neighbor.x).powi(2) +
                                                  (atom.y - neighbor.y).powi(2) +
                                                  (atom.z - neighbor.z).powi(2);

                                    if dist_sq <= cutoff_sq {
                                        edges.push((i, j));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        edges
    }

    /// Parses a raw PDB file, sanitizes it, and constructs a MolecularComplex
    pub fn parse_pdb_to_complex<P: AsRef<Path>>(filepath: P) -> Option<MolecularComplex> {
        let file = File::open(filepath).ok()?;
        let reader = BufReader::new(file);

        let mut atoms = Vec::new();
        let mut seeds = Vec::new();
        let mut explicit_edges = Vec::new();

        // Used to map PDB Serial Numbers to our continuous 0..N internal index
        let mut serial_to_internal = HashMap::new();
        let mut internal_idx = 0;

        for line_result in reader.lines() {
            let line = line_result.ok()?;

            // 1. Sanitize and Extract ATOM / HETATM
            if line.starts_with("ATOM") || line.starts_with("HETATM") {
                let res_name = line.get(17..20).unwrap_or("   ").trim();
                let alt_loc = line.chars().nth(16).unwrap_or(' ');

                // DATA SANITIZATION: Ignore Water and Alternate Quantum Locations
                if res_name == "HOH" || (alt_loc != ' ' && alt_loc != 'A') {
                    continue;
                }

                let serial: usize = line.get(6..11)?.trim().parse().ok()?;
                let element = line.get(76..78).unwrap_or(" C").trim().to_string();

                let x: f32 = line.get(30..38)?.trim().parse().ok()?;
                let y: f32 = line.get(38..46)?.trim().parse().ok()?;
                let z: f32 = line.get(46..54)?.trim().parse().ok()?;

                atoms.push(Atom3D { id: serial, element: element.clone(), x, y, z });
                serial_to_internal.insert(serial, internal_idx);

                // Initialize Galois State
                seeds.push(SymDiff::embed_to_field(element.as_bytes()));
                internal_idx += 1;
            }

            // 2. Extract Explicit Connectivity (If provided)
            if line.starts_with("CONECT") {
                let tokens: Vec<&str> = line.split_whitespace().collect();
                if tokens.len() >= 2 {
                    if let Ok(u_serial) = tokens[1].parse::<usize>() {
                        for v_token in &tokens[2..] {
                            if let Ok(v_serial) = v_token.parse::<usize>() {
                                explicit_edges.push((u_serial, v_serial));
                            }
                        }
                    }
                }
            }
        }

        if atoms.is_empty() { return None; }

        let t0 = Instant::now();
        // 3. Compute Structural Density (Covalent bonds cutoff: ~1.9Å, Disulfide ~2.1Å)
        // We set cutoff to 2.2Å to capture all valid organic cross-links.
        let mut inferred_edges = Self::infer_covalent_bonds_spatial_hash(&atoms, 2.2);

        // 4. Merge Explicit and Inferred edges
        for (u, v) in explicit_edges {
            if let (Some(&i), Some(&j)) = (serial_to_internal.get(&u), serial_to_internal.get(&v)) {
                if i < j { inferred_edges.push((i, j)); }
                else { inferred_edges.push((j, i)); }
            }
        }

        // Deduplicate edges
        inferred_edges.sort_unstable();
        inferred_edges.dedup();

        let var_count = atoms.len();
        let mut clauses = Vec::with_capacity(inferred_edges.len());
        let mut var_to_clauses = vec![vec![]; var_count];

        for (c_idx, (u, v)) in inferred_edges.into_iter().enumerate() {
            clauses.push(vec![u, v]);
            var_to_clauses[u].push(c_idx);
            var_to_clauses[v].push(c_idx);
        }

        println!("    [TRACE] PDB 3D Spatial Hashing complete. V={}, E={} (Computed in {} ms)", var_count, clauses.len(), t0.elapsed().as_millis());

        Some(MolecularComplex {
            var_count,
            clauses,
            var_to_clauses,
            seeds,
        })
    }
}
