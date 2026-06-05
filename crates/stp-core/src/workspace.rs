use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::ids::WorkspaceId;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceMetadata {
    pub workspace_id: WorkspaceId,
    pub workspace_path: PathBuf,
    pub repo_root: PathBuf,
    pub branch_name: Option<String>,
    pub is_git: bool,
}

#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("workspace path does not exist: {0}")]
    MissingPath(PathBuf),
    #[error("failed to canonicalize workspace path {path}: {source}")]
    Canonicalize {
        path: PathBuf,
        source: std::io::Error,
    },
}

pub fn discover_workspace(path: &Path) -> Result<WorkspaceMetadata, WorkspaceError> {
    let workspace_path = canonical_workspace(path)?;
    let repo_root = git_repo_root(&workspace_path);
    let branch_name =
        git_line(&workspace_path, ["branch", "--show-current"]).filter(|branch| !branch.is_empty());
    let is_git = repo_root.is_some();
    let repo_root = repo_root.unwrap_or_else(|| workspace_path.clone());

    Ok(WorkspaceMetadata {
        workspace_id: workspace_id_for_path(&workspace_path),
        workspace_path,
        repo_root,
        branch_name,
        is_git,
    })
}

pub fn canonical_workspace(path: &Path) -> Result<PathBuf, WorkspaceError> {
    if !path.exists() {
        return Err(WorkspaceError::MissingPath(path.to_path_buf()));
    }
    path.canonicalize()
        .map_err(|source| WorkspaceError::Canonicalize {
            path: path.to_path_buf(),
            source,
        })
}

pub fn workspace_id_for_path(path: &Path) -> WorkspaceId {
    let mut hasher = Sha256::new();
    hasher.update(path.display().to_string().as_bytes());
    let digest = hasher.finalize();
    WorkspaceId::new(hex::encode(&digest[..16]))
}

fn git_line<const N: usize>(cwd: &Path, args: [&str; N]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?;
    Some(value.trim().to_owned())
}

fn git_repo_root(workspace_path: &Path) -> Option<PathBuf> {
    let common_dir = git_line(
        workspace_path,
        ["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    let common_path = PathBuf::from(common_dir);
    let repo_root = if common_path.file_name().and_then(|name| name.to_str()) == Some(".git") {
        common_path.parent()?.to_path_buf()
    } else {
        PathBuf::from(git_line(workspace_path, ["rev-parse", "--show-toplevel"])?)
    };
    repo_root.canonicalize().ok()
}
