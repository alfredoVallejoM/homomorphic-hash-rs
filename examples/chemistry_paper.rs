use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use homomorphic_hash_rs::domains::chemistry::demo4_thermodynamic::Demo4ThermodynamicLimit;
use homomorphic_hash_rs::harness::runner::BenchmarkRunner;

// =========================================================================
// INDUSTRIAL DATASET MANAGER
// =========================================================================

struct DatasetSource<'a> {
    url: &'a str,
    smiles_col_idx: usize,
    has_header: bool,
}

struct DatasetManager;

impl DatasetManager {
    fn ensure_directories() {
        fs::create_dir_all("data/chemistry").expect("Failed to create base directory");
        fs::create_dir_all("data/chemistry/results").expect("Failed to create results directory");
    }

    fn fetch_and_clean(sources: Vec<DatasetSource>, output_path: &str) {
        if Path::new(output_path).exists() {
            return;
        }
        let mut success = false;

        for source in sources {
            match ureq::get(source.url).call() {
                Ok(res) => {
                    let response_text = res.into_string().unwrap();
                    let mut out_file = File::create(output_path).unwrap();

                    for (i, line) in response_text.lines().enumerate() {
                        if source.has_header && i == 0 {
                            continue;
                        }
                        let parts: Vec<&str> = line.split(',').collect();
                        if parts.len() > source.smiles_col_idx {
                            let smiles = parts[source.smiles_col_idx].trim().trim_matches('"');
                            if !smiles.is_empty() && smiles != "smiles" {
                                writeln!(out_file, "{}", smiles).unwrap();
                            }
                        }
                    }
                    success = true;
                    break;
                }
                Err(_) => continue,
            }
        }
        if !success {
            panic!("CRITICAL: Network mirrors failed for {}", output_path);
        }
    }

    pub fn prepare_all_datasets() -> (String, String, String) {
        Self::ensure_directories();

        // Path definitions directly inside data/chemistry/
        let massive_path = "data/chemistry/massive_industrial_sample.csv";
        let isomers_path = "data/chemistry/pentadecane_isomers.csv";
        let hts_1m_path = "data/chemistry/hts_1m_zinc.csv";

        Self::fetch_and_clean(vec![
            DatasetSource { url: "https://raw.githubusercontent.com/aspuru-guzik-group/chemical_vae/master/models/zinc_properties/250k_rndm_zinc_drugs_clean_3.csv", smiles_col_idx: 0, has_header: true }
        ], massive_path);

        Self::fetch_and_clean(vec![
            DatasetSource { url: "https://pubchem.ncbi.nlm.nih.gov/rest/pug/compound/fastformula/C15H32/property/IsomericSMILES/CSV", smiles_col_idx: 1, has_header: true }
        ], isomers_path);

        // 1 Million Molecule Dataset (ZINC15 Lead-Like Tranche equivalent)
        Self::fetch_and_clean(vec![
            DatasetSource { url: "https://raw.githubusercontent.com/cartst/ZINC15-subsets/master/zinc_1M_sample.csv", smiles_col_idx: 0, has_header: true },
            DatasetSource { url: "https://raw.githubusercontent.com/wengong-jin/icml18-jtnn/master/data/zinc/train.txt", smiles_col_idx: 0, has_header: false }
        ], hts_1m_path);

        (
            massive_path.to_string(),
            isomers_path.to_string(),
            hts_1m_path.to_string(),
        )
    }
}

// =========================================================================
// MAIN PIPELINE LAUNCHER
// =========================================================================
fn main() {
    println!("🧪 INITIALIZING SCIENTIFIC PIPELINE");
    let (_massive_path, _isomers_path, _hts_1m_path) = DatasetManager::prepare_all_datasets();
    let mut runner = BenchmarkRunner::new();
    /*
    // REGISTER DEMO 1 (Invariance Levels 1, 2, 3)
    runner.add_experiment(Box::new(Demo1Level1Positional::new()));
    runner.add_experiment(Box::new(Demo1Level2Symmetry::new()));
    runner.add_experiment(Box::new(Demo1Level3Massive::new(&massive_path, 249455)));

    // REGISTER DEMO 2 (1-WL Defeat Levels 1, 2, 3, 4)
    runner.add_experiment(Box::new(Demo2Level1Alkanes::new()));
    runner.add_experiment(Box::new(Demo2Level2Aromatic::new()));
    runner.add_experiment(Box::new(Demo2Level3WLParadox::new()));
    runner.add_experiment(Box::new(Demo2Level4MassivePubChem::new(&isomers_path)));

    // REGISTER DEMO 3 (Homomorphic HTS Shield)
    println!(">>> REGISTERING DEMO 3: HIGH-THROUGHPUT SCREENING (NP-Complete Shield)");
    runner.add_experiment(Box::new(Demo3HTSShield::new(&hts_1m_path)));
    */
    println!(">>> REGISTERING DEMO 4: THERMODYNAMIC LIMIT (O(V+E) Asymptotics)");
    runner.add_experiment(Box::new(Demo4ThermodynamicLimit::new()));
    // Global log routed directly to the results folder
    runner.ignite("data/chemistry/results/telemetry_global_harness.csv");
    println!("✅ FULL PIPELINE COMPLETE. Check data/chemistry/results/ for specific telemetry.");
}
