use std::vec;

use askama::Template;
use indoc::indoc;
use pretty_assertions::assert_eq;

use crate::{
    dco::check::{CheckInput, CheckOutput, CommitCheckOutput, CommitError, CommitSuccessReason, check},
    github::{Commit, Config, ConfigAllowRemediationCommits, ConfigRequire, User},
};

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
fn single_commit_no_signoff_author_is_member_but_the_commit_verification_is_unknown() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            login: Some("user1".to_string()),
            ..Default::default()
        }),
        verified: None,
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

            signed-off-by: USER1 <USER1@Email.Test>
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
fn single_commit_valid_signoff_github_bot_noreply_email() {
    let commit1 = Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "github-actions[bot]".to_string(),
            email: "12345678+github-actions[bot]@users.noreply.github.com".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            Signed-off-by: github-actions[bot] <12345678+github-actions[bot]@users.noreply.github.com>
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
fn two_commits_no_signoff_in_first_remediation_commit_committed_by_declarant_in_second() {
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
fn two_commits_no_signoff_in_first_remediation_commit_case_insensitive_declarant_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "User1".to_string(),
            email: "User1@Email.Test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "User1".to_string(),
            email: "User1@Email.Test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "User1".to_string(),
            email: "User1@Email.Test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "User1".to_string(),
            email: "User1@Email.Test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            i, user1 <user1@email.test>, HEREBY add my Signed-off-by to this commit: sha1

            Signed-off-by: User1 <User1@Email.Test>
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
fn two_commits_no_signoff_in_first_remediation_merge_commit_no_signoff_in_second() {
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
        is_merge: true,
        message: indoc! {r"
            Merge branch 'main' into feature

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
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![],
                    success_reason: Some(CommitSuccessReason::IsMerge),
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
fn two_commits_no_signoff_in_first_valid_remediation_merge_commit_in_second() {
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
        is_merge: true,
        message: indoc! {r"
            Merge branch 'main' into feature

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
                    success_reason: Some(CommitSuccessReason::IsMerge),
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
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![CommitError::SignOffMismatch],
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
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![CommitError::SignOffMismatch],
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
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![CommitError::SignOffMismatch],
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
fn two_commits_no_signoff_in_first_remediation_commit_from_different_author_in_second() {
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

            I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1

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
fn two_commits_no_signoff_in_first_remediation_commit_abbreviated_sha_in_second() {
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
        sha: "1111111111abcdef".to_string(),
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

            I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: 1111111

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
fn two_commits_no_signoff_in_first_remediation_commit_with_trailing_whitespace_in_second() {
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
        message: concat!(
            "Test commit message\n",
            "\n",
            "I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1   \n",
            "\n",
            "Signed-off-by: user1 <user1@email.test>\n",
        )
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
fn two_commits_no_signoff_in_first_remediation_commit_with_crlf_line_endings_in_second() {
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
        .replace('\n', "\r\n"),
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
fn two_commits_no_signoff_in_first_3p_valid_remediation_commit_in_second_but_remediation_not_enabled_in_config()
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
fn two_commits_no_signoff_in_first_3p_valid_remediation_commit_in_second_but_3p_remediation_not_enabled_in_config()
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
fn two_commits_no_signoff_in_first_3p_remediation_commit_case_insensitive_in_second() {
    let commit1 = Commit {
        author: Some(User {
            name: "User1".to_string(),
            email: "User1@Email.Test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "User1".to_string(),
            email: "User1@Email.Test".to_string(),
            ..Default::default()
        }),
        message: "Test commit message".to_string(),
        sha: "sha1".to_string(),
        ..Default::default()
    };
    let commit2 = Commit {
        author: Some(User {
            name: "User2".to_string(),
            email: "User2@Email.Test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "User2".to_string(),
            email: "User2@Email.Test".to_string(),
            ..Default::default()
        }),
        message: indoc! {r"
            Test commit message

            on behalf of user1 <user1@email.test>, i, user2 <user2@email.test>, HEREBY add my Signed-off-by to this commit: sha1

            Signed-off-by: User2 <User2@Email.Test>
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
fn two_commits_no_signoff_in_first_3p_remediation_commit_committed_by_representative_in_second() {
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
            name: "user3".to_string(),
            email: "user3@email.test".to_string(),
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
fn two_commits_no_signoff_in_first_3p_remediation_commit_with_trailing_whitespace_in_second() {
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
        message: concat!(
            "Test commit message\n",
            "\n",
            "On behalf of user1 <user1@email.test>, I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1   \n",
            "\n",
            "Signed-off-by: user1 <user1@email.test>\n",
        )
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
fn two_commits_no_signoff_in_first_3p_remediation_commit_with_crlf_line_endings_in_second() {
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
        .replace('\n', "\r\n"),
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

            On behalf of user1 <user1@email.test>, I, user2 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1

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

            On behalf of user1 <user1@email.test>, I, user1 <user2@email.test>, hereby add my Signed-off-by to this commit: sha1

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

            On behalf of user1 <user1@email.test>, I, user1 <user1@email.test>, hereby add my Signed-off-by to this commit: sha1
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
fn three_commits_no_signoff_in_first_remediation_commit_without_signoff_in_second_valid_remediation_commit_in_third()
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
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
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
            num_commits_with_errors: 1,
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
fn three_commits_valid_signoff_in_first_redundant_remediation_commit_in_second_redundant_3p_remediation_commit_in_third()
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
fn three_commits_no_signoff_in_first_valid_remediation_commit_in_second_redundant_3p_remediation_commit_in_third()
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
fn three_commits_no_signoff_in_first_remediation_commit_no_signoff_in_second_valid_3p_remediation_commit_in_third()
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
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
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
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn three_commits_no_signoff_in_first_3p_remediation_commit_no_signoff_in_second_valid_remediation_commit_in_third()
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
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
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
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn three_commits_no_signoff_in_first_3p_remediation_commit_no_signoff_in_second_valid_3p_remediation_commit_in_third()
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
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
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
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn three_commits_no_signoff_in_first_3p_remediation_commit_no_signoff_in_second_3p_remediation_commit_no_signoff_in_third()
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
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit2,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                },
                CommitCheckOutput {
                    commit: commit3,
                    errors: vec![CommitError::SignOffNotFound],
                    success_reason: None,
                }
            ],
            config,
            head_ref: "main".to_string(),
            num_commits_with_errors: 3,
            only_last_commit_contains_errors: false,
        }
    );
}

#[test]
fn output_check_passed_renders_summary_table() {
    // Setup a passing check output with a long commit subject
    let mut commit2_output = render_commit_output(
        "fedcba9876543210",
        "This is a very long commit subject that exceeds fifty characters",
        vec![],
    );
    commit2_output.success_reason = Some(CommitSuccessReason::ValidSignOffInRemediationCommit);
    let output = CheckOutput {
        commits: vec![
            render_commit_output(
                "0123456789abcdef",
                "Test commit message\n\nSigned-off-by: user1 <user1@email.test>",
                vec![],
            ),
            commit2_output,
        ],
        config: Default::default(),
        head_ref: "feature".to_string(),
        num_commits_with_errors: 0,
        only_last_commit_contains_errors: false,
    };

    // Check the whole rendered output
    assert_eq!(
        output.render().unwrap(),
        indoc! {r"
            All commits are signed off, the check **passed**.
            ## Summary

            <br />

            | | Sha | Message | Pass or fail reason |
            | -- | ---- | :---- | :---- |
            | :green_circle: | [0123456](https://github.com/org/repo/commit/0123456789abcdef) |Test commit message|`Valid sign-off found`|
            | :green_circle: | [fedcba9](https://github.com/org/repo/commit/fedcba9876543210) |This is a very long commit subject that exceeds fi...|`Valid sign-off found in remediation commit`|

        "}
    );
}

#[test]
fn output_errors_details_include_each_error_found_once() {
    // Setup a check output where two commits share the same error
    let output = CheckOutput {
        commits: vec![
            render_commit_output(
                "0123456789abcdef",
                "Test commit message",
                vec![CommitError::InvalidCommitterEmail, CommitError::SignOffNotFound],
            ),
            render_commit_output(
                "fedcba9876543210",
                "Test commit message",
                vec![CommitError::SignOffNotFound],
            ),
        ],
        config: Default::default(),
        head_ref: "feature".to_string(),
        num_commits_with_errors: 2,
        only_last_commit_contains_errors: false,
    };

    // Render the output
    let rendered = output.render().unwrap();

    // Check only the errors found are detailed, once each
    assert_eq!(rendered.matches("## Errors details").count(), 1);
    assert_eq!(rendered.matches("### Invalid committer email").count(), 1);
    assert_eq!(rendered.matches("### Sign-off not found").count(), 1);
    assert!(!rendered.contains("### Invalid author email"));
    assert!(!rendered.contains("### No sign-off matches the author or committer"));
}

#[test]
fn output_errors_details_without_how_to_fix_for_email_errors_only() {
    // Setup a check output with an email error only
    let output = CheckOutput {
        commits: vec![render_commit_output(
            "0123456789abcdef",
            "Test commit message",
            vec![CommitError::InvalidAuthorEmail],
        )],
        config: Default::default(),
        head_ref: "feature".to_string(),
        num_commits_with_errors: 1,
        only_last_commit_contains_errors: true,
    };

    // Render the output
    let rendered = output.render().unwrap();

    // Check the error is detailed without sign-off fixing instructions
    assert!(rendered.contains("|`Invalid author email`|"));
    assert!(rendered.contains("### Invalid author email"));
    assert!(!rendered.contains("## How to fix"));
}

#[test]
fn output_headline_counts_failing_commits() {
    // Check the headline when the only commit fails
    let output = CheckOutput {
        commits: vec![render_commit_output(
            "0123456789abcdef",
            "Test commit message",
            vec![CommitError::SignOffNotFound],
        )],
        config: Default::default(),
        head_ref: "feature".to_string(),
        num_commits_with_errors: 1,
        only_last_commit_contains_errors: true,
    };
    let rendered = output.render().unwrap();
    assert!(
        rendered.starts_with("**All commits** are incorrectly signed off, the check **did not pass**.\n")
    );

    // Check the headline when one of several commits fails
    let output = CheckOutput {
        commits: vec![
            render_commit_output("0123456789abcdef", "Test commit message", vec![]),
            render_commit_output(
                "fedcba9876543210",
                "Test commit message",
                vec![CommitError::SignOffNotFound],
            ),
        ],
        config: Default::default(),
        head_ref: "feature".to_string(),
        num_commits_with_errors: 1,
        only_last_commit_contains_errors: true,
    };
    let rendered = output.render().unwrap();
    assert!(
        rendered.starts_with("There is **one commit** incorrectly signed off, the check **did not pass**.\n")
    );

    // Check the headline and summary rows when some commits fail
    let output = CheckOutput {
        commits: vec![
            render_commit_output(
                "0123456789abcdef",
                "Test commit message",
                vec![CommitError::InvalidAuthorEmail, CommitError::SignOffNotFound],
            ),
            render_commit_output(
                "fedcba9876543210",
                "Test commit message",
                vec![CommitError::SignOffNotFound],
            ),
            render_commit_output("abcdef0123456789", "Test commit message", vec![]),
        ],
        config: Default::default(),
        head_ref: "feature".to_string(),
        num_commits_with_errors: 2,
        only_last_commit_contains_errors: false,
    };
    let rendered = output.render().unwrap();
    assert!(
        rendered.starts_with("There are **2 commits** incorrectly signed off, the check **did not pass**.\n")
    );
    assert!(rendered.contains(indoc! {r"
        | :red_circle: | [0123456](https://github.com/org/repo/commit/0123456789abcdef) |Test commit message|`Invalid author email` `Sign-off not found`|
        | :red_circle: | [fedcba9](https://github.com/org/repo/commit/fedcba9876543210) |Test commit message|`Sign-off not found`|
        | :green_circle: | [abcdef0](https://github.com/org/repo/commit/abcdef0123456789) |Test commit message|`Valid sign-off found`|
    "}));
}

#[test]
fn output_how_to_fix_amend_last_commit_when_only_last_commit_fails() {
    // Setup a check output where only the last commit fails
    let output = CheckOutput {
        commits: vec![
            render_commit_output("0123456789abcdef", "Test commit message", vec![]),
            render_commit_output(
                "fedcba9876543210",
                "Test commit message",
                vec![CommitError::SignOffMismatch],
            ),
        ],
        config: Default::default(),
        head_ref: "feature".to_string(),
        num_commits_with_errors: 1,
        only_last_commit_contains_errors: true,
    };

    // Render the output
    let rendered = output.render().unwrap();

    // Check the instructions amend the last commit and push the head ref
    assert!(rendered.contains("## How to fix missing or invalid sign-offs"));
    assert!(rendered.contains("#### Amend last commit"));
    assert!(rendered.contains("`git commit --amend --signoff`"));
    assert!(rendered.contains("`git push --force-with-lease origin feature`"));
    assert!(!rendered.contains("#### Rebase the branch"));
}

#[test]
fn output_how_to_fix_rebase_branch_when_not_only_last_commit_fails() {
    // Setup a check output where a commit other than the last one fails
    let output = CheckOutput {
        commits: vec![
            render_commit_output(
                "0123456789abcdef",
                "Test commit message",
                vec![CommitError::SignOffNotFound],
            ),
            render_commit_output("fedcba9876543210", "Test commit message", vec![]),
            render_commit_output("abcdef0123456789", "Test commit message", vec![]),
        ],
        config: Default::default(),
        head_ref: "feature".to_string(),
        num_commits_with_errors: 1,
        only_last_commit_contains_errors: false,
    };

    // Render the output
    let rendered = output.render().unwrap();

    // Check the instructions rebase all commits and push the head ref
    assert!(rendered.contains("## How to fix missing or invalid sign-offs"));
    assert!(rendered.contains("#### Rebase the branch"));
    assert!(rendered.contains("`git rebase HEAD~3 --signoff`"));
    assert!(rendered.contains("`git push --force-with-lease origin feature`"));
    assert!(!rendered.contains("#### Amend last commit"));
}

#[test]
fn output_how_to_fix_remediation_options_depend_on_config() {
    // Setup the expected snippets and a renderer for each remediation config
    let not_allowed = "Remediation commits are not allowed for this repository";
    let individual_link = "https://github.com/cncf/dco2?tab=readme-ov-file#individual";
    let third_party_link = "https://github.com/cncf/dco2?tab=readme-ov-file#third-party";
    let render = |individual: bool, third_party: bool| {
        CheckOutput {
            commits: vec![render_commit_output(
                "0123456789abcdef",
                "Test commit message",
                vec![CommitError::SignOffNotFound],
            )],
            config: Config {
                allow_remediation_commits: Some(ConfigAllowRemediationCommits {
                    individual: Some(individual),
                    third_party: Some(third_party),
                }),
                ..Default::default()
            },
            head_ref: "feature".to_string(),
            num_commits_with_errors: 1,
            only_last_commit_contains_errors: true,
        }
        .render()
        .unwrap()
    };

    // Remediation commits disabled
    let rendered = render(false, false);
    assert!(rendered.contains(not_allowed));
    assert!(!rendered.contains(individual_link));
    assert!(!rendered.contains(third_party_link));

    // Third-party remediation commits require individual ones to be enabled
    let rendered = render(false, true);
    assert!(rendered.contains(not_allowed));
    assert!(!rendered.contains(individual_link));
    assert!(!rendered.contains(third_party_link));

    // Only individual remediation commits enabled
    let rendered = render(true, false);
    assert!(!rendered.contains(not_allowed));
    assert!(rendered.contains(individual_link));
    assert!(!rendered.contains(third_party_link));

    // Individual and third-party remediation commits enabled
    let rendered = render(true, true);
    assert!(!rendered.contains(not_allowed));
    assert!(rendered.contains(individual_link));
    assert!(rendered.contains(third_party_link));
}

// Helpers.

/// Create a commit check output to be used in the output render tests.
fn render_commit_output(sha: &str, message: &str, errors: Vec<CommitError>) -> CommitCheckOutput {
    let success_reason = errors.is_empty().then_some(CommitSuccessReason::ValidSignOff);
    CommitCheckOutput {
        commit: Commit {
            html_url: format!("https://github.com/org/repo/commit/{sha}"),
            message: message.to_string(),
            sha: sha.to_string(),
            ..Default::default()
        },
        errors,
        success_reason,
    }
}
