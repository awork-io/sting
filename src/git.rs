use std::fmt;
use std::path::Path;

use anyhow::{Context, Result};
use git2::{Delta, DiffOptions, Repository};

#[derive(Debug, Clone, PartialEq)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
    Renamed,
}

impl fmt::Display for ChangeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChangeType::Added => write!(f, "A"),
            ChangeType::Modified => write!(f, "M"),
            ChangeType::Deleted => write!(f, "D"),
            ChangeType::Renamed => write!(f, "R"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChangedFile {
    pub path: String,
    pub change_type: ChangeType,
}

impl ChangedFile {
    pub fn new(path: String, change_type: ChangeType) -> Self {
        Self { path, change_type }
    }
}

pub fn get_changed_files(repo_path: &Path, base_ref: &str) -> Result<Vec<ChangedFile>> {
    let repo = Repository::discover(repo_path).with_context(|| {
        format!(
            "Failed to find git repository at or above '{}'",
            repo_path.display()
        )
    })?;

    let repo_root = repo
        .workdir()
        .ok_or_else(|| anyhow::anyhow!("Repository has no working directory (bare repository)"))?;

    // Resolve the base reference to a commit
    let base_obj = repo.revparse_single(base_ref).with_context(|| {
        format!(
            "Could not resolve git reference '{}'. Ensure it exists.",
            base_ref
        )
    })?;

    let base_commit = base_obj
        .peel_to_commit()
        .with_context(|| format!("Reference '{}' does not point to a commit", base_ref))?;

    let base_tree = base_commit
        .tree()
        .with_context(|| "Failed to get tree from base commit")?;

    let head_ref = repo
        .head()
        .with_context(|| "Failed to get HEAD reference")?;
    let head_commit = head_ref
        .peel_to_commit()
        .with_context(|| "HEAD does not point to a commit")?;

    let head_tree = head_commit
        .tree()
        .with_context(|| "Failed to get tree from HEAD commit")?;

    let mut diff_opts = DiffOptions::new();
    diff_opts.include_untracked(false);

    let diff = repo
        .diff_tree_to_tree(Some(&base_tree), Some(&head_tree), Some(&mut diff_opts))
        .with_context(|| "Failed to compute diff between base and HEAD")?;

    let mut changed_files = Vec::new();

    diff.foreach(
        &mut |delta, _| {
            let change_type = match delta.status() {
                Delta::Added => ChangeType::Added,
                Delta::Deleted => ChangeType::Deleted,
                Delta::Modified => ChangeType::Modified,
                Delta::Renamed => ChangeType::Renamed,
                Delta::Copied => ChangeType::Added,
                _ => return true, // Skip other types
            };

            let file_path = if delta.status() == Delta::Deleted {
                delta.old_file().path()
            } else {
                delta.new_file().path()
            };

            if let Some(path) = file_path {
                let absolute_path = repo_root.join(path);
                let path_str = absolute_path.to_string_lossy().to_string();
                changed_files.push(ChangedFile::new(path_str, change_type));
            }

            true
        },
        None,
        None,
        None,
    )
    .with_context(|| "Failed to iterate over diff")?;

    Ok(changed_files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_change_type_display() {
        assert_eq!(format!("{}", ChangeType::Added), "A");
        assert_eq!(format!("{}", ChangeType::Modified), "M");
        assert_eq!(format!("{}", ChangeType::Deleted), "D");
        assert_eq!(format!("{}", ChangeType::Renamed), "R");
    }

    #[test]
    fn test_changed_file_new() {
        let cf = ChangedFile::new("/path/to/file.ts".to_string(), ChangeType::Modified);
        assert_eq!(cf.path, "/path/to/file.ts");
        assert_eq!(cf.change_type, ChangeType::Modified);
    }
}
