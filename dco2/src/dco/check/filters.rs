//! This module contains some filters for the check output template.

use super::{CommitCheckOutput, CommitError};

/// Template filter to check if any of the commits contain any of the
/// provided errors.
pub(crate) fn contains_error(
    commits_with_errors: &[CommitCheckOutput],
    errors: &[CommitError],
) -> askama::Result<bool> {
    Ok(commits_with_errors.iter().any(|c| c.errors.iter().any(|e| errors.contains(e))))
}
