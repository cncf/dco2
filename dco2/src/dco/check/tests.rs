use crate::{
    dco::check::{check, CheckInput, CheckOutput, CommitCheckOutput, CommitError},
    github::{Commit, GitUser},
};
use indoc::indoc;
use std::vec;

#[test]
fn single_commit_no_signoff_is_merge_commit() {
    let commit1 = Commit {
        is_merge: true,
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_no_signoff_author_is_bot() {
    let commit1 = Commit {
        author: Some(GitUser {
            is_bot: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_valid_signoff_author_match() {
    let commit1 = Commit {
        author: Some(GitUser {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(GitUser {
            name: "user2".to_string(),
            email: "user2@email.test".to_string(),
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
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_valid_signoff_committer_match() {
    let commit1 = Commit {
        author: Some(GitUser {
            name: "user2".to_string(),
            email: "user2@email.test".to_string(),
            ..Default::default()
        }),
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
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_valid_signoff_multiple_signoffs() {
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
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_valid_signoff_signoff_case_insensitive() {
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
        message: indoc! {r"
            Test commit message

            signed-off-by: USER1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_valid_signoff_signoff_trailing_whitespace() {
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
        message: "Test\n\nSigned-off-by: user1 <user1@email.test>   ".to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_valid_signoff_email_contains_subdomain() {
    let commit1 = Commit {
        author: Some(GitUser {
            name: "user1".to_string(),
            email: "user1@email.some.test".to_string(),
            ..Default::default()
        }),
        committer: Some(GitUser {
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
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_valid_signoff_email_contains_plus_alias() {
    let commit1 = Commit {
        author: Some(GitUser {
            name: "user1".to_string(),
            email: "user1+alias@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(GitUser {
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
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_invalid_author_email() {
    let commit1 = Commit {
        author: Some(GitUser {
            name: "user1".to_string(),
            email: "invalid".to_string(),
            ..Default::default()
        }),
        committer: Some(GitUser {
            name: "user2".to_string(),
            email: "user2@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            Signed-off-by: user2 <user2@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::InvalidAuthorEmail],
            }],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_invalid_author_email_and_no_signoff() {
    let commit1 = Commit {
        author: Some(GitUser {
            name: "user1".to_string(),
            email: "invalid".to_string(),
            ..Default::default()
        }),
        committer: Some(GitUser {
            name: "user2".to_string(),
            email: "user2@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::InvalidAuthorEmail, CommitError::SignOffNotFound],
            }],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_invalid_author_email_also_used_in_signoff() {
    let commit1 = Commit {
        author: Some(GitUser {
            name: "user1".to_string(),
            email: "invalid".to_string(),
            ..Default::default()
        }),
        committer: Some(GitUser {
            name: "user2".to_string(),
            email: "user2@email.test".to_string(),
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
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::InvalidAuthorEmail],
            }],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_invalid_committer_email() {
    let commit1 = Commit {
        author: Some(GitUser {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(GitUser {
            name: "user2".to_string(),
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
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::InvalidCommitterEmail],
            }],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_invalid_committer_email_and_no_signoff() {
    let commit1 = Commit {
        author: Some(GitUser {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(GitUser {
            name: "user2".to_string(),
            email: "invalid".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::InvalidCommitterEmail, CommitError::SignOffNotFound],
            }],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_invalid_committer_email_also_used_in_signoff() {
    let commit1 = Commit {
        author: Some(GitUser {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(GitUser {
            name: "user2".to_string(),
            email: "invalid".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            Signed-off-by: user2 <invalid>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::InvalidCommitterEmail],
            }],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_invalid_author_and_committer_email_same_email() {
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
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::InvalidCommitterEmail],
            }],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_invalid_author_and_committer_email_different_emails() {
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
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![
                    CommitError::InvalidCommitterEmail,
                    CommitError::InvalidAuthorEmail
                ],
            }],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_signoff_not_found() {
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
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffNotFound],
            }],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_invalid_signoff_multiple_signoffs() {
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
        message: indoc! {r"
            Test commit message

            Signed-off-by: userx <userx@email.test>
            Signed-off-by: usery <usery@email.test>
            Signed-off-by: userz <userz@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffMismatch],
            }],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_invalid_signoff_name_mismatch() {
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
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1x <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffMismatch],
            }],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_invalid_signoff_email_mismatch() {
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
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1 <user1x@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffMismatch],
            }],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_invalid_signoff_name_and_email_mismatch() {
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
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1x <user1x@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffMismatch],
            }],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_invalid_signoff_extra_whitespace_around_name() {
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
        message: indoc! {r"
            Test commit message

            Signed-off-by:  user1  <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffMismatch],
            }],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_invalid_signoff_extra_whitespace_around_email() {
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
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1 < user1@email.test >
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffMismatch],
            }],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_invalid_signoff_missing_name_in_signoff() {
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
        message: indoc! {r"
            Test commit message

            Signed-off-by: <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffNotFound],
            }],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_invalid_signoff_missing_email_in_signoff() {
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
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffNotFound],
            }],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_invalid_signoff_missing_email_brackets_in_signoff() {
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
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1 user1@email.test
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffNotFound],
            }],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_invalid_signoff_missing_name_and_email_in_signoff() {
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
        message: indoc! {r"
            Test commit message

            Signed-off-by:
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffNotFound],
            }],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_invalid_signoff_name_and_email_swapped_in_signoff() {
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
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1@email.test <user1>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffMismatch],
            }],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_invalid_signoff_invalid_email_in_signoff() {
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
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1 <user1(at)email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffMismatch],
            }],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn single_commit_invalid_signoff_email_alias_used_in_signoff_but_not_in_author_email() {
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
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1 <user1+alias@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffMismatch],
            }],
            head_ref: "main".to_string(),
            total_commits: 1,
        }
    );
}

#[test]
fn two_commits_valid_signoff_in_both() {
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
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };
    let commit2 = commit1.clone();

    let input = CheckInput {
        commits: vec![commit1, commit2],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![],
            head_ref: "main".to_string(),
            total_commits: 2,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_valid_signoff_in_second() {
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
    let commit2 = Commit {
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
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone(), commit2],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffNotFound],
            }],
            head_ref: "main".to_string(),
            total_commits: 2,
        }
    );
}

#[test]
fn two_commits_valid_signoff_in_first_no_signoff_in_second() {
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
        message: indoc! {r"
        Test commit message

        Signed-off-by: user1 <user1@email.test>
    "}
        .to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
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
        commits: vec![commit1, commit2.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![CommitCheckOutput {
                commit: commit2,
                errors: vec![CommitError::SignOffNotFound],
            }],
            head_ref: "main".to_string(),
            total_commits: 2,
        }
    );
}

#[test]
fn two_commits_invalid_signoff_in_first_valid_signoff_in_second() {
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
        message: indoc! {r"
            Test commit message

            Signed-off-by: userx <userx@email.test>
        "}
        .to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
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
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone(), commit2],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffMismatch],
            }],
            head_ref: "main".to_string(),
            total_commits: 2,
        }
    );
}

#[test]
fn two_commits_valid_signoff_in_first_invalid_signoff_in_second() {
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
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
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
        message: indoc! {r"
            Test commit message

            Signed-off-by: userx <userx@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1, commit2.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![CommitCheckOutput {
                commit: commit2,
                errors: vec![CommitError::SignOffMismatch],
            }],
            head_ref: "main".to_string(),
            total_commits: 2,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_invalid_signoff_in_second() {
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
    let commit2 = Commit {
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
        message: indoc! {r"
            Test commit message

            Signed-off-by: userx <userx@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![CommitError::SignOffNotFound],
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![CommitError::SignOffMismatch],
                }
            ],
            head_ref: "main".to_string(),
            total_commits: 2,
        }
    );
}

#[test]
fn two_commits_invalid_signoff_in_first_no_signoff_in_second() {
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
        message: indoc! {r"
        Test commit message

        Signed-off-by: userx <userx@email.test>
    "}
        .to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
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
        commits: vec![commit1.clone(), commit2.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![CommitError::SignOffMismatch],
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![CommitError::SignOffNotFound],
                }
            ],
            head_ref: "main".to_string(),
            total_commits: 2,
        }
    );
}

#[test]
fn three_commits_valid_signoff_in_all() {
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
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };
    let commit2 = commit1.clone();
    let commit3 = commit1.clone();

    let input = CheckInput {
        commits: vec![commit1, commit2, commit3],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![],
            head_ref: "main".to_string(),
            total_commits: 3,
        }
    );
}

#[test]
fn three_commits_valid_signoff_first_and_second_no_signoff_third() {
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
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };
    let commit2 = commit1.clone();
    let commit3 = Commit {
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
        commits: vec![commit1, commit2, commit3.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![CommitCheckOutput {
                commit: commit3,
                errors: vec![CommitError::SignOffNotFound],
            }],
            head_ref: "main".to_string(),
            total_commits: 3,
        }
    );
}

#[test]
fn three_commits_invalid_signoff_first_no_signoff_second_valid_signoff_third() {
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
        message: indoc! {r"
            Test commit message

            Signed-off-by: userx <userx@email.test>
        "}
        .to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
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
    let commit3 = Commit {
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
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone(), commit3],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![CommitError::SignOffMismatch],
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![CommitError::SignOffNotFound],
                }
            ],
            head_ref: "main".to_string(),
            total_commits: 3,
        }
    );
}

#[test]
fn three_commits_valid_signoff_first_invalid_signoff_second_valid_signoff_third() {
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
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
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
        message: indoc! {r"
            Test commit message

            Signed-off-by: userx <userx@email.test>
        "}
        .to_string(),
        ..Default::default()
    };
    let commit3 = commit1.clone();

    let input = CheckInput {
        commits: vec![commit1, commit2.clone(), commit3],
        config: Default::default(),
        head_ref: "main".to_string(),
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits_with_errors: vec![CommitCheckOutput {
                commit: commit2,
                errors: vec![CommitError::SignOffMismatch],
            }],
            head_ref: "main".to_string(),
            total_commits: 3,
        }
    );
}
