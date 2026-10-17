# Dogfood Fixture

This folder is a small end-to-end fixture for the compiled `replace` executable.
The integration test copies `input/` to a temporary working directory, runs:

```sh
replace --config-path replace.toml run
replace apply
```

Then it compares the resulting files against `expected/`.

The fixture intentionally uses nested folders and mixed extensions so config-driven
runs exercise traversal and staging across multiple files.
