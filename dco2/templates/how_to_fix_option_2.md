### Option 2: fix commits without sign-off

{%+ if only_last_commit_contains_errors +%}
  #### Amend last commit

  To fix the incorrectly signed off commit:

  1. Ensure you have a local copy of your branch by [checking out the pull request locally via command line](https://help.github.com/en/github/collaborating-with-issues-and-pull-requests/checking-out-pull-requests-locally).
  2. In your local branch, run: `git commit --amend --signoff`
  3. Force push your changes to overwrite the branch: `git push --force-with-lease origin {{+ head_ref }}`

{%+ else +%}
  #### Rebase the branch

  If you have a local git environment and meet the criteria below, one option is to rebase the branch and add your Signed-off-by lines in the new commits. Please note that if others have already begun work based upon the commits in this branch, this solution will rewrite history and may cause serious issues for collaborators ([described in the git documentation](https://git-scm.com/book/en/v2/Git-Branching-Rebasing) under "The Perils of Rebasing").

  > [!WARNING]
  > **You should only do this if:**
  >
  > * You are the only author of the commits in this branch
  > * You are absolutely certain nobody else is doing any work based upon this branch
  > * There are no empty commits in the branch

  To add your Signed-off-by line to every commit in this branch:

  1. Ensure you have a local copy of your branch by [checking out the pull request locally via command line](https://help.github.com/en/github/collaborating-with-issues-and-pull-requests/checking-out-pull-requests-locally).
  2. In your local branch, run: `git rebase HEAD~{{ total_commits +}} --signoff`
  3. Force push your changes to overwrite the branch: `git push --force-with-lease origin {{+ head_ref }}`
{% endif %}
