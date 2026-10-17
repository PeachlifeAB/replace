# Configuration

Layers, highest precedence first.

1. `--config-path FILE`, read alone
2. The nearest project config, `<project>/.replace/config.toml`
3. The global config, `$XDG_CONFIG_HOME/replace/config.toml`; without XDG,
   `~/.replace` on macOS and `~/.config/replace` elsewhere

A key in a higher layer replaces the lower value whole, lists included.

`replace run` applies the `[[rules]]` from the merged configuration.
Use `--config-path FILE` to run a specific rules file.

Example:

```toml
root = "src"
apply_immediately = false

[[rules]]
type = "literal"
find = "foo"
replace = "bar"

[[rules]]
type = "regex"
pattern = "\\d+"
replacement = "42"

[[rules]]
type = "ast"
language = "typescript"
pattern = "var $A"
replacement = "let $A"

# Native ast-grep YAML rules are supported through `replace ast --rule`.
# A rule must define `language`, `rule`, and one `fix`; `constraints`,
# relational/composite rules, and pattern strictness are preserved when staged.
```

Hooks run for config-driven runs with `apply_immediately = true`, and not otherwise:

```toml
apply_immediately = true

[hooks]
pre_apply = ["cargo test --locked"]
post_apply = ["cargo fmt --all"]
```
