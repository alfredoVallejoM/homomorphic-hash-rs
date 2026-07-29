use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{PathBuf};
use std::time::Instant;
use flate2::read::GzDecoder;

// =========================================================================
// CHEMISTRY DOMAIN FETCHER: INDUSTRIAL DATA INGESTION ENGINE
// Objective: Automated, stream-based acquisition and decompression of
// massive global biochemical and structural datasets (ZINC, UniProt, ChEMBL).
// All data is strictly siloed within the chemistry domain boundaries.
// =========================================================================

/// Authoritative scientific repositories for topological benchmarking
pub enum DatasetCatalog {
    /// ZINC20 Tranche: Parametrized by 3D properties.
    /// Example tranche: "BAAA" (Standard reactivity, mid-weight). Contains ~100k-1M SMILES.
    ZincTranche(&'static str),

    /// UniProtKB Swiss-Prot: The fully manually annotated and reviewed human/global proteome.
    /// Formatted in FASTA. ~570,000 empirical protein sequences.
    UniProtSwissProt,

    /// ChEMBL: FDA Approved Drugs. Excellent for Zero-Noise Isosteric benchmarking.
    ChemblFDA,

    /// RCSB PDB: Structural dataset index or massive PDB archive mapped for the Matrix Bifurcation.
    ProteinDataBankBulk,
}

impl DatasetCatalog {
    /// Resolves the dataset target to its authoritative, real-world HTTPS endpoint.
    fn resolve_endpoint(&self) -> (&'static str, String, bool) {
        match self {
            DatasetCatalog::ZincTranche(tranche) => {
                // ZINC20 uses a predictable URL structure for 2D SMILES tranches.
                // E.g., http://files.docking.org/2D/BA/BAAA.smi.gz
                let prefix = &tranche[0..2];
                let url = format!("https://files.docking.org/2D/{}/{}.smi.gz", prefix, tranche);
                let filename = format!("zinc_{}.smi", tranche);
                (filename.leak(), url, true) // is_gzipped = true
            },
            DatasetCatalog::UniProtSwissProt => {
                // Official UniProt HTTPS endpoint for the complete reviewed knowledgebase
                let url = "https://ftp.uniprot.org/pub/databases/uniprot/current_release/knowledgebase/complete/uniprot_sprot.fasta.gz".to_string();
                ("uniprot_sprot.fasta", url, true)
            },
            DatasetCatalog::ChemblFDA => {
                // Standard SMILES dump for ChEMBL approved drugs (approx ~3,000 highly curated structures)
                let url = "https://ftp.ebi.ac.uk/pub/databases/chembl/ChEMBLdb/latest/chembl_drugs.smi.gz".to_string();
                ("chembl_fda.smi", url, true)
            },
            DatasetCatalog::ProteinDataBankBulk => {
                // For PDB bulk, we point to the structural resolution index to extract the Contact Maps
                let url = "https://files.rcsb.org/pub/pdb/derived_data/index/resolu.idx".to_string();
                ("pdb_resolutions.idx", url, false)
            }
        }
    }
}

pub struct ChemistryDataFetcher {
    lake_dir: PathBuf,
}

impl ChemistryDataFetcher {
    /// Initializes the fetcher specifically for the chemistry domain data directory.
    pub fn new() -> Self {
        // Enforcing domain-driven design: all empirical data goes to the chemistry module's vault
        let path = PathBuf::from("data/chemistry/datasets");
        if !path.exists() {
            fs::create_dir_all(&path).expect("CRITICAL: Failed to create Chemistry Data directory");
        }
        Self { lake_dir: path }
    }

    /// Fetches, decompresses (if necessary), and saves the dataset to the domain directory.
    /// Returns the absolute path to the ready-to-parse raw file.
    pub fn fetch(&self, target: DatasetCatalog) -> PathBuf {
        let (filename, url, is_gzipped) = target.resolve_endpoint();
        let final_path = self.lake_dir.join(filename);

        // 1. Cache Verification (Avoid re-downloading terabytes of data)
        if final_path.exists() && final_path.metadata().unwrap().len() > 0 {
            println!("    [CACHE] Dataset '{}' already exists in data/chemistry/datasets/. Skipping download.", filename);
            return final_path;
        }

        println!("    [NETWORK] Initiating download for '{}'...", filename);
        println!("    [TRACE] Target Endpoint: {}", url);

        let t0 = Instant::now();

        // 2. Establish Blocking HTTPS Connection
        let response = ureq::get(&url)
            .set("User-Agent", "HomomorphicHash-Research-Node/1.0")
            .call()
            .unwrap_or_else(|err| panic!("CRITICAL: Failed to connect to empirical dataset endpoint: {}", err));

        let content_length = response.header("Content-Length")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);

        println!("    [TRACE] Connection established. Stream size: {} bytes. Commencing Zero-RAM-Spike ingestion...", content_length);

        // 3. Setup Disk Writer mapped to data/chemistry/datasets/
        let file = File::create(&final_path)
            .unwrap_or_else(|err| panic!("CRITICAL: Failed to create file {:?}: {}", final_path, err));
        let mut writer = BufWriter::new(file);

        // 4. Streaming & Decompression Pipeline
        if is_gzipped {
            // Pipe: Network TCP Stream -> GZ Decoder -> Buffered Disk Writer
            let mut decoder = GzDecoder::new(response.into_reader());
            io::copy(&mut decoder, &mut writer)
                .expect("CRITICAL: Streaming decompression failed. Possible corrupted upstream archive.");
        } else {
            // Pipe: Network TCP Stream -> Buffered Disk Writer
            let mut reader = response.into_reader();
            io::copy(&mut reader, &mut writer)
                .expect("CRITICAL: Raw streaming failed.");
        }

        writer.flush().unwrap();
        let elapsed = t0.elapsed().as_secs();
        let final_size_mb = final_path.metadata().unwrap().len() as f64 / (1024.0 * 1024.0);

        println!("    [OK] Ingestion Complete: '{}' saved to data/chemistry/datasets/. Extracted {:.2} MB in {} seconds.", filename, final_size_mb, elapsed);

        final_path
    }
}
