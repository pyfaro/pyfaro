# Verbose output and redaction

## `--verbose`

`-v` / `--verbose` makes Pyfaro explain what it is about to run. It is accepted
before the subcommand for every command and after it for every command except
`test`:

```sh
pyfaro -v sync
pyfaro sync --verbose
pyfaro dev -v
pyfaro -v test          # Pyfaro's trace; pytest is not verbose
pyfaro test -v          # no trace; -v goes to pytest
```

`pyfaro test` forwards everything after `test` other than `--project` to
pytest verbatim, `-v` and `--verbose` included, as documented in the
[test guide](test.md#argument-forwarding).

Without the flag, output is exactly what it was before: Pyfaro adds nothing.
With it, Pyfaro writes lines to standard error before it launches anything:

```text
pyfaro lint: running /work/demo/.venv/bin/ruff --version
pyfaro lint: using ruff 0.16.10 at /work/demo/.venv/bin/ruff
pyfaro lint: running /work/demo/.venv/bin/ruff check
```

- `pyfaro <command>: using <version> at <path>`: each external tool Pyfaro
  resolved, with the version it reported. uv is always listed (for the
  commands that use it); `lint` and `fmt` list Ruff, `type` ty, `test` pytest
  and `check` Ruff and ty. The probes are the ones `pyfaro tools` and
  `pyfaro doctor` use.
- `pyfaro <command>: running <command line>`: every process Pyfaro executes,
  including uv's version probes, the uv command lines, task commands and each
  step of `check`. Words that contain spaces or quotes are quoted.
- `pyfaro <command>: working directory <path>`: when a process does not run in
  the project root, for example `pyfaro run -- <program>` from a
  subdirectory.
- `pyfaro <command>: environment variables set: A, B`: the **names** of
  variables Pyfaro sets for a process, such as a task's `env`. Values are
  never printed.

The child's own output is not changed or prefixed.

## Redaction

Pyfaro never prints the secret part of a URL's userinfo. In verbose lines, in
`error:` messages (including text taken from tool diagnostics and
configuration values that Pyfaro echoes), in Clap's usage errors and in the
`doctor` report:

```text
https://user:secret@host/simple   ->   https://user:****@host/simple
https://token@host/simple         ->   https://****@host/simple
```

This applies to any argument that is a URL and to `--flag=URL`. Other
arguments are printed as given, so do not pass a secret as a plain argument.

Not covered: output that a child process writes straight to the terminal (uv,
Ruff, ty, pytest and your own programs and tasks). Pyfaro does not read or
rewrite it; uv itself masks the passwords of URLs it prints. A TOML syntax
error quotes the offending line of `pyproject.toml`, with URL userinfo masked
but nothing else, so keep secrets out of that file.
