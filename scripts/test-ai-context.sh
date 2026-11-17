#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/repo with spaces/scripts" "$fixture/repo with spaces/docs/architecture" "$fixture/repo with spaces/tasks"
root="$fixture/repo with spaces"
cp "$repo_root/scripts/ai-context.sh" "$root/scripts/ai-context.sh"
files=(AGENTS.md README.md docs/project-context.md docs/architecture/contracts.md)
for file in "${files[@]}"; do
  printf 'fixture: %s\n' "$file" > "$root/$file"
done
printf 'task marker\n' > "$root/tasks/task with spaces.md"
printf 'log marker\n' > "$root/docs/ai-log.md"
cd "$fixture"
bash "$root/scripts/ai-context.sh" > "$fixture/output"
for file in "${files[@]}"; do
  grep -Fq -- "--- $file ---" "$fixture/output"
  grep -Fq "fixture: $file" "$fixture/output"
done
grep -Fq 'log marker' "$fixture/output"
for task in 'tasks/task with spaces.md' "$root/tasks/task with spaces.md"; do
  bash "$root/scripts/ai-context.sh" "$task" > "$fixture/output"
  grep -Fq 'task marker' "$fixture/output"
done
for missing in "${files[@]}" 'tasks/missing.md'; do
  if [[ "$missing" != tasks/* ]]; then
    mv "$root/$missing" "$root/$missing.saved"
  fi
  if bash "$root/scripts/ai-context.sh" "$missing" > "$fixture/output" 2> "$fixture/error"; then
    printf 'Expected failure for missing file: %s\n' "$missing" >&2
    exit 1
  fi
  test ! -s "$fixture/output"
  grep -Fq "Missing context file: $missing" "$fixture/error"
  if [[ "$missing" != tasks/* ]]; then
    mv "$root/$missing.saved" "$root/$missing"
  fi
done
rm "$root/docs/ai-log.md"
bash "$root/scripts/ai-context.sh" > "$fixture/output"
grep -Fq '# Fuzzy Music agent context' "$fixture/output"
printf 'PASS: context bundle, task paths, missing inputs, optional log\n'
