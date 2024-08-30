use std::vec;

use super::{check, CheckInput};
use crate::{
    dco::{CommitCheckOutput, CommitError},
    github::{Commit, GitUser},
};
use indoc::indoc;

#[test]
fn check_no_signoff_single_commit_is_merge_commit() {
    let commit1 = Commit {
        is_merge: true,
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1],
    };
    let output = check(&input);

    assert!(output.commits_with_errors.is_empty());
}

#[test]
fn check_no_signoff_single_commit_author_is_bot() {
    let commit1 = Commit {
        author: Some(GitUser {
            is_bot: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1],
    };
    let output = check(&input);

    assert!(output.commits_with_errors.is_empty());
}

#[test]
fn check_valid_signoff_single_commit_author_match() {
    let commit1 = Commit {
        author: Some(GitUser {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1],
    };
    let output = check(&input);

    assert!(output.commits_with_errors.is_empty());
}

#[test]
fn check_valid_signoff_single_commit_committer_match() {
    let commit1 = Commit {
        committer: Some(GitUser {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1],
    };
    let output = check(&input);

    assert!(output.commits_with_errors.is_empty());
}

#[test]
fn check_valid_signoff_single_commit_multiple_signoffs() {
    let commit1 = Commit {
        author: Some(GitUser {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            Signed-off-by: userx <userx@email.test>
            Signed-off-by: user1 <user1@email.test>
            Signed-off-by: usery <usery@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1],
    };
    let output = check(&input);

    assert!(output.commits_with_errors.is_empty());
}

#[test]
fn check_valid_signoff_single_commit_signoff_case_insensitive() {
    let commit1 = Commit {
        author: Some(GitUser {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            signed-off-by: USER1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1],
    };
    let output = check(&input);

    assert!(output.commits_with_errors.is_empty());
}

#[test]
fn check_valid_signoff_single_commit_signoff_trailing_whitespace() {
    let commit1 = Commit {
        author: Some(GitUser {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test\n\nSigned-off-by: user1 <user1@email.test>   ".to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1],
    };
    let output = check(&input);

    assert!(output.commits_with_errors.is_empty());
}

#[test]
fn check_valid_signoff_single_commit_email_contains_subdomain() {
    let commit1 = Commit {
        author: Some(GitUser {
            name: "user1".to_string(),
            email: "user1@email.some.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1 <user1@email.some.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1],
    };
    let output = check(&input);

    assert!(output.commits_with_errors.is_empty());
}

#[test]
fn check_valid_signoff_single_commit_email_contains_plus_alias() {
    let commit1 = Commit {
        author: Some(GitUser {
            name: "user1".to_string(),
            email: "user1+alias@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1 <user1+alias@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1],
    };
    let output = check(&input);

    assert!(output.commits_with_errors.is_empty());
}

#[test]
fn check_invalid_author_email_single_commit() {
    let commit1 = Commit {
        author: Some(GitUser {
            name: "user1".to_string(),
            email: "invalid".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1 <invalid>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone()],
    };
    let output = check(&input);

    assert_eq!(
        output.commits_with_errors,
        vec![CommitCheckOutput {
            commit: commit1,
            errors: vec![CommitError::InvalidAuthorEmail],
        }]
    );
}

#[test]
fn check_invalid_author_email_and_no_signoff_single_commit() {
    let commit1 = Commit {
        author: Some(GitUser {
            name: "user1".to_string(),
            email: "invalid".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone()],
    };
    let output = check(&input);

    assert_eq!(
        output.commits_with_errors,
        vec![CommitCheckOutput {
            commit: commit1,
            errors: vec![CommitError::InvalidAuthorEmail, CommitError::SignOffNotFound],
        }]
    );
}

#[test]
fn check_invalid_committer_email_single_commit() {
    let commit1 = Commit {
        committer: Some(GitUser {
            name: "user1".to_string(),
            email: "invalid".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1 <invalid>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone()],
    };
    let output = check(&input);

    assert_eq!(
        output.commits_with_errors,
        vec![CommitCheckOutput {
            commit: commit1,
            errors: vec![CommitError::InvalidCommitterEmail],
        }]
    );
}

#[test]
fn check_invalid_author_and_committer_email_single_commit_same_email() {
    let commit1 = Commit {
        author: Some(GitUser {
            name: "user1".to_string(),
            email: "invalid".to_string(),
            ..Default::default()
        }),
        committer: Some(GitUser {
            name: "user1".to_string(),
            email: "invalid".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone()],
    };
    let output = check(&input);

    assert_eq!(
        output.commits_with_errors,
        vec![CommitCheckOutput {
            commit: commit1,
            errors: vec![CommitError::InvalidCommitterEmail],
        }]
    );
}

#[test]
fn check_invalid_author_and_committer_email_single_commit_different_emails() {
    let commit1 = Commit {
        author: Some(GitUser {
            name: "user1".to_string(),
            email: "invalid".to_string(),
            ..Default::default()
        }),
        committer: Some(GitUser {
            name: "user1".to_string(),
            email: "invalid2".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone()],
    };
    let output = check(&input);

    assert_eq!(
        output.commits_with_errors,
        vec![CommitCheckOutput {
            commit: commit1,
            errors: vec![
                CommitError::InvalidCommitterEmail,
                CommitError::InvalidAuthorEmail
            ],
        }]
    );
}

#[test]
fn check_signoff_not_found_single_commit() {
    let commit1 = Commit {
        author: Some(GitUser {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(GitUser {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone()],
    };
    let output = check(&input);

    assert_eq!(
        output.commits_with_errors,
        vec![CommitCheckOutput {
            commit: commit1,
            errors: vec![CommitError::SignOffNotFound],
        }]
    );
}
