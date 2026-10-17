---
name: replace
description: Use when renaming identifiers, text or file names across many files, running a codemod, or undoing a bulk edit, and the `replace` command is installed.
---

# replace

Rewrites are staged first. Nothing on disk changes until `apply`.

## Loop

```bash
replace --json FIND REPLACE [PATHS...]   # stage; read stages[].id, file, replacements
replace show ID                          # review the diff
replace --json apply [ID]                # all stages when ID is omitted
replace --json -y undo ID                # revert an applied change
```

Skip review: `replace --json -y FIND REPLACE`.

## Matchers

| Need                      | Command                                             |
| ------------------------- | --------------------------------------------------- |
| Exact text                | `replace FIND REPLACE` or `replace literal ...`     |
| Pattern                   | `replace regex PATTERN REPLACE` (`-i` ignores case) |
| Code structure            | `replace ast --lang LANG 'foo($A)' 'bar($A)'`       |

`literal` and `regex` also rename matching file and folder names; `--skip-paths` keeps them.
`ast` never renames.

Scope: `--include GLOB`, `--exclude GLOB`, `--hidden`, `--no-ignore`, `--no-default-ignores`,
`--max-files N`

## Contract

- With `--json`, stdout is one document matching `schema/replace.output.json`. Warnings and
  notes go to stderr. `list` and `log` print arrays.
- No prompt without a terminal. `discard`, `undo` and `gc` need `-y`, else exit 2.
- Zero matches exits 0 with `REPLACE_W_ZERO_MATCHES`. `--strict` makes any warning exit 1.

| Exit | Meaning                         | Action                                   |
| ---- | ------------------------------- | ---------------------------------------- |
| 2    | Bad arguments or unknown stage  | Fix the call                             |
| 4    | AST pattern or parse error      | Fix the pattern                          |
| 5    | Drift or two stages on one file | Apply one ID, or `discard`, then restage |
| 6    | Apply failed and rolled back    | Files are unchanged; rerun               |
| 8    | Lock held                       | Retry, or raise `--lock-timeout`         |

A file edited after staging drifts. Restage it.

Flags go before or after the command.

## References

- `references/commands.md`: `list`, `check`, `explain`, `gc`, `status`, `run`, `from-error`, and
  the flags beyond the loop.
- `references/anchors.md`: applying part of a stage, and where it falls short.
- `docs/config.md`: `.replace/config.toml` rules and hooks for `replace run`.
