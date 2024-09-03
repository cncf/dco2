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

/// Template filter to check if all the commits contain the same errors.
pub(crate) fn have_same_errors(commits_with_errors: &[CommitCheckOutput]) -> askama::Result<bool> {
    for i in 1..commits_with_errors.len() {
        if commits_with_errors[i].errors != commits_with_errors[0].errors {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use crate::dco::check::{
        filters::{contains_error, have_same_errors},
        CommitCheckOutput, CommitError,
    };

    #[test]
    fn contains_error_one_commit_error_found() {
        let commits_with_errors = vec![CommitCheckOutput {
            commit: Default::default(),
            errors: vec![CommitError::InvalidAuthorEmail],
        }];

        assert!(contains_error(&commits_with_errors, &[CommitError::InvalidAuthorEmail]).unwrap());
    }

    #[test]
    fn contains_error_one_commit_error_not_found() {
        let commits_with_errors = vec![CommitCheckOutput {
            commit: Default::default(),
            errors: vec![CommitError::InvalidAuthorEmail],
        }];

        assert!(!contains_error(&commits_with_errors, &[CommitError::InvalidCommitterEmail]).unwrap());
    }

    #[test]
    fn contains_error_two_commits_error_found() {
        let commits_with_errors = vec![
            CommitCheckOutput {
                commit: Default::default(),
                errors: vec![CommitError::InvalidAuthorEmail],
            },
            CommitCheckOutput {
                commit: Default::default(),
                errors: vec![CommitError::InvalidCommitterEmail],
            },
        ];

        assert!(contains_error(
            &commits_with_errors,
            &[CommitError::InvalidAuthorEmail, CommitError::SignOffNotFound]
        )
        .unwrap());
    }

    #[test]
    fn contains_error_two_commits_error_not_found() {
        let commits_with_errors = vec![
            CommitCheckOutput {
                commit: Default::default(),
                errors: vec![CommitError::InvalidAuthorEmail],
            },
            CommitCheckOutput {
                commit: Default::default(),
                errors: vec![CommitError::InvalidCommitterEmail],
            },
        ];

        assert!(!contains_error(
            &commits_with_errors,
            &[CommitError::SignOffNotFound, CommitError::SignOffMismatch]
        )
        .unwrap());
    }

    #[test]
    fn have_same_errors_one_commit_always_true() {
        let commits_with_errors = vec![CommitCheckOutput {
            commit: Default::default(),
            errors: vec![CommitError::InvalidAuthorEmail],
        }];

        assert!(have_same_errors(&commits_with_errors).unwrap());
    }

    #[test]
    fn have_same_errors_two_commits_one_error_matches() {
        let commits_with_errors = vec![
            CommitCheckOutput {
                commit: Default::default(),
                errors: vec![CommitError::InvalidAuthorEmail],
            },
            CommitCheckOutput {
                commit: Default::default(),
                errors: vec![CommitError::InvalidAuthorEmail],
            },
        ];

        assert!(have_same_errors(&commits_with_errors).unwrap());
    }

    #[test]
    fn have_same_errors_two_commits_two_errors_match() {
        let commits_with_errors = vec![
            CommitCheckOutput {
                commit: Default::default(),
                errors: vec![CommitError::InvalidAuthorEmail, CommitError::SignOffNotFound],
            },
            CommitCheckOutput {
                commit: Default::default(),
                errors: vec![CommitError::InvalidAuthorEmail, CommitError::SignOffNotFound],
            },
        ];

        assert!(have_same_errors(&commits_with_errors).unwrap());
    }

    #[test]
    fn have_same_errors_two_commits_two_errors_do_not_match() {
        let commits_with_errors = vec![
            CommitCheckOutput {
                commit: Default::default(),
                errors: vec![
                    CommitError::InvalidAuthorEmail,
                    CommitError::InvalidCommitterEmail,
                ],
            },
            CommitCheckOutput {
                commit: Default::default(),
                errors: vec![CommitError::InvalidAuthorEmail, CommitError::SignOffNotFound],
            },
        ];

        assert!(!have_same_errors(&commits_with_errors).unwrap());
    }

    #[test]
    fn have_same_errors_three_commits_two_errors_match() {
        let commits_with_errors = vec![
            CommitCheckOutput {
                commit: Default::default(),
                errors: vec![CommitError::InvalidAuthorEmail, CommitError::SignOffNotFound],
            },
            CommitCheckOutput {
                commit: Default::default(),
                errors: vec![CommitError::InvalidAuthorEmail, CommitError::SignOffNotFound],
            },
            CommitCheckOutput {
                commit: Default::default(),
                errors: vec![CommitError::InvalidAuthorEmail, CommitError::SignOffNotFound],
            },
        ];

        assert!(have_same_errors(&commits_with_errors).unwrap());
    }

    #[test]
    fn have_same_errors_three_commits_one_error_does_not_match() {
        let commits_with_errors = vec![
            CommitCheckOutput {
                commit: Default::default(),
                errors: vec![CommitError::InvalidAuthorEmail],
            },
            CommitCheckOutput {
                commit: Default::default(),
                errors: vec![CommitError::SignOffNotFound],
            },
            CommitCheckOutput {
                commit: Default::default(),
                errors: vec![CommitError::InvalidAuthorEmail],
            },
        ];

        assert!(!have_same_errors(&commits_with_errors).unwrap());
    }
}
