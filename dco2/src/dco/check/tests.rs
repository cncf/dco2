use crate::{
    dco::check::{check, CheckInput, CheckOutput, CommitCheckOutput, CommitError, CommitSuccessReason},
    github::{Commit, Config, ConfigAllowRemediationCommits, ConfigRequire, User},
};
use indoc::indoc;
use pretty_assertions::assert_eq;
use std::vec;

#[test]
fn single_commit_no_signoff_is_merge_commit() {
    let commit1 = Commit {
        is_merge: true,
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![],
                success_reason: Some(CommitSuccessReason::IsMerge),
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn single_commit_no_signoff_author_is_bot() {
    let commit1 = Commit {
        author: Some(User {
            is_bot: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![],
                success_reason: Some(CommitSuccessReason::FromBot),
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn single_commit_no_signoff_author_is_member() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            login: Some("user1".to_string()),
            ..Default::default()
        }),
        verified: Some(true),
        ..Default::default()
    };

    let config = Config {
        require: Some(ConfigRequire { members: Some(false) }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec!["user1".to_string()],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![],
                success_reason: Some(CommitSuccessReason::FromMember),
            }],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn single_commit_no_signoff_committer_is_member() {
    let commit1 = Commit {
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            login: Some("user1".to_string()),
            ..Default::default()
        }),
        verified: Some(true),
        ..Default::default()
    };

    let config = Config {
        require: Some(ConfigRequire { members: Some(false) }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec!["user1".to_string()],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![],
                success_reason: Some(CommitSuccessReason::FromMember),
            }],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn single_commit_no_signoff_author_is_member_but_members_are_required_to_signoff() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            login: Some("user1".to_string()),
            ..Default::default()
        }),
        verified: Some(true),
        ..Default::default()
    };

    let config = Config {
        require: Some(ConfigRequire { members: Some(true) }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec!["user1".to_string()],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffNotFound],
                success_reason: None,
            }],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn single_commit_no_signoff_committer_is_member_but_members_are_required_to_signoff() {
    let commit1 = Commit {
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            login: Some("user1".to_string()),
            ..Default::default()
        }),
        verified: Some(true),
        ..Default::default()
    };

    let config = Config {
        require: Some(ConfigRequire { members: Some(true) }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec!["user1".to_string()],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffNotFound],
                success_reason: None,
            }],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn single_commit_no_signoff_author_is_member_but_the_commit_is_not_verified() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            login: Some("user1".to_string()),
            ..Default::default()
        }),
        verified: Some(false),
        ..Default::default()
    };

    let config = Config {
        require: Some(ConfigRequire { members: Some(false) }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec!["user1".to_string()],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffNotFound],
                success_reason: None,
            }],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn single_commit_no_signoff_committer_is_member_but_the_commit_is_not_verified() {
    let commit1 = Commit {
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            login: Some("user1".to_string()),
            ..Default::default()
        }),
        verified: Some(false),
        ..Default::default()
    };

    let config = Config {
        require: Some(ConfigRequire { members: Some(false) }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec!["user1".to_string()],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffNotFound],
                success_reason: None,
            }],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn single_commit_valid_signoff_author_match() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        commits: vec![commit1.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![],
                success_reason: Some(CommitSuccessReason::ValidSignOff),
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn single_commit_valid_signoff_committer_match() {
    let commit1 = Commit {
        author: Some(User {
            name: "user2".to_string(),
            email: "user2@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        commits: vec![commit1.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![],
                success_reason: Some(CommitSuccessReason::ValidSignOff),
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn single_commit_valid_signoff_multiple_signoffs() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        commits: vec![commit1.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![],
                success_reason: Some(CommitSuccessReason::ValidSignOff),
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn single_commit_valid_signoff_signoff_case_insensitive() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        commits: vec![commit1.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![],
                success_reason: Some(CommitSuccessReason::ValidSignOff),
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn single_commit_valid_signoff_signoff_trailing_whitespace() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test\n\nSigned-off-by: user1 <user1@email.test>   ".to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![],
                success_reason: Some(CommitSuccessReason::ValidSignOff),
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn single_commit_valid_signoff_email_contains_subdomain() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.some.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        commits: vec![commit1.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![],
                success_reason: Some(CommitSuccessReason::ValidSignOff),
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn single_commit_valid_signoff_email_contains_plus_alias() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1+alias@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        commits: vec![commit1.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![],
                success_reason: Some(CommitSuccessReason::ValidSignOff),
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn single_commit_invalid_author_email() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "invalid".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::InvalidAuthorEmail],
                success_reason: None,
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn single_commit_invalid_author_email_and_no_signoff() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "invalid".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::InvalidAuthorEmail, CommitError::SignOffNotFound],
                success_reason: None,
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn single_commit_invalid_author_email_also_used_in_signoff() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "invalid".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::InvalidAuthorEmail],
                success_reason: None,
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn single_commit_invalid_committer_email() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::InvalidCommitterEmail],
                success_reason: None,
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn single_commit_invalid_committer_email_and_no_signoff() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::InvalidCommitterEmail, CommitError::SignOffNotFound],
                success_reason: None,
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn single_commit_invalid_committer_email_also_used_in_signoff() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::InvalidCommitterEmail],
                success_reason: None,
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn single_commit_invalid_author_and_committer_email_same_email() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "invalid".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::InvalidCommitterEmail],
                success_reason: None,
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn single_commit_invalid_author_and_committer_email_different_emails() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "invalid".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![
                    CommitError::InvalidCommitterEmail,
                    CommitError::InvalidAuthorEmail
                ],
                success_reason: None,
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn single_commit_signoff_not_found() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffNotFound],
                success_reason: None,
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn single_commit_invalid_signoff_multiple_signoffs() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffMismatch],
                success_reason: None,
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn single_commit_invalid_signoff_name_mismatch() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffMismatch],
                success_reason: None,
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn single_commit_invalid_signoff_email_mismatch() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffMismatch],
                success_reason: None,
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn single_commit_invalid_signoff_name_and_email_mismatch() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffMismatch],
                success_reason: None,
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn single_commit_invalid_signoff_extra_whitespace_around_name() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffMismatch],
                success_reason: None,
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn single_commit_invalid_signoff_extra_whitespace_around_email() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffMismatch],
                success_reason: None,
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn single_commit_invalid_signoff_missing_name_in_signoff() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffNotFound],
                success_reason: None,
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn single_commit_invalid_signoff_missing_email_in_signoff() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffNotFound],
                success_reason: None,
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn single_commit_invalid_signoff_missing_email_brackets_in_signoff() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffNotFound],
                success_reason: None,
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn single_commit_invalid_signoff_missing_name_and_email_in_signoff() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffNotFound],
                success_reason: None,
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn single_commit_invalid_signoff_name_and_email_swapped_in_signoff() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffMismatch],
                success_reason: None,
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn single_commit_invalid_signoff_invalid_email_in_signoff() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffMismatch],
                success_reason: None,
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn single_commit_invalid_signoff_email_alias_used_in_signoff_but_not_in_author_email() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![CommitCheckOutput {
                commit: commit1,
                errors: vec![CommitError::SignOffMismatch],
                success_reason: None,
            }],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn two_commits_valid_signoff_in_both() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        commits: vec![commit1.clone(), commit2.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_valid_signoff_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        commits: vec![commit1.clone(), commit2.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_valid_signoff_in_first_no_signoff_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                }
            ],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn two_commits_invalid_signoff_in_first_valid_signoff_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        commits: vec![commit1.clone(), commit2.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![CommitError::SignOffMismatch],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_valid_signoff_in_first_invalid_signoff_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![CommitError::SignOffMismatch],
                    success_reason: None,
                }
            ],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_invalid_signoff_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![CommitError::SignOffMismatch],
                    success_reason: None,
                }
            ],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 2,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_invalid_signoff_in_first_no_signoff_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![CommitError::SignOffMismatch],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                }
            ],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 2,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_valid_remediation_commit_in_second_but_remediation_not_enabled_in_config()
{
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_valid_remediation_commit_matching_author_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user2".to_string(),
            email: "user2@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOffInRemediationCommit),
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_valid_remediation_commit_matching_committer_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user2".to_string(),
            email: "user2@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOffInRemediationCommit),
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_invalid_signoff_incorrect_name_in_first_valid_remediation_commit_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            Signed-off-by: userx <user1@email.test>
        "}
        .to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOffInRemediationCommit),
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_invalid_signoff_incorrect_email_in_first_valid_remediation_commit_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1 <userx@email.test>
        "}
        .to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOffInRemediationCommit),
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_valid_signoff_in_first_redundant_remediation_commit_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_valid_signoff_in_first_remediation_commit_non_existent_sha_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: non-existent

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_remediation_commit_no_signoff_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOffInRemediationCommit),
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_remediation_commit_different_name_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "userx".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "userx".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            I, userx <user1@email.test>, hereby add my Signed-off-by to this commit: sha1

            Signed-off-by: userx <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_remediation_commit_different_email_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "userx@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "userx@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            I, user1 <userx@email.test>, hereby add my Signed-off-by to this commit: sha1

            Signed-off-by: user1 <userx@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_remediation_commit_different_name_and_email_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "userx".to_string(),
            email: "userx@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "userx".to_string(),
            email: "userx@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            I, userx <userx@email.test>, hereby add my Signed-off-by to this commit: sha1

            Signed-off-by: userx <userx@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_remediation_commit_different_name_in_signoff_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1

            Signed-off-by: userx <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOffInRemediationCommit),
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![CommitError::SignOffMismatch],
                    success_reason: None,
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_remediation_commit_different_email_in_signoff_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1

            Signed-off-by: user1 <userx@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOffInRemediationCommit),
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![CommitError::SignOffMismatch],
                    success_reason: None,
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_remediation_commit_different_name_and_email_in_signoff_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1

            Signed-off-by: userx <userx@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOffInRemediationCommit),
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![CommitError::SignOffMismatch],
                    success_reason: None,
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_remediation_commit_different_name_in_remediation_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            I, userx <user1@email.test>, hereby add my Signed-off-by to this commit: sha1

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_remediation_commit_different_email_in_remediation_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            I, user1 <userx@email.test>, hereby add my Signed-off-by to this commit: sha1

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_remediation_commit_different_name_and_email_in_remediation_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            I, userx <userx@email.test>, hereby add my Signed-off-by to this commit: sha1

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_remediation_commit_sha_mismatch_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha2

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_3p_valid_remediation_commit_in_second_but_remediation_not_enabled_in_config(
) {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user2".to_string(),
            email: "user2@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user2".to_string(),
            email: "user2@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            On behalf of user1 <user1@email.test>, I, user2 <user2@email.test>, hereby add my Signed-off-by to this commit: sha1

            Signed-off-by: user2 <user2@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_3p_valid_remediation_commit_in_second_but_3p_remediation_not_enabled_in_config(
) {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user2".to_string(),
            email: "user2@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user2".to_string(),
            email: "user2@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            On behalf of user1 <user1@email.test>, I, user2 <user2@email.test>, hereby add my Signed-off-by to this commit: sha1

            Signed-off-by: user2 <user2@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_3p_valid_remediation_commit_from_same_author_and_committer_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            On behalf of user1 <user1@email.test>, I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            third_party: Some(true),
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOffInRemediationCommit),
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_3p_valid_remediation_commit_from_different_author_and_committer_in_second()
{
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user2".to_string(),
            email: "user2@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user2".to_string(),
            email: "user2@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            On behalf of user1 <user1@email.test>, I, user2 <user2@email.test>, hereby add my Signed-off-by to this commit: sha1

            Signed-off-by: user2 <user2@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            third_party: Some(true),
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOffInRemediationCommit),
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_3p_valid_remediation_commit_from_committer_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user2".to_string(),
            email: "user2@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user2".to_string(),
            email: "user2@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user2".to_string(),
            email: "user2@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            On behalf of user1 <user1@email.test>, I, user2 <user2@email.test>, hereby add my Signed-off-by to this commit: sha1

            Signed-off-by: user2 <user2@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            third_party: Some(true),
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOffInRemediationCommit),
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_3p_valid_remediation_commit_in_second_individual_remediations_disabled() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            On behalf of user1 <user1@email.test>, I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(false),
            third_party: Some(true),
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_3p_remediation_commit_declarant_name_mismatch_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            On behalf of user2 <user1@email.test>, I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            third_party: Some(true),
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_3p_remediation_commit_declarant_email_mismatch_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            On behalf of user1 <user2@email.test>, I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            third_party: Some(true),
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_3p_remediation_commit_sha_mismatch_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            On behalf of user1 <user1@email.test>, I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha2

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            third_party: Some(true),
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_invalid_3p_remediation_commit_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            For user1 <user1@email.test>, I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            third_party: Some(true),
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_3p_remediation_commit_representative_name_mismatch_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            For user1 <user1@email.test>, I, user2 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            third_party: Some(true),
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_3p_remediation_commit_representative_email_mismatch_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            For user1 <user1@email.test>, I, user1 <user2@email.test>, hereby add my Signed-off-by to this commit: sha1

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            third_party: Some(true),
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn two_commits_no_signoff_in_first_3p_remediation_commit_no_signoff_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            For user1 <user1@email.test>, I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            third_party: Some(true),
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 2,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn three_commits_valid_signoff_in_all() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        commits: vec![commit1.clone(), commit2.clone(), commit3.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                },
                CommitCheckOutput {
                    commit: commit3,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn three_commits_valid_signoff_first_and_second_no_signoff_third() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        ..Default::default()
    };

    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone(), commit3.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                },
                CommitCheckOutput {
                    commit: commit3,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                }
            ],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}

#[test]
fn three_commits_invalid_signoff_first_no_signoff_second_valid_signoff_third() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        ..Default::default()
    };
    let commit3 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        commits: vec![commit1.clone(), commit2.clone(), commit3.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![CommitError::SignOffMismatch],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit3,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 2,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn three_commits_valid_signoff_first_invalid_signoff_second_valid_signoff_third() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
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
        commits: vec![commit1.clone(), commit2.clone(), commit3.clone()],
        config: Default::default(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![CommitError::SignOffMismatch],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit3,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config: Default::default(),
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn three_commits_no_signoff_in_first_remediation_commit_without_signoff_in_second_valid_remediation_commit_in_third(
) {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1
        "}
        .to_string(),
        sha: "sha2".to_string(),
        ..Default::default()
    };
    let commit3 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha2

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone(), commit3.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOffInRemediationCommit),
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOffInRemediationCommit),
                },
                CommitCheckOutput {
                    commit: commit3,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn three_commits_no_signoff_in_first_no_signoff_in_second_valid_remediation_commit_for_both_in_third() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha2".to_string(),
        ..Default::default()
    };
    let commit3 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1
            I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha2

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone(), commit3.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOffInRemediationCommit),
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOffInRemediationCommit),
                },
                CommitCheckOutput {
                    commit: commit3,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn three_commits_valid_signoff_in_first_redundant_remediation_commit_in_second_redundant_3p_remediation_commit_in_third(
) {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        sha: "sha2".to_string(),
        ..Default::default()
    };
    let commit3 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            On behalf of user1 <user1@email.test>, I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha2

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            third_party: Some(true),
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone(), commit3.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                },
                CommitCheckOutput {
                    commit: commit3,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn three_commits_no_signoff_in_first_valid_remediation_commit_in_second_redundant_3p_remediation_commit_in_third(
) {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        sha: "sha2".to_string(),
        ..Default::default()
    };
    let commit3 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            On behalf of user1 <user1@email.test>, I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha2

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            third_party: Some(true),
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone(), commit3.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOffInRemediationCommit),
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                },
                CommitCheckOutput {
                    commit: commit3,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn three_commits_no_signoff_in_first_remediation_commit_no_signoff_in_second_valid_3p_remediation_commit_in_third(
) {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1
        "}
        .to_string(),
        sha: "sha2".to_string(),
        ..Default::default()
    };
    let commit3 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            On behalf of user1 <user1@email.test>, I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha2

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            third_party: Some(true),
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone(), commit3.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOffInRemediationCommit),
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOffInRemediationCommit),
                },
                CommitCheckOutput {
                    commit: commit3,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn three_commits_no_signoff_in_first_3p_remediation_commit_no_signoff_in_second_valid_remediation_commit_in_third(
) {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            On behalf of user1 <user1@email.test>, I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1
        "}
        .to_string(),
        sha: "sha2".to_string(),
        ..Default::default()
    };
    let commit3 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha2

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            third_party: Some(true),
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone(), commit3.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOffInRemediationCommit),
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOffInRemediationCommit),
                },
                CommitCheckOutput {
                    commit: commit3,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn three_commits_no_signoff_in_first_3p_remediation_commit_no_signoff_in_second_valid_3p_remediation_commit_in_third(
) {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            On behalf of user1 <user1@email.test>, I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1
        "}
        .to_string(),
        sha: "sha2".to_string(),
        ..Default::default()
    };
    let commit3 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            On behalf of user1 <user1@email.test>, I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha2

            Signed-off-by: user1 <user1@email.test>
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            third_party: Some(true),
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone(), commit3.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOffInRemediationCommit),
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOffInRemediationCommit),
                },
                CommitCheckOutput {
                    commit: commit3,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOff),
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 0,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn three_commits_no_signoff_in_first_3p_remediation_commit_no_signoff_in_second_3p_remediation_commit_no_signoff_in_third(
) {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            On behalf of user1 <user1@email.test>, I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1
        "}
        .to_string(),
        sha: "sha2".to_string(),
        ..Default::default()
    };
    let commit3 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            On behalf of user1 <user1@email.test>, I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha2
        "}
        .to_string(),
        ..Default::default()
    };

    let config = Config {
        allow_remediation_commits: Some(ConfigAllowRemediationCommits {
            individual: Some(true),
            third_party: Some(true),
        }),
        ..Default::default()
    };
    let input = CheckInput {
        commits: vec![commit1.clone(), commit2.clone(), commit3.clone()],
        config: config.clone(),
        head_ref: "main".to_string(),
        members: vec![],
    };
    let output = check(&input);

    assert_eq!(
        output,
        CheckOutput {
            commits: vec![
                CommitCheckOutput {
                    commit: commit1,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOffInRemediationCommit),
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::ValidSignOffInRemediationCommit),
                },
                CommitCheckOutput {
                    commit: commit3,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
    );
}
