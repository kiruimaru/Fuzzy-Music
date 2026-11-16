#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
task_file="${1:-}"

files=(
  "$repo_root/AGENTS.md"
  "$repo_root/README.md"
  "$repo_root/docs/project-context.md"
  "$repo_root/docs/architecture/contracts.md"
)

if [[ -n "$task_file" ]]; then
  if [[ "$task_file" != /* ]]; then
    task_file="$repo_root/$task_file"
  fi
  files+=("$task_file")
fi

for file in "${files[@]}"; do
  if [[ ! -f "$file" ]]; then
    printf 'Missing context file: %s\n' "${file#"$repo_root"/}" >&2
    exit 1
  fi
done

printf '%s\n' '# Fuzzy Music agent context'
printf '%s\n' "Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
printf '%s\n\n' 'Use the repository files below as the working context. Source code and tests remain authoritative.'

for file in "${files[@]}"; do
  printf '\n--- %s ---\n\n' "${file#"$repo_root"/}"
  cat "$file"
  printf '\n'
done

printf '\n--- recent AI log ---\n\n'
if [[ -f "$repo_root/docs/ai-log.md" ]]; then
  sed -n '1,180p' "$repo_root/docs/ai-log.md"
fi
