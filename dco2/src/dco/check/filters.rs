//! This module contains some filters for the check output template.

use super::{CommitCheckOutput, CommitError};

/// Template filter to check if any of the commits contain any of the
/// provided errors.
pub(crate) fn contains_error(commits: &[CommitCheckOutput], errors: &[CommitError]) -> askama::Result<bool> {
    Ok(commits.iter().any(|c| c.errors.iter().any(|e| errors.contains(e))))
}

#[cfg(test)]
mod tests {
    use crate::dco::check::{filters::contains_error, CommitCheckOutput, CommitError};

    #[test]
    fn contains_error_one_commit_error_found() {
        let commits = vec![CommitCheckOutput {
            commit: Default::default(),
            errors: vec![CommitError::InvalidAuthorEmail],
            success_reason: None,
        }];

        assert!(contains_error(&commits, &[CommitError::InvalidAuthorEmail]).unwrap());
    }

    #[test]
    fn contains_error_one_commit_error_not_found() {
        let commits = vec![CommitCheckOutput {
            commit: Default::default(),
            errors: vec![CommitError::InvalidAuthorEmail],
            success_reason: None,
        }];

        assert!(!contains_error(&commits, &[CommitError::InvalidCommitterEmail]).unwrap());
    }

    #[test]
    fn contains_error_two_commits_error_found() {
        let commits = vec![
            CommitCheckOutput {
                commit: Default::default(),
                errors: vec![CommitError::InvalidAuthorEmail],
                success_reason: None,
            },
            CommitCheckOutput {
                commit: Default::default(),
                errors: vec![CommitError::InvalidCommitterEmail],
                success_reason: None,
            },
        ];

        assert!(contains_error(
            &commits,
            &[CommitError::InvalidAuthorEmail, CommitError::SignOffNotFound]
        )
        .unwrap());
    }

    #[test]
    fn contains_error_two_commits_error_not_found() {
        let commits = vec![
            CommitCheckOutput {
                commit: Default::default(),
                errors: vec![CommitError::InvalidAuthorEmail],
                success_reason: None,
            },
            CommitCheckOutput {
                commit: Default::default(),
                errors: vec![CommitError::InvalidCommitterEmail],
                success_reason: None,
            },
        ];

        assert!(!contains_error(
            &commits,
            &[CommitError::SignOffNotFound, CommitError::SignOffMismatch]
        )
        .unwrap());
    }
}
