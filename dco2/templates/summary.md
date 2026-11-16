{% let table = self.summary_table() %}
## Summary

<br />

| | Sha | Message | Pass or fail reason |
| -- | ---- | :---- | :---- |
{%+ for entry in table.commits %}
  {% let sha_truncated = entry.commit.sha.to_string()|truncate_no_dots(7) %}
  {% let subject = entry.commit.message.lines().next().unwrap_or_default() %}
  {% if entry.errors.is_empty() %}
    | :green_circle: | [{{ sha_truncated }}]({{ entry.commit.html_url }}) | {{ subject|truncate(50) }} | {% if let Some(success_reason) = entry.success_reason %}`{{ success_reason|capitalize }}`{% endif %} |
  {% else %}
    | :red_circle: | [{{ sha_truncated }}]({{ entry.commit.html_url }}) | {{ subject|truncate(50) }} |{% for error in entry.errors %}{% if !loop.first %}{{ " " +}}{% endif %}`{{ error|capitalize }}`{% endfor %} |
  {% endif +%}
{%+ endfor +%}
{% if table.hidden_failing_commits > 0 || table.hidden_passing_commits > 0 ~%}

Showing {{+ table.commits.len() +}} of {{+ commits.len() +}} commits (not displayed: {%+ if table.hidden_failing_commits > 0 %}{{ table.hidden_failing_commits +}} failing{% if table.hidden_passing_commits > 0 %}, {%+ endif %}{% endif %}{% if table.hidden_passing_commits > 0 %}{{ table.hidden_passing_commits +}} passing{% endif %}).
{%+ endif +%}
