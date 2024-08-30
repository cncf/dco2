{% if commits_with_errors.is_empty() -%}
All commits are signed off!
{% else -%}
There is at least one commit incorrectly signed off. This means that the author of this commit failed to include a Signed-off-by line in the commit message.
{% endif -%}
