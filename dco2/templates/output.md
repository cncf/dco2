{% let total_commits = commits.len() %}

### Check result

{%+ if num_commits_with_errors == 0 %}
  All commits are signed off, the check **passed**.

  {%~ include "summary.md" +%}
{%+ else %}
  {% if num_commits_with_errors == total_commits %}
    **All commits** are incorrectly signed off
  {% else if num_commits_with_errors == 1 %}
    There is **one commit** incorrectly signed off
  {% else %}
    There are **{{+ num_commits_with_errors +}} commits** incorrectly signed off
  {% endif %}
  , the check **did not pass**.

  {%~ include "summary.md" +%}

  {%~ include "errors_details.md" +%}

  {%~ include "how_to_fix.md" +%}

{% endif %}
