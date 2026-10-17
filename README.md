# Replace

Find, replace and rename with recovery options.

```bash
replace foo bar # shows the diffs with "Y/n" prompt to confirm change
replace undo # reverts the last applied change
```

[SKILL.md](./skills/replace/SKILL.md)

Install by cloning this repo and run `cargo install --path replace`

## Commands

```bash
replace FIND REPLACE [PATHS...]
replace literal FIND REPLACE [PATHS...]
replace regex PATTERN REPLACE [PATHS...]
replace ast [--lang LANGUAGE] [--selector NODE_KIND] [--strictness LEVEL] PATTERN REWRITE [PATHS...]
replace ast --rule RULE.yml [PATHS...]
replace log
```

## Matchers

```bash
replace foo bar
replace regex 'v(\d+)' 'version $1'
replace ast --lang go 'oldName($A)' 'newName($A)'
```

`$A` binds one expression in `ast`; `$$$BODY` binds a list. `literal` and `regex`
also rename matching file and folder names; `ast` never does.

## Common

| Option                 | Description                                       |
| ---------------------- | ------------------------------------------------- |
| `--apply -y`           | Apply directly, non interactive                   |
| `--skip-paths`         | Keep matching file and folder names               |
| `--include GLOB`       | Scan files matching the glob, repeatable          |
| `--exclude GLOB`       | Skip files matching the glob, repeatable          |
| `--files0-from FILE`   | NUL-separated paths, `-` for stdin                |
| `--hidden`             | Include hidden files and folders                  |
| `--no-ignore`          | Include replacing paths in `.gitignore` `.ignore` |
| `--no-default-ignores` | Include `node_modules` and similar                |
| `--expand`             | Print the whole preview, no folding               |
| `--json`               | Machine-readable output, never prompts            |
| `--stats`              | Summary statistics                                |

## Stage lifecycle

```bash
replace list [--json]
replace show [STAGE_ID]
replace apply [STAGE_ID]
replace discard [STAGE_ID]
replace check
replace explain STAGE_ID
replace log
replace undo STAGE_ID
replace gc
```

Rewrites get staged under `.replace/stage`.

Applying all stages prevalidates the selected stages, writes atomically, and
rolls back prior writes in the batch if a later write fails. Apply operations
use a repository-local `.replace/lock` lock file. Symlink targets are
skipped/rejected.

## Apply part of a stage

```bash
replace --diff anchored show STAGE_ID
replace apply STAGE_ID --only-anchors file#a1b2
```

Anchors are the `file#hash` lines printed before each changed line. The rest
stays staged. `--skip-anchors` inverts the choice; `--expect-anchors` fails with
exit 5 if the file moved.

## Scripts and agents

```bash
replace --json foo bar
replace --json -y foo bar
replace --json list
replace --json log
replace --json -y undo STAGE_ID
```

Stdout is one JSON document matching `schema/replace.output.json`. Warnings and
notes go to stderr. Nothing prompts without a terminal; destructive commands
need `-y`. See exit codes below.

## Config and rules

See:

- `docs/config.md`
- `config.example.toml`
- `schema/replace.config.json`

## Diagnostics

`replace from-error MESSAGE_OR_FILE` stages a low-confidence candidate from
diagnostics containing a file path. It records source diagnostic metadata for
review; `from-error` stages cannot be applied directly.

### Exit codes and warnings

Skipped files (binary, too large, symlink, not UTF-8) and zero matches are
warnings: the run exits 0, prints `REPLACE_W_*` on stderr, and lists them under
`warnings` with `--json`. Errors print `REPLACE_E_*`.

| Code  | Meaning                                           |
| ----- | ------------------------------------------------- |
| 0     | Success, with or without warnings                 |
| 1     | `--strict` and a warning, or an I/O error         |
| 2     | Bad arguments or unknown stage                    |
| 4     | AST pattern or parse error                        |
| 5     | Drift or conflict: the file changed since staging |
| 6     | Apply failed and reverted its writes              |
| 8     | Lock timeout                                      |
| 9, 10 | Pre-apply or post-apply hook failed               |

## Safety

- Source SHA drift checks before apply.
- Repository-local lock around apply operations.
- Atomic file persistence via temp-file rename.
- Multi-stage apply prevalidates all selected stages and rolls back written
  files on failure.
- Symlink targets are refused.
- Stage metadata records cwd, command argv, timestamp, and git author when
  available.

## License

MIT or Apache-2.0, at your option. Unless you state otherwise, contributions are
dual licensed the same way.
