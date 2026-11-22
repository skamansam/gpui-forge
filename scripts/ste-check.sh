#!/usr/bin/env bash
# Lint prose a change touches, against the structural ASD-STE100 rules. The
# AGENTS.md prose rule applies when a file is touched, so this checks each
# changed file in full rather than the diff alone. .md/.mdx files are linted
# whole. Source files are linted through their comments by ste-comments.py.
#
# Usage:
#   scripts/ste-check.sh [BASE]        # files changed vs BASE...HEAD (default: origin/main)
#   scripts/ste-check.sh --files F...  # an explicit file list (the pre-commit hook)
#
# Vendored trees are excluded: upstream prose is not ours to rewrite.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

# ste-lint.py is vendored — upstream prose, not ours to rewrite.
EXCLUDE='^(vendor/|hosts/.*/vendor/|target/|node_modules/|site/node_modules/|scripts/ste-lint\.py$|.*(LICENSE|OFL|COPYING|NOTICE)[^/]*$)'

if [ "${1:-}" = "--files" ]; then
    shift
    files=("$@")
else
    base="${1:-origin/main}"
    mapfile -t files < <(git diff --name-only --diff-filter=ACM "${base}...HEAD")
    mapfile -t untracked < <(git ls-files --others --exclude-standard)
    files+=("${untracked[@]}")
fi

targets=()
for f in "${files[@]}"; do
    [[ "$f" =~ $EXCLUDE ]] && continue
    [ -f "$f" ] && targets+=("$f")
done

if [ ${#targets[@]} -eq 0 ]; then
    echo "ste-check: no prose files changed"
    exit 0
fi

echo "ste-check: linting ${#targets[@]} file(s): ${targets[*]}"
exec python3 scripts/ste-comments.py "${targets[@]}"
