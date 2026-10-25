# Shell completion

`pyfaro completions <SHELL>` prints a static tab-completion script on standard
output for `bash`, `zsh`, `fish` or `powershell` and exits 0. It needs no
project and no uv, and writes nothing to disk. A missing or unknown shell is a
Clap usage error (exit 2). Elvish is deliberately not offered.

```sh
pyfaro completions bash > ~/.local/share/bash-completion/completions/pyfaro
mkdir -p ~/.zfunc && pyfaro completions zsh > ~/.zfunc/_pyfaro
pyfaro completions fish > ~/.config/fish/completions/pyfaro.fish
pyfaro completions powershell | Out-String | Invoke-Expression
```

The script comes from `clap_complete` and is generated from the same `Cli`
definition as `--help`, so a new command or flag is completed without extra
work. Completion is static: it does not read `pyproject.toml`, so task names
are not completed.

For Zsh, put `_pyfaro` in a directory on `fpath` before `compinit` runs. For
Bash with the `bash-completion` package, use the per-user
`bash-completion/completions` directory, or `source <(pyfaro completions
bash)` from `~/.bashrc`. Fish loads `~/.config/fish/completions` on demand.
PowerShell loads it from the profile. After upgrading Pyfaro, regenerate any
saved script; in Zsh also remove the `~/.zcompdump*` cache.

## Help text

`pyfaro --help` ends with a short workflow and a link to the website. The
subcommands `create`, `sync`, `add`, `run`, `dev`, `test`, `check`, `python`
and `completions` end with an `Examples:` section. These texts live in
`src/cli/help.rs`. A unit test parses every indented line that starts with
`pyfaro ` with the real argument definitions, so an example that stops being
valid fails the build; keep examples free of shell quoting and of trailing
text other than a ` # comment`.

## Argument documentation

The same flag has the same help text everywhere:

| Argument | Wording |
| --- | --- |
| `--project` | Explicit project directory; otherwise discover the nearest project |
| `--uv` | uv executable (a path or a name found on PATH) |
| `--offline` | Disable network access in uv |
| `--locked` | Refuse to update an absent or stale uv.lock |
| `--frozen` | Use uv.lock without checking it against project metadata |
| `-v`, `--verbose` | Print tool versions and every command line before it runs |

`-v`/`--verbose` is flattened into every subcommand except `test` through
`VerboseArgs`, and the completion scripts offer it everywhere it is accepted.
`python --project` says how it differs: the directory is exact and not
discovered. Switches start with an imperative verb, values with a noun phrase;
defaults are written `(defaults to ...)` and sentences have no trailing period.

## Tests

The e2e suite (`tests/e2e/completions.rs`) runs the real binary for every shell
and checks the script with the real shell: `bash -n`, `zsh -n`,
`fish --no-execute`, and the PowerShell parser on Windows. For Bash it also
sources the script and completes `pyfaro sy` to `sync`. The tests fail, not
skip, when `zsh` or `fish` is missing; the CI e2e job installs both.
