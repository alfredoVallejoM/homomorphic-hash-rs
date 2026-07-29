use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

pub struct CsvTelemetry {
    file: std::fs::File,
}

impl CsvTelemetry {
    pub fn new(experiment_name: &str, headers: &str) -> Self {
        let dir = Path::new("metrology_data");
        if !dir.exists() {
            std::fs::create_dir_all(dir).unwrap();
        }
        let filepath = dir.join(format!("{}.csv", experiment_name));

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(filepath)
            .unwrap();

        writeln!(file, "{}", headers).unwrap();
        Self { file }
    }

    pub fn record(&mut self, n: usize, metric_name: &str, value: f64) {
        writeln!(self.file, "{},{},{:.4}", n, metric_name, value).unwrap();
    }
}
