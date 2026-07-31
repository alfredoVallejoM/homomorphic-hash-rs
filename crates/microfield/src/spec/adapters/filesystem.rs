//! Transactional filesystem artifact publication.

use std::{
    collections::BTreeSet,
    fmt,
    fs::{self, File},
    io::Write as _,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::spec::{
    model::GeneratedArtifacts,
    ports::{ArtifactSink, Publication},
};

/// Filesystem adapter failure with operation and path context.
#[derive(Debug)]
pub struct FileSystemError {
    operation: &'static str,
    path: PathBuf,
    source: std::io::Error,
}

impl fmt::Display for FileSystemError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} `{}` failed: {}",
            self.operation,
            self.path.display(),
            self.source
        )
    }
}

impl std::error::Error for FileSystemError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// Unit-of-work adapter that stages complete outputs before an atomic rename.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileSystemArtifactSink {
    root: PathBuf,
}

impl FileSystemArtifactSink {
    /// Creates a sink rooted at the supplied artifact directory.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Returns the publication root.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl ArtifactSink for FileSystemArtifactSink {
    type Error = FileSystemError;

    fn publish(&self, artifacts: &GeneratedArtifacts) -> Result<Publication, Self::Error> {
        io("create publication root", &self.root, || {
            fs::create_dir_all(&self.root)
        })?;

        let target = self.root.join(artifacts.field_name());
        let replaced_existing = existing_directory(&target)?;
        let staging = create_unique_directory(&self.root, artifacts.field_name(), "staging")?;
        if let Err(error) = write_staging(&staging, artifacts) {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }

        if !replaced_existing {
            if let Err(error) = fs::rename(&staging, &target) {
                let _ = fs::remove_dir_all(&staging);
                return Err(FileSystemError {
                    operation: "commit staged artifacts",
                    path: target,
                    source: error,
                });
            }
            return Ok(Publication::new(target, false));
        }

        let backup = unique_path(&self.root, artifacts.field_name(), "backup");
        if let Err(source) = fs::rename(&target, &backup) {
            let _ = fs::remove_dir_all(&staging);
            return Err(FileSystemError {
                operation: "move previous publication to backup",
                path: target,
                source,
            });
        }
        if let Err(source) = fs::rename(&staging, &target) {
            let restore = fs::rename(&backup, &target);
            let _ = fs::remove_dir_all(&staging);
            return match restore {
                Ok(()) => Err(FileSystemError {
                    operation: "commit staged artifacts",
                    path: target,
                    source,
                }),
                Err(restore_error) => Err(FileSystemError {
                    operation: "restore previous publication after commit failure",
                    path: backup,
                    source: restore_error,
                }),
            };
        }
        io("remove committed backup", &backup, || {
            fs::remove_dir_all(&backup)
        })?;
        Ok(Publication::new(target, true))
    }

    fn matches(&self, artifacts: &GeneratedArtifacts) -> Result<bool, Self::Error> {
        let target = self.root.join(artifacts.field_name());
        if !target.is_dir() {
            return Ok(false);
        }

        let actual = collect_entries(&target, &target)?;
        let expected = expected_entries(artifacts);
        if actual != expected {
            return Ok(false);
        }
        for file in artifacts.files() {
            let path = target.join(file.relative_path());
            let contents = io("read committed artifact", &path, || fs::read(&path))?;
            if contents != file.contents() {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

fn write_staging(staging: &Path, artifacts: &GeneratedArtifacts) -> Result<(), FileSystemError> {
    for generated in artifacts.files() {
        let path = staging.join(generated.relative_path());
        if let Some(parent) = path.parent() {
            io("create staged directory", parent, || {
                fs::create_dir_all(parent)
            })?;
        }
        let mut file = io("create staged artifact", &path, || File::create(&path))?;
        io("write staged artifact", &path, || {
            file.write_all(generated.contents())
        })?;
        io("synchronize staged artifact", &path, || file.sync_all())?;
    }
    Ok(())
}

fn collect_entries(root: &Path, directory: &Path) -> Result<BTreeSet<String>, FileSystemError> {
    let mut result = BTreeSet::new();
    let entries = io("read publication directory", directory, || {
        fs::read_dir(directory)
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| FileSystemError {
            operation: "read publication entry",
            path: directory.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let file_type = io("inspect publication entry", &path, || entry.file_type())?;
        if file_type.is_dir() {
            let relative = relative_path(root, &path);
            result.insert(format!("{relative}/"));
            result.extend(collect_entries(root, &path)?);
        } else if file_type.is_file() {
            result.insert(relative_path(root, &path));
        } else {
            return Err(FileSystemError {
                operation: "reject special publication entry",
                path,
                source: std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "symlinks and special files are not valid artifacts",
                ),
            });
        }
    }
    Ok(result)
}

fn expected_entries(artifacts: &GeneratedArtifacts) -> BTreeSet<String> {
    let mut entries = BTreeSet::new();
    for file in artifacts.files() {
        entries.insert(file.relative_path().to_owned());
        let mut parent = Path::new(file.relative_path()).parent();
        while let Some(directory) = parent {
            if directory.as_os_str().is_empty() {
                break;
            }
            entries.insert(format!(
                "{}/",
                directory.to_string_lossy().replace('\\', "/")
            ));
            parent = directory.parent();
        }
    }
    entries
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("publication entry is below its traversal root")
        .to_string_lossy()
        .replace('\\', "/")
}

fn existing_directory(target: &Path) -> Result<bool, FileSystemError> {
    match fs::symlink_metadata(target) {
        Ok(metadata) if metadata.file_type().is_dir() => Ok(true),
        Ok(_) => Err(FileSystemError {
            operation: "validate existing publication",
            path: target.to_path_buf(),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "existing publication must be a real directory",
            ),
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(FileSystemError {
            operation: "inspect existing publication",
            path: target.to_path_buf(),
            source,
        }),
    }
}

fn create_unique_directory(
    root: &Path,
    field_name: &str,
    purpose: &str,
) -> Result<PathBuf, FileSystemError> {
    for attempt in 0..100 {
        let path = unique_path_with_attempt(root, field_name, purpose, attempt);
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(source) => {
                return Err(FileSystemError {
                    operation: "create staging directory",
                    path,
                    source,
                });
            }
        }
    }
    Err(FileSystemError {
        operation: "create unique staging directory",
        path: root.to_path_buf(),
        source: std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "exhausted unique staging names",
        ),
    })
}

fn unique_path(root: &Path, field_name: &str, purpose: &str) -> PathBuf {
    unique_path_with_attempt(root, field_name, purpose, 0)
}

fn unique_path_with_attempt(
    root: &Path,
    field_name: &str,
    purpose: &str,
    attempt: usize,
) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    root.join(format!(
        ".{field_name}.{purpose}-{}-{timestamp}-{attempt}",
        std::process::id()
    ))
}

fn io<T>(
    operation: &'static str,
    path: &Path,
    action: impl FnOnce() -> std::io::Result<T>,
) -> Result<T, FileSystemError> {
    action().map_err(|source| FileSystemError {
        operation,
        path: path.to_path_buf(),
        source,
    })
}
