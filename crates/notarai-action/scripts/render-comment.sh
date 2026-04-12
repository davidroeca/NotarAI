#!/usr/bin/env bash
# render-comment.sh - Convert notarai check JSON output into a PR comment.
#
# Usage: render-comment.sh < notarai-check.json
#
# Reads the CHECK_EXIT_CODE environment variable to determine the overall status.
# Outputs Markdown suitable for posting as a GitHub PR comment.

set -euo pipefail

json=$(cat)
exit_code="${CHECK_EXIT_CODE:-0}"

# Parse counts from JSON
errors=$(echo "$json" | jq -r '.summary.errors // 0')
warnings=$(echo "$json" | jq -r '.summary.warnings // 0')
total=$((errors + warnings))

# Determine status emoji and text
if [ "$exit_code" -eq 2 ]; then
  cat <<'UNINIT'
<!-- notarai-action -->
## NotarAI Drift Check

NotarAI is not initialized in this repository. Run `notarai init` to get started.
UNINIT
  exit 0
fi

if [ "$errors" -gt 0 ]; then
  status_icon="&#10060;"
  status_text="$errors error(s) found"
elif [ "$warnings" -gt 0 ]; then
  status_icon="&#9888;&#65039;"
  status_text="$warnings warning(s)"
else
  status_icon="&#9989;"
  status_text="All clean"
fi

# Start the comment
cat <<EOF
<!-- notarai-action -->
## NotarAI Drift Check

${status_icon} ${total} finding(s) | ${errors} error(s) | ${warnings} warning(s) -- ${status_text}
EOF

# If there are no findings, we're done
if [ "$total" -eq 0 ]; then
  exit 0
fi

echo ""

# Group findings by type
for check_type in orphaned_glob circular_ref coverage_gap changed_since_reconciliation overlapping_coverage behavior_incomplete; do
  type_findings=$(echo "$json" | jq -c "[.findings[] | select(.type == \"$check_type\")]")
  count=$(echo "$type_findings" | jq 'length')

  if [ "$count" -eq 0 ]; then
    continue
  fi

  # Human-readable type name and severity
  case "$check_type" in
    orphaned_glob)
      label="Orphaned globs"
      severity="error" ;;
    circular_ref)
      label="Circular \$ref chains"
      severity="error" ;;
    coverage_gap)
      label="Coverage gaps"
      severity="warning" ;;
    changed_since_reconciliation)
      label="Changed since reconciliation"
      severity="warning" ;;
    overlapping_coverage)
      label="Overlapping coverage"
      severity="warning" ;;
    behavior_incomplete)
      label="Incomplete behaviors"
      severity="warning" ;;
    *)
      label="$check_type"
      severity="info" ;;
  esac

  if [ "$severity" = "error" ]; then
    icon="&#10060;"
  else
    icon="&#9888;&#65039;"
  fi

  cat <<EOF
<details>
<summary>${icon} ${label} (${count})</summary>

EOF

  echo "$type_findings" | jq -r '.[] | "- " + .message' | head -50

  if [ "$count" -gt 50 ]; then
    echo ""
    echo "_...and $((count - 50)) more_"
  fi

  cat <<EOF

</details>

EOF
done
