---
description: "Install Pyfaro on Linux, macOS or Windows with the install script, uv, pipx, pip, Cargo or a prebuilt binary, and verify it."
---

# Installation

Pyfaro ships as one native executable. Install it with the install script,
from PyPI with `uv tool install`, with Cargo or from a prebuilt archive,
then make sure [uv](#install-uv) is available.
Pyfaro never installs uv for you.

!!! note "Beta release"
    The latest published version is `0.5.0-beta.1`. Every release is a
    prerelease, which is why the install script looks up the newest release
    itself and Cargo needs an explicit `--version`.

## Supported platforms

| Platform | Install script | Prebuilt archive |
| --- | --- | --- |
| Linux x86_64 (glibc) | `install.sh` | `x86_64-unknown-linux-gnu.tar.gz` |
| Linux aarch64 (glibc) | `install.sh` | `aarch64-unknown-linux-gnu.tar.gz` |
| macOS Apple silicon | `install.sh` | `aarch64-apple-darwin.tar.gz` |
| macOS Intel | `install.sh` | `x86_64-apple-darwin.tar.gz` |
| Windows x86_64 | `install.ps1` | `x86_64-pc-windows-msvc.zip` |

Other platforms, such as musl-based Linux (Alpine) or Windows on ARM, can
install with [Cargo](#install-with-cargo). See also
[platform support](../concepts/platform-support.md).

## Install with the script

The scripts are served by this site and do the same thing on every platform:
find the newest release, download the archive for your machine, verify it
and install the binary.

=== "Linux / macOS"

    ```console
    $ curl --proto '=https' --tlsv1.2 -LsSf https://pyfaro.dev/install.sh | sh
    ```

=== "Windows (PowerShell)"

    ```powershell
    powershell -ExecutionPolicy ByPass -c "irm https://pyfaro.dev/install.ps1 | iex"
    ```

    Works in Windows PowerShell 5.1 and PowerShell 7.

### Download, inspect, then run

Piping a script into a shell runs code you have not read. If you would rather
read it first, download it, look at it, and run the file you read:

=== "Linux / macOS"

    ```console
    $ curl --proto '=https' --tlsv1.2 -LsSf https://pyfaro.dev/install.sh -o install.sh
    $ less install.sh
    $ sh install.sh
    ```

=== "Windows (PowerShell)"

    ```powershell
    irm https://pyfaro.dev/install.ps1 -OutFile install.ps1
    notepad install.ps1
    powershell -ExecutionPolicy ByPass -File .\install.ps1
    ```

The scripts are short and commented. Both are also in the
[repository](https://github.com/pyfaro/pyfaro/tree/main/website/docs).

### What the script does

1. Detects your operating system and CPU and picks the matching release
   archive, or stops with a clear message if there is none.
2. Looks up the newest GitHub release, prereleases included, or uses
   `PYFARO_VERSION`.
3. Downloads the archive and `SHA256SUMS` over HTTPS only (TLS 1.2 or newer).
4. Verifies the archive's SHA-256 against `SHA256SUMS`. If the checksum is
   missing, does not match, or the machine has no tool to compute it
   (`sha256sum`, `shasum` or `openssl`; `Get-FileHash` on Windows), it stops
   and installs nothing.
5. Extracts to a temporary directory and installs the binary to
   `~/.local/bin` (Windows: `%USERPROFILE%\.local\bin`) or the directory you
   name, replacing any previous copy.
6. Prints the version, target and destination, runs `pyfaro --version`, and
   tells you if uv is missing.

What it does **not** do: it never uses `sudo`, never edits your shell profile,
the registry or `PATH`, and never installs uv. When the install directory is
not on your `PATH`, it prints the exact line to add. The whole script is
wrapped in a function that runs on the last line, so a download that is cut
short cannot run half an installer.

!!! info "What the checksum proves"
    `SHA256SUMS` is downloaded from the same release as the archive. The check
    catches a corrupt or altered download, or a bad mirror. It cannot tell you
    who published the release; the archives are unsigned.

### Options

Set these as environment variables before running the script.

| Variable | Meaning |
| --- | --- |
| `PYFARO_VERSION` | Install this version, for example `0.3.0-alpha.1`, instead of the newest. Also how you pin or downgrade. |
| `PYFARO_INSTALL_DIR` | Directory for the binary. Default: `~/.local/bin` (Windows: `%USERPROFILE%\.local\bin`). |
| `GITHUB_TOKEN` | Optional. Sent only to `api.github.com` when looking up the newest release, to avoid anonymous rate limits (for example in CI). |
| `PYFARO_DOWNLOAD_BASE` | For testing mirrors only. Replaces `https://github.com/pyfaro/pyfaro/releases/download`; files are read from `<base>/v<version>/`. Must start with `https://`, or `file://` for a local directory. The checksum is still verified. |

=== "Linux / macOS"

    ```console
    $ curl --proto '=https' --tlsv1.2 -LsSf https://pyfaro.dev/install.sh |
    >   PYFARO_VERSION=0.3.0-alpha.1 PYFARO_INSTALL_DIR=/opt/tools/bin sh
    ```

=== "Windows (PowerShell)"

    ```powershell
    $env:PYFARO_VERSION = "0.3.0-alpha.1"
    $env:PYFARO_INSTALL_DIR = "C:\tools\bin"
    irm https://pyfaro.dev/install.ps1 | iex
    ```

### Put it on your PATH

If the install directory is not on your `PATH`, the script prints how to add
it. On Linux and macOS, add this line to your shell profile (`~/.profile`,
`~/.bashrc` or `~/.zshrc`):

```sh
export PATH="$HOME/.local/bin:$PATH"
```

On Windows, add the directory to your user `PATH` (Settings, System, About,
Advanced system settings, Environment Variables), or run the
`SetEnvironmentVariable` line the script prints.

## Install from PyPI

If you already use Python tools, install Pyfaro like any other command-line
tool. The wheels hold only the `pyfaro` executable, so there is no Rust
toolchain and no Python code to import. They are published from `0.5.0b1`
(Pyfaro's `0.5.0-beta.1`; Python spells pre-releases `a`, `b` and `rc`).

=== "uv"

    ```console
    $ uv tool install pyfaro
    ```

=== "pipx"

    ```console
    $ pipx install pyfaro
    ```

=== "pip"

    ```console
    $ pip install pyfaro
    ```

    This installs into the active environment. Prefer `uv tool install` or
    `pipx` to keep Pyfaro out of your project environments.

Wheels exist for Linux x86_64 and aarch64 (glibc 2.17 or newer, so also older
distributions than the archives support), macOS Apple silicon and Intel, and
Windows x86_64. Other platforms build from the source distribution, which
needs Rust 1.90 or newer.

!!! note "Pre-releases"
    Every published Pyfaro version is a pre-release for now. When a package
    has no stable version, uv and pip fall back to the newest pre-release, so
    the commands above work as written. To install an exact version, name it:
    `uv tool install pyfaro==0.5.0b1`. After a stable release exists, the
    plain command installs that one; add `--prerelease allow` (uv) or `--pre`
    (pip, or `pipx install --pip-args=--pre pyfaro`) to keep following the
    pre-releases.

The executable lands in the tool bin directory (`~/.local/bin`). If your
shell cannot find `pyfaro`, run `uv tool update-shell` or `pipx ensurepath`
and open a new terminal.

## Install with Cargo

With Rust 1.90 or newer:

```console
$ cargo install pyfaro --version 0.5.0-beta.1 --locked
```

Cargo puts the binary in `~/.cargo/bin`, which `rustup` normally adds to your
`PATH`.

!!! note "The version is required for now"
    Cargo only selects pre-release versions when you name one. While every
    published Pyfaro version is a pre-release, a bare `cargo install pyfaro
    --locked` fails with `could not find pyfaro in registry crates-io with
    version *`.

## Install from an archive

Prefer to do it by hand? Download the archive for your platform from
[GitHub Releases](https://github.com/pyfaro/pyfaro/releases), together with
`SHA256SUMS`.

=== "Linux / macOS"

    Archives are named `pyfaro-<version>-<target>.tar.gz`, with the targets
    from the [table above](#supported-platforms). Verify the checksum, then
    extract and install:

    ```console
    $ VERSION=0.5.0-beta.1
    $ FILE=pyfaro-$VERSION-x86_64-unknown-linux-gnu.tar.gz
    $ grep " $FILE\$" SHA256SUMS | sha256sum -c -
    pyfaro-0.5.0-beta.1-x86_64-unknown-linux-gnu.tar.gz: OK
    $ tar -xzf $FILE
    $ install -m 755 pyfaro-$VERSION-x86_64-unknown-linux-gnu/pyfaro ~/.local/bin/
    ```

    On macOS, `sha256sum` is not installed by default; use
    `shasum -a 256 -c -` instead.

    ```console
    $ grep " $FILE\$" SHA256SUMS | shasum -a 256 -c -
    ```

    !!! warning "Unsigned binaries on macOS"
        The archives are not signed or notarized. If Gatekeeper blocks the
        binary, remove the quarantine attribute from the extracted file with
        `xattr -d com.apple.quarantine pyfaro`. The install script avoids
        this because `curl` does not set the attribute.

=== "Windows"

    Download `pyfaro-<version>-x86_64-pc-windows-msvc.zip`, then in
    PowerShell:

    ```powershell
    $Version = "0.5.0-beta.1"
    $File = "pyfaro-$Version-x86_64-pc-windows-msvc.zip"
    $Expected = (Select-String -Path SHA256SUMS -Pattern " $File$").Line.Split(" ")[0]
    $Actual = (Get-FileHash $File -Algorithm SHA256).Hash.ToLower()
    if ($Actual -eq $Expected) { "OK" } else { "MISMATCH" }
    Expand-Archive $File -DestinationPath .
    ```

    Move `pyfaro.exe` from the extracted `pyfaro-<version>-x86_64-pc-windows-msvc`
    folder to a directory on your `PATH`.

Each archive contains the `pyfaro` executable, `README.md`, `LICENSE` and
`CHANGELOG.md`.

!!! tip "Always verify"
    `SHA256SUMS` lists every asset of the release. A mismatch means the
    download is corrupt or has been tampered with: delete it and download
    again.

### Verify provenance (optional)

Releases from `0.5.0-beta.1` on carry GitHub build-provenance attestations for
every archive, wheel and the `.crate`. They show which workflow run built a file, which
the checksum alone cannot. With the [GitHub CLI](https://cli.github.com/),
verify a downloaded archive before you extract it:

```console
$ gh attestation verify pyfaro-<version>-x86_64-unknown-linux-gnu.tar.gz --repo pyfaro/pyfaro
```

Earlier releases, including `0.4.0-alpha.1`, have no attestations. The install
script does not run this check, since it would make the GitHub CLI a
requirement. See the
[release guide](https://github.com/pyfaro/pyfaro/blob/main/docs/releasing.md)
for how releases are built and reproduced.

## Tab completion

`pyfaro completions <SHELL>` prints a completion script for Bash, Zsh, Fish or
PowerShell. Where to save it is in the [`completions` guide](../commands/completions.md);
run it again after an upgrade to pick up new commands.

## Install uv

Pyfaro delegates dependency and environment work to uv and requires **uv
0.9.26 or newer**. **Pyfaro never installs uv for you**, and neither does the
install script: it only tells you when uv is missing.

=== "Linux / macOS"

    ```console
    $ curl -LsSf https://astral.sh/uv/install.sh | sh
    ```

=== "Windows"

    ```powershell
    powershell -ExecutionPolicy ByPass -c "irm https://astral.sh/uv/install.ps1 | iex"
    ```

Other installation methods are covered in the
[uv documentation](https://docs.astral.sh/uv/getting-started/installation/).
Pyfaro finds `uv` on your `PATH`; use the `--uv` option on a command to
select another executable.

If uv is missing, too old or broken, [`pyfaro doctor`](../commands/doctor.md)
and every command that needs uv say so in one line and tell you what to do:
install uv, run `uv self update` (or reinstall uv) when it is older than
0.9.26, or pass `--uv <path>`. If it is unclear which uv Pyfaro picked,
[`pyfaro -v`](../workflows/troubleshooting.md#verbose-output) prints the path
and version it uses before it runs anything.

## Verify the installation

```console
$ pyfaro --version
pyfaro 0.5.0-beta.1
$ pyfaro tools uv
uv: /home/you/.local/bin/uv (uv 0.9.26)
```

`pyfaro tools uv` fails with exit code 1 when uv is missing or older than
0.9.26. `pyfaro doctor` checks the rest of the toolchain; see
[`pyfaro tools`](../commands/tools.md).

## Upgrade

=== "Install script"

    Run the script again. It installs the newest release over the old binary
    and says which version it replaced. Set `PYFARO_VERSION` to move to a
    specific version, including an older one.

=== "uv / pipx"

    ```console
    $ uv tool upgrade pyfaro
    $ pipx upgrade pyfaro
    ```

    Add `--prerelease allow` (uv) once a stable version exists and you want
    to stay on pre-releases.

=== "Cargo"

    ```console
    $ cargo install pyfaro --version <new-version> --locked --force
    ```

=== "Archive"

    Download the new archive, verify it and replace the executable.

## Uninstall

=== "Install script / archive"

    Delete the executable: `~/.local/bin/pyfaro` on Linux and macOS,
    `%USERPROFILE%\.local\bin\pyfaro.exe` on Windows, or wherever you put it
    (`PYFARO_INSTALL_DIR`). The script changed nothing else on your machine.
    If you added the install directory to your `PATH` yourself and nothing
    else uses it, remove that line too.

=== "uv / pipx"

    ```console
    $ uv tool uninstall pyfaro
    $ pipx uninstall pyfaro
    ```

=== "Cargo"

    ```console
    $ cargo uninstall pyfaro
    ```

Pyfaro keeps no global state. Your projects, `uv.lock` files and `.venv`
directories are untouched, and uv stays installed.

## If the install fails

| Message | What to do |
| --- | --- |
| `checksum mismatch … nothing was installed` | The download is corrupt or has been tampered with. Run the script again; if it repeats, report it. |
| `no SHA-256 tool found` | Install `sha256sum`, `shasum` or `openssl`. The script refuses to install what it cannot verify. |
| `could not look up the newest release` | GitHub rate-limited the lookup. Set `PYFARO_VERSION`, or `GITHUB_TOKEN`. |
| `musl-based Linux is not supported` | The archives need glibc. Use Cargo. |
| `unsupported CPU architecture` | There is no prebuilt archive for it. Use Cargo. |
| `neither curl nor wget found` | Install one of them. |
| `could not write … (this script never uses sudo …)` | Set `PYFARO_INSTALL_DIR` to a directory you own. |
| `HOME is not set` | Set `PYFARO_INSTALL_DIR` to the directory to install into. |
| `invalid version: …` | `PYFARO_VERSION` may only contain letters, digits, `.`, `+` and `-`. |

## Next steps

- Take the [five-minute tour](first-steps.md).
- Set up [tab completion](../commands/completions.md).
- Read about [platform support](../concepts/platform-support.md).
- See the [tested tool versions](../reference/tested-versions.md).
