#!/usr/bin/env bash
#
# Sample script to label open pull requests whose DCO2 check is failing.
#
# DCO2 only labels pull requests when it processes new events for them, so this
# script can be used to label the pull requests that were already open when the
# `label` option was enabled. See the "Labeling pull requests" section in the
# README for more details.
#
# Requirements: bash 4 or later and the GitHub CLI (https://cli.github.com),
# authenticated with access to the repository (`gh auth login`). Labels with
# non-ASCII characters also require a UTF-8 locale.

set -euo pipefail

if ((BASH_VERSINFO[0] < 4)); then
  echo "error: bash 4 or later is required" >&2
  exit 1
fi

usage() {
  cat <<EOF
Usage: $(basename "$0") [--dry-run] [--app-slug SLUG] OWNER/REPO LABEL

Adds LABEL to the open pull requests in OWNER/REPO whose latest DCO check run
from the DCO2 GitHub App for that pull request failed (conclusion:
action_required).

Pull requests that already have the label (case-insensitive) or that don't have
a DCO check run from the app are skipped. The label is created if it doesn't
exist (with the same color and description DCO2 uses); existing labels are
never modified.

Options:
  --app-slug SLUG  Slug of the DCO2 GitHub App (default: dco-2)
  --dry-run        Print what would be done without making any changes
  -h, --help       Show this help
EOF
}

die() {
  echo "error: $*" >&2
  exit 1
}

is_ascii() {
  ! LC_ALL=C grep -q '[^[:print:]]' <<<"$1"
}

# Parse arguments
app_slug="dco-2"
dry_run=false
positional=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --app-slug)
      [[ $# -ge 2 ]] || die "--app-slug requires a value"
      app_slug="$2"
      shift 2
      ;;
    --dry-run)
      dry_run=true
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    --)
      shift
      positional+=("$@")
      break
      ;;
    -*)
      usage >&2
      die "unknown option: $1"
      ;;
    *)
      positional+=("$1")
      shift
      ;;
  esac
done
if [[ ${#positional[@]} -ne 2 ]]; then
  usage >&2
  exit 1
fi
repo="${positional[0]}"
label="${positional[1]}"

# Validate arguments
[[ "$repo" =~ ^[^/[:space:]]+/[^/[:space:]]+$ ]] || die "invalid repository: $repo"
[[ "$app_slug" =~ ^[A-Za-z0-9_-]+$ ]] || die "invalid app slug: $app_slug"
if ! is_ascii "$label" && [[ "$(locale charmap 2>/dev/null)" != "UTF-8" ]]; then
  die "labels with non-ASCII characters require a UTF-8 locale"
fi
label="${label#"${label%%[![:space:]]*}"}"
label="${label%"${label##*[![:space:]]}"}"
[[ -n "$label" && ${#label} -le 50 ]] || die "the label must have 1-50 characters"
[[ "$label" != "." && "$label" != ".." ]] || die "invalid label: $label"
command -v gh >/dev/null 2>&1 || die "the GitHub CLI (gh) is required"

label_lc="${label,,}"
tab=$'\t'

# Check if the label exists in the repository (case-insensitive)
label_exists() {
  local names
  names="$(gh api --paginate "repos/$repo/labels?per_page=100" --jq '.[].name' </dev/null)" ||
    die "failed to list labels in $repo"
  grep -Fxq -- "$label_lc" <<<"${names,,}"
}

# Make sure the label exists before the first write, creating it only if missing
label_ready=false
ensure_label() {
  if [[ "$label_ready" == true ]]; then
    return 0
  fi
  if ! label_exists; then
    if [[ "$dry_run" == true ]]; then
      echo "Would create label \"$label\""
    else
      # Without --force, an existing label is never updated. DCO2 may create
      # the label concurrently, so check again before giving up
      gh label create --repo "$repo" --color ededed \
        --description "DCO check requires attention" -- "$label" </dev/null ||
        label_exists ||
        die "failed to create label \"$label\""
    fi
  fi
  label_ready=true
}

# List open pull requests: number, head SHA and labels (tab separated)
echo "Listing open pull requests in $repo..."
prs="$(gh api --paginate "repos/$repo/pulls?state=open&per_page=100" \
  --jq '.[] | [(.number | tostring), .head.sha, .labels[].name] | join("\t")')"

# Process pull requests
checked=0
labeled=0
skipped=0
failed=0
while IFS="$tab" read -r number sha labels <&3; do
  [[ -n "$number" ]] || continue
  checked=$((checked + 1))

  # Skip pull requests that already have the label
  if [[ "$tab${labels,,}$tab" == *"$tab$label_lc$tab"* ]]; then
    echo "#$number: skipped (already labeled)"
    skipped=$((skipped + 1))
    continue
  fi

  # Get the conclusion of the latest DCO check run created by the DCO2 app for
  # this pull request (check runs reference it in their external id, and those
  # with an empty external id apply to any pull request)
  if ! conclusion="$(gh api \
    "repos/$repo/commits/$sha/check-runs?check_name=DCO&filter=latest&per_page=100" \
    --jq "[.check_runs[] | select(.app.slug == \"$app_slug\")
      | select((.external_id // \"\") as \$id | \$id == \"\" or \$id == \"$number\")]
      | max_by(.id) | if . == null then \"none\" else (.conclusion // .status) end" \
    </dev/null)"; then
    echo "#$number: failed to get DCO check run" >&2
    failed=$((failed + 1))
    continue
  fi
  if [[ "$conclusion" != "action_required" ]]; then
    if [[ "$conclusion" == "none" ]]; then
      echo "#$number: skipped (no DCO check run from $app_slug)"
    else
      echo "#$number: skipped (DCO check: $conclusion)"
    fi
    skipped=$((skipped + 1))
    continue
  fi

  # Label pull request
  ensure_label
  if [[ "$dry_run" == true ]]; then
    echo "#$number: would add label \"$label\""
    labeled=$((labeled + 1))
    continue
  fi
  if gh api --method POST "repos/$repo/issues/$number/labels" \
    -f "labels[]=$label" --silent </dev/null; then
    echo "#$number: labeled"
    labeled=$((labeled + 1))
  else
    echo "#$number: failed to add label" >&2
    failed=$((failed + 1))
  fi

  # Pause between writes to respect GitHub's secondary rate limits
  sleep 1
done 3<<<"$prs"

# Print summary
if [[ "$dry_run" == true ]]; then
  labeled_msg="$labeled would be labeled"
else
  labeled_msg="$labeled labeled"
fi
echo "Done: $checked checked, $labeled_msg, $skipped skipped, $failed failed"
[[ $failed -eq 0 ]]
