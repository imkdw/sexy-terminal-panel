use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use thiserror::Error;

use super::model::Registry;
use crate::workspace::WorkspaceError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryStore {
    path: PathBuf,
}

impl RegistryStore {
    pub const fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<Registry, RegistryError> {
        if !self.path.exists() {
            return Ok(Registry::default());
        }
        let raw = fs::read_to_string(&self.path).map_err(|source| RegistryError::Read {
            path: self.path.clone(),
            source,
        })?;
        serde_json::from_str(&raw).map_err(|source| RegistryError::MalformedRegistry {
            path: self.path.clone(),
            source,
        })
    }

    pub fn save(&self, registry: &Registry) -> Result<(), RegistryError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|source| RegistryError::Write {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let temp_path = self
            .path
            .with_extension(format!("tmp-{}", std::process::id()));
        let write_result = write_atomic(&temp_path, &self.path, registry);
        if write_result.is_err() {
            let _ = fs::remove_file(&temp_path);
        }
        write_result
    }
}

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error(transparent)]
    Workspace(#[from] WorkspaceError),
    #[error("failed to read registry {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to write registry {path}: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("malformed registry {path}: {source}")]
    MalformedRegistry {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("failed to encode registry: {0}")]
    Encode(serde_json::Error),
}

fn write_atomic(
    temp_path: &Path,
    final_path: &Path,
    registry: &Registry,
) -> Result<(), RegistryError> {
    let encoded = serde_json::to_vec_pretty(registry).map_err(RegistryError::Encode)?;
    let mut file = File::create(temp_path).map_err(|source| RegistryError::Write {
        path: temp_path.to_path_buf(),
        source,
    })?;
    file.write_all(&encoded)
        .map_err(|source| RegistryError::Write {
            path: temp_path.to_path_buf(),
            source,
        })?;
    file.sync_all().map_err(|source| RegistryError::Write {
        path: temp_path.to_path_buf(),
        source,
    })?;
    fs::rename(temp_path, final_path).map_err(|source| RegistryError::Write {
        path: final_path.to_path_buf(),
        source,
    })
}
