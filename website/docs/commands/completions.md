# pyfaro completions

`pyfaro completions <SHELL>` prints a tab-completion script for Bash, Zsh, Fish
or PowerShell on standard output. Save it where your shell loads completions
and `pyfaro <TAB>` completes subcommands, flags and fixed values such as the
shell names.

The command needs no project and no uv. It writes nothing to disk; the script
is generated from the same definitions that produce `pyfaro --help`.

## Quick example

```console
$ pyfaro completions bash > ~/.local/share/bash-completion/completions/pyfaro
```

After you open a new shell:

```console
$ pyfaro sy<TAB>
$ pyfaro sync
```

```text
pyfaro completions <SHELL>
```

`<SHELL>` is one of `bash`, `zsh`, `fish` or `powershell`. Anything else is a
usage error with exit code `2`:

```console
$ pyfaro completions tcsh
error: invalid value 'tcsh' for '<SHELL>'
  [possible values: bash, zsh, fish, powershell]

For more information, try '--help'.
```

## Install the script

The completions are static: they know the commands and flags, not the tasks of
your project.

### Bash

Bash completion needs the `bash-completion` package (or Homebrew's
`bash-completion@2`). Save the script in the per-user completions directory,
which bash-completion loads on demand:

```sh
mkdir -p ~/.local/share/bash-completion/completions
pyfaro completions bash > ~/.local/share/bash-completion/completions/pyfaro
```

Without bash-completion, load it from `~/.bashrc` instead:

```sh
source <(pyfaro completions bash)
```

### Zsh

Save the script as `_pyfaro` in a directory of your `fpath`, before
`compinit` runs in `~/.zshrc`:

```sh
mkdir -p ~/.zfunc
pyfaro completions zsh > ~/.zfunc/_pyfaro
```

```sh
# ~/.zshrc
fpath=(~/.zfunc $fpath)
autoload -Uz compinit && compinit
```

### Fish

Fish loads files from its user completions directory automatically:

```sh
mkdir -p ~/.config/fish/completions
pyfaro completions fish > ~/.config/fish/completions/pyfaro.fish
```

### PowerShell

Add this line to your profile (open it with `notepad $PROFILE`; create the file
first with `New-Item -Force $PROFILE` if it does not exist):

```powershell
pyfaro completions powershell | Out-String | Invoke-Expression
```

## Refresh after an upgrade

A saved script is a snapshot. After you upgrade Pyfaro, generate it again with
the same command to pick up new commands and flags, then start a new shell.
In Zsh also delete the completion cache first, for example
`rm -f ~/.zcompdump*`. The PowerShell line above and the Bash `source <(...)`
line regenerate the script every time the shell starts, so they need no
refresh.

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | The script was printed |
| `2` | Missing or unknown shell |

## See also

- [Command line reference](../reference/cli.md): every command and flag.
- [Exit codes](../concepts/exit-codes.md)
