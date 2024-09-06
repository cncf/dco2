//! This module contains some filters for the check output template.

use super::{CommitCheckOutput, CommitError};

/// Template filter to check if any of the commits contain any of the
/// provided errors.
pub(crate) fn contains_error(commits: &[CommitCheckOutput], errors: &[CommitError]) -> askama::Result<bool> {
    Ok(commits.iter().any(|c| c.errors.iter().any(|e| errors.contains(e))))
}

/// Template filter to truncate a string to the specified length without adding
/// dots at the end.
pub(crate) fn truncate_no_dots(s: String, length: usize) -> askama::Result<String> {
    Ok(s.chars().take(length).collect::<String>())
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

    #[test]
    fn truncate_no_dots_works() {
        assert_eq!(
            "Hello".to_string(),
            super::truncate_no_dots("Hello".to_string(), 10).unwrap()
        );
        assert_eq!(
            "Hello".to_string(),
            super::truncate_no_dots("Hello, World!".to_string(), 5).unwrap()
        );
    }
}
