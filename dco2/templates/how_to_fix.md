{%~ if commits|contains_error([CommitError::SignOffNotFound, CommitError::SignOffMismatch]) +%}
  ## How to fix missing or invalid sign-offs

  {%~ include "how_to_fix_option_1.md" +%}

  {%~ include "how_to_fix_option_2.md" +%}
{% endif %}
