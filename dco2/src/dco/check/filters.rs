//! This module contains some filters for the check output template.

use super::{CommitCheckOutput, CommitError};

/// Template filter to check if any of the commits contain any of the
/// provided errors.
#[askama::filter_fn]
pub(crate) fn contains_error(
    commits: &[CommitCheckOutput],
    _: &dyn askama::Values,
    errors: &[CommitError],
) -> askama::Result<bool> {
    Ok(commits.iter().any(|c| c.errors.iter().any(|e| errors.contains(e))))
}

/// Template filter to truncate a string to the specified length without adding
/// dots at the end.
#[askama::filter_fn]
pub(crate) fn truncate_no_dots(s: String, _: &dyn askama::Values, length: usize) -> askama::Result<String> {
    Ok(s.chars().take(length).collect::<String>())
}

#[cfg(test)]
mod tests {
    use crate::dco::check::{
        CommitCheckOutput, CommitError,
        filters::{contains_error, truncate_no_dots},
    };

    #[test]
    fn contains_error_one_commit_error_found() {
        let commits = vec![CommitCheckOutput {
            commit: Default::default(),
            errors: vec![CommitError::InvalidAuthorEmail],
            success_reason: None,
        }];

        assert!(
            contains_error::default()
                .with_errors(&[CommitError::InvalidAuthorEmail])
                .execute(&commits, askama::NO_VALUES)
                .unwrap()
        );
    }

    #[test]
    fn contains_error_one_commit_error_not_found() {
        let commits = vec![CommitCheckOutput {
            commit: Default::default(),
            errors: vec![CommitError::InvalidAuthorEmail],
            success_reason: None,
        }];

        assert!(
            !contains_error::default()
                .with_errors(&[CommitError::InvalidCommitterEmail])
                .execute(&commits, askama::NO_VALUES)
                .unwrap()
        );
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

        assert!(
            contains_error::default()
                .with_errors(&[CommitError::InvalidAuthorEmail, CommitError::SignOffNotFound,])
                .execute(&commits, askama::NO_VALUES)
                .unwrap()
        );
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

        assert!(
            !contains_error::default()
                .with_errors(&[CommitError::SignOffNotFound, CommitError::SignOffMismatch])
                .execute(&commits, askama::NO_VALUES)
                .unwrap()
        );
    }

    #[test]
    fn truncate_no_dots_works() {
        assert_eq!(
            "Hello".to_string(),
            truncate_no_dots::default()
                .with_length(10)
                .execute("Hello".to_string(), askama::NO_VALUES)
                .unwrap()
        );
        assert_eq!(
            "Hello".to_string(),
            truncate_no_dots::default()
                .with_length(5)
                .execute("Hello, World!".to_string(), askama::NO_VALUES)
                .unwrap()
        );
    }
}
