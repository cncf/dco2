### Option 1: add remediation commit

{%+ if config.individual_remediation_commits_are_allowed() %}
  * [Commit author adds a remediation commit](https://github.com/cncf/dco2?tab=readme-ov-file#individual)
  {%~ if config.third_party_remediation_commits_are_allowed() %}
  * [Authorized individual adds a remediation commit on behalf of the commit author](https://github.com/cncf/dco2?tab=readme-ov-file#third-party)
  {% endif +%}
{%+ else %}
  Remediation commits are not allowed for this repository. For more details about how to enable them, please see the [documentation](https://github.com/cncf/dco2?tab=readme-ov-file#remediation-commits).
{% endif +%}
