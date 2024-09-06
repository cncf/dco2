## Errors details

{%~ if commits|contains_error([CommitError::InvalidAuthorEmail]) +%}
  ### Invalid author email

  The commit author email is not [valid](https://datatracker.ietf.org/doc/html/rfc5322#section-3.4.1).
{%~ endif +%}

{%~ if commits|contains_error([CommitError::InvalidCommitterEmail]) +%}
  ### Invalid committer email

  The commit committer email is not [valid](https://datatracker.ietf.org/doc/html/rfc5322#section-3.4.1).
{%~ endif +%}

{%~ if commits|contains_error([CommitError::SignOffNotFound]) +%}
  ### Sign-off not found

  No sign-off was found in the commit message. This usually means that the author or committer of this commit failed to include a Signed-off-by line in the commit message. In some cases, this error can also be raised if the sign-off is not in the correct format.

  To avoid having pull requests blocked in the future, always include a `Signed-off-by: User1 <user1@email.test>` line in *every* commit message. You can also do this automatically by using the -s flag (i.e., `git commit -s`).
{%~ endif +%}

{%~ if commits|contains_error([CommitError::SignOffMismatch]) +%}
  ### No sign-off matches the author or committer

  A valid sign-off was found in the commit message, but it doesn't match neither the author nor the committer. Make sure that both the name and email in the sign-off line match the author or committer of the commit.
{%~ endif +%}
