# The quality gate

[`pyfaro check`](../commands/check.md) is the one command to run before you
commit. It runs the format check, the Ruff lint and the ty type check, prints
a summary, and exits non-zero if any step failed. Without `--fix` it never
writes a file.

## Check before committing

```console
$ pyfaro check
4 files already formatted
All checks passed!
All checks passed!

== pyfaro check summary ==
format: ok
lint: ok
type: ok
check: ok
```

When something fails, every step still runs, so you see all problems at once:

```console
$ pyfaro check
unformatted: File would be reformatted
  --> src/textkit/__init__.py:16:12
   |
15 | def shout(text: str) -> str:
   -     return  text.upper()+"!"
16 +     return text.upper() + "!"
   |

1 file would be reformatted, 3 files already formatted
All checks passed!
All checks passed!

== pyfaro check summary ==
format: failed (exit 1)
lint: ok
type: ok
check: failed (format)
```

The exit code is `0` when every step passed and `1` otherwise.

## Let `--fix` tidy up

```console
$ pyfaro check --fix
fix: format and lint can be fixed automatically; type has no automatic fixes
All checks passed!
fix lint: ok
1 file reformatted, 3 files left unchanged
fix format: ok
fix: rechecking without writing
4 files already formatted
All checks passed!
All checks passed!

== pyfaro check summary ==
format: ok
lint: ok
type: ok
check: ok
```

`--fix` applies Ruff's safe lint fixes and formatting first, then reruns
every check read-only. The exit code reflects that verification run. Type
errors and fixes Ruff considers unsafe are left for you.

!!! note "Review before you commit"
    `--fix` rewrites files in place. Look at `git diff` and re-stage the
    changes before committing.

## Wire it into a git pre-commit hook

A hook is a script. Create `.git/hooks/pre-commit`:

```sh
#!/bin/sh
# Block the commit when formatting, lint, or type checks fail.
exec pyfaro check
```

and make it executable:

```sh
chmod +x .git/hooks/pre-commit
```

With a fixable problem in the tree, the commit is refused (git prints the
`check` output above and creates no commit). Fix, re-stage, and commit again:

```console
$ git commit -m "add shout"
$ pyfaro check --fix
$ git add -A
$ git commit -m "add shout"
[main a5ba62e] add shout
 1 file changed, 4 insertions(+)
```

On the successful attempt the hook prints the summary shown earlier first.

!!! tip "Keep the hook cheap"
    `check` does not sync, so the hook assumes `.venv` is up to date. If a
    teammate changes dependencies, `pyfaro sync` brings you back in line.

!!! note "Hooks are local"
    `.git/hooks` is not committed. To share a hook, keep the script in the
    repository (for example `scripts/pre-commit`) and run
    `git config core.hooksPath scripts`, or rely on
    [CI](continuous-integration.md) as the shared gate. A hook can always be
    skipped with `git commit --no-verify`, so CI remains the real gate.

## Narrow the scope

`check` accepts files or directories, and `--project` selects another project:

```sh
pyfaro check src/textkit
pyfaro check --project ../other-project
```

## Next steps

- Run the gate on every push: [Continuous integration](continuous-integration.md).
- Run the pieces separately: [`lint`](../commands/lint.md),
  [`fmt`](../commands/fmt.md), [`type`](../commands/type.md).
- Missing-tool errors: [Troubleshooting](troubleshooting.md).
