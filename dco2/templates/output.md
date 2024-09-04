{% if num_commits_with_errors == 0 -%}
All commits are signed off!
{% else -%}
There is at least one commit incorrectly signed off. This means that the author of this commit failed to include a Signed-off-by line in the commit message.
{% endif -%}

{% if commits|contains_error([CommitError::SignOffNotFound, CommitError::SignOffMismatch]) %}
Tada!
{% endif %}
