//! Filesystem transaction and command-line behavior contracts.

#![cfg(feature = "generator")]

use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

use microfield::spec::{FileSystemArtifactSink, Generator, error::PipelineError};

#[test]
fn artifact_check_detects_extra_files_and_empty_directories() {
    let temporary = TemporaryDirectory::new("microfield-extra-entry");
    let sink = FileSystemArtifactSink::new(temporary.path());
    let generator = Generator::default();
    let manifest = manifest_path();
    let publication = generator
        .emit(&manifest, &sink)
        .expect("publication succeeds");

    let extra_file = publication.output_directory().join("extra.bin");
    fs::write(&extra_file, b"extra").expect("extra file is writable");
    assert!(!generator.check(&manifest, &sink).expect("check succeeds"));
    fs::remove_file(extra_file).expect("extra file removable");

    let empty_directory = publication.output_directory().join("empty");
    fs::create_dir(&empty_directory).expect("extra directory is writable");
    assert!(!generator.check(&manifest, &sink).expect("check succeeds"));
    fs::remove_dir(empty_directory).expect("extra directory removable");
    assert!(generator.check(&manifest, &sink).expect("clean check"));
}

#[test]
fn existing_non_directory_publication_is_preserved_on_failure() {
    let temporary = TemporaryDirectory::new("microfield-existing-file");
    let target = temporary.path().join("gf2_256_hh_v1");
    fs::write(&target, b"do-not-replace").expect("target fixture writable");
    let sink = FileSystemArtifactSink::new(temporary.path());

    assert!(matches!(
        Generator::default().emit(manifest_path(), &sink),
        Err(PipelineError::Adapter(_))
    ));
    assert_eq!(
        fs::read(&target).expect("target remains readable"),
        b"do-not-replace"
    );
    assert_eq!(directory_names(temporary.path()), ["gf2_256_hh_v1"]);
}

#[test]
fn publication_root_that_is_a_file_fails_without_modification() {
    let temporary = TemporaryDirectory::new("microfield-root-file");
    let root = temporary.path().join("root");
    fs::write(&root, b"root-sentinel").expect("root fixture writable");
    let sink = FileSystemArtifactSink::new(&root);

    assert!(Generator::default().emit(manifest_path(), &sink).is_err());
    assert_eq!(
        fs::read(root).expect("root remains readable"),
        b"root-sentinel"
    );
}

#[cfg(unix)]
#[test]
fn artifact_check_rejects_symlinks_in_committed_output() {
    use std::os::unix::fs::symlink;

    let temporary = TemporaryDirectory::new("microfield-symlink");
    let sink = FileSystemArtifactSink::new(temporary.path());
    let generator = Generator::default();
    let manifest = manifest_path();
    let publication = generator
        .emit(&manifest, &sink)
        .expect("publication succeeds");
    symlink(
        publication.output_directory().join("metadata.json"),
        publication.output_directory().join("metadata-link.json"),
    )
    .expect("symlink fixture created");

    assert!(matches!(
        generator.check(&manifest, &sink),
        Err(PipelineError::Adapter(_))
    ));
}

#[test]
fn successful_replacement_leaves_no_staging_or_backup_directories() {
    let temporary = TemporaryDirectory::new("microfield-clean-transaction");
    let sink = FileSystemArtifactSink::new(temporary.path());
    let generator = Generator::default();
    let manifest = manifest_path();

    generator.emit(&manifest, &sink).expect("first emission");
    generator
        .emit(&manifest, &sink)
        .expect("replacement emission");

    assert_eq!(directory_names(temporary.path()), ["gf2_256_hh_v1"]);
}

#[test]
fn cli_validate_normalize_and_certify_have_stable_success_contracts() {
    let validate = run_cli([OsStr::new("validate"), manifest_argument()]);
    assert!(validate.status.success());
    assert_eq!(
        String::from_utf8(validate.stdout)
            .expect("UTF-8 stdout")
            .trim(),
        "valid gf2_256_hh_v1 6b62fea68b968fd4f8c39a4f69b78f714c80858b1d0f667ec5a63d4417b43ca8"
    );
    assert!(validate.stderr.is_empty());

    let normalize = run_cli([
        OsStr::new("normalize"),
        manifest_argument(),
        OsStr::new("--json"),
    ]);
    assert!(normalize.status.success());
    let descriptor: serde_json::Value =
        serde_json::from_slice(&normalize.stdout).expect("normalize emits JSON");
    assert_eq!(descriptor["schema"], 1);
    assert_eq!(descriptor["degree"], 256);
    assert_eq!(descriptor["modulus"], serde_json::json!([256, 10, 5, 2, 0]));

    let certify = run_cli([OsStr::new("certify"), manifest_argument()]);
    assert!(certify.status.success());
    let certificate: serde_json::Value =
        serde_json::from_slice(&certify.stdout).expect("certify emits JSON");
    assert_eq!(certificate["validator"], "microfield-rabin-v1");
}

#[test]
fn cli_emit_and_check_use_distinct_exit_codes_for_drift() {
    let temporary = TemporaryDirectory::new("microfield-cli-output");
    let output = temporary.path().as_os_str();
    let emit = run_cli([
        OsStr::new("emit"),
        manifest_argument(),
        OsStr::new("--out"),
        output,
        OsStr::new("--json"),
    ]);
    assert!(emit.status.success());
    let publication: serde_json::Value = serde_json::from_slice(&emit.stdout).expect("emit JSON");
    assert_eq!(publication["ok"], true);
    assert_eq!(publication["replaced_existing"], false);

    let check_arguments = [
        OsStr::new("check"),
        manifest_argument(),
        OsStr::new("--out"),
        output,
        OsStr::new("--json"),
    ];
    let clean = run_cli(check_arguments);
    assert!(clean.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&clean.stdout).expect("check JSON")["matches"],
        true
    );

    fs::write(
        temporary.path().join("gf2_256_hh_v1").join("metadata.json"),
        b"drift\n",
    )
    .expect("test drift writable");
    let drift = run_cli(check_arguments);
    assert_eq!(drift.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&drift.stdout).expect("drift JSON")["matches"],
        false
    );
}

#[test]
fn cli_vectors_can_publish_validated_json_to_a_file() {
    let temporary = TemporaryDirectory::new("microfield-cli-vectors");
    let output_path = temporary.path().join("vectors.json");
    let committed_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("reference-vectors")
        .join("gf2_256_hh_v1.json");
    let output = run_cli([
        OsStr::new("vectors"),
        manifest_argument(),
        OsStr::new("--oracle-json"),
        committed_path.as_os_str(),
        OsStr::new("--out"),
        output_path.as_os_str(),
    ]);

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout)
            .expect("UTF-8 stdout")
            .trim(),
        output_path.display().to_string()
    );
    assert!(output.stderr.is_empty());
    assert_eq!(
        fs::read(&output_path).expect("published vectors readable"),
        fs::read(committed_path).expect("committed vectors readable")
    );
}

#[test]
fn cli_usage_and_pipeline_errors_use_exit_two_and_json_stderr() {
    let unknown = run_cli(["unknown", "--json"]);
    assert_eq!(unknown.status.code(), Some(2));
    assert!(unknown.stdout.is_empty());
    let error: serde_json::Value =
        serde_json::from_slice(&unknown.stderr).expect("JSON error envelope");
    assert_eq!(error["ok"], false);
    assert!(
        error["error"]
            .as_str()
            .expect("error is text")
            .contains("unknown command")
    );

    let missing_output = run_cli([OsStr::new("emit"), manifest_argument()]);
    assert_eq!(missing_output.status.code(), Some(2));
    assert!(
        String::from_utf8(missing_output.stderr)
            .expect("UTF-8 stderr")
            .contains("missing required option")
    );
}

fn run_cli<I, S>(arguments: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new(env!("CARGO_BIN_EXE_microfield-gen"))
        .args(arguments)
        .output()
        .expect("CLI process starts")
}

fn manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fields")
        .join("gf2_256_hh_v1.toml")
}

fn manifest_argument() -> &'static OsStr {
    OsStr::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fields/gf2_256_hh_v1.toml"
    ))
}

fn directory_names(path: &Path) -> Vec<String> {
    let mut names = fs::read_dir(path)
        .expect("directory readable")
        .map(|entry| {
            entry
                .expect("entry readable")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    names.sort_unstable();
    names
}

struct TemporaryDirectory(PathBuf);

impl TemporaryDirectory {
    fn new(prefix: &str) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("{prefix}-{}-{timestamp}", std::process::id()));
        fs::create_dir(&path).expect("unique temporary directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
