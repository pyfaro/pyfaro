# Installation

Pyfaro ships as one native executable. Install it with Cargo or from a
prebuilt archive, then make sure [uv](#install-uv) is available.

!!! note "Alpha release"
    The latest published version is `0.3.0-alpha.1`. Releases are unsigned.

## Install Pyfaro

=== "Cargo"

    With Rust 1.90 or newer:

    ```console
    $ cargo install pyfaro --version 0.3.0-alpha.1 --locked
    ```

    Cargo puts the binary in `~/.cargo/bin`, which `rustup` normally adds to
    your `PATH`.

    !!! note "The version is required for now"
        Cargo only selects pre-release versions when you name one. While every
        published Pyfaro version is an alpha, a bare `cargo install pyfaro
        --locked` fails with `could not find pyfaro in registry crates-io with
        version *`.

=== "Linux / macOS archive"

    Download the archive for your platform from
    [GitHub Releases](https://github.com/pyfaro/pyfaro/releases), together
    with `SHA256SUMS`:

    | Platform | Archive |
    | --- | --- |
    | Linux x86_64 | `pyfaro-<version>-x86_64-unknown-linux-gnu.tar.gz` |
    | Linux aarch64 | `pyfaro-<version>-aarch64-unknown-linux-gnu.tar.gz` |
    | macOS Apple silicon | `pyfaro-<version>-aarch64-apple-darwin.tar.gz` |
    | macOS Intel | `pyfaro-<version>-x86_64-apple-darwin.tar.gz` |

    Verify the checksum, then extract and install:

    ```console
    $ VERSION=0.3.0-alpha.1
    $ FILE=pyfaro-$VERSION-x86_64-unknown-linux-gnu.tar.gz
    $ grep " $FILE\$" SHA256SUMS | sha256sum -c -
    pyfaro-0.3.0-alpha.1-x86_64-unknown-linux-gnu.tar.gz: OK
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
        `xattr -d com.apple.quarantine pyfaro`.

=== "Windows archive"

    Download `pyfaro-<version>-x86_64-pc-windows-msvc.zip` and `SHA256SUMS`
    from [GitHub Releases](https://github.com/pyfaro/pyfaro/releases), then
    in PowerShell:

    ```powershell
    $Version = "0.3.0-alpha.1"
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

## Install uv

Pyfaro delegates dependency and environment work to uv and requires **uv
0.9.26 or newer**. It never installs uv for you.

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

## Verify the installation

```console
$ pyfaro --version
pyfaro 0.3.0-alpha.1
$ pyfaro tools uv
uv: /home/you/.local/bin/uv (uv 0.9.26)
```

`pyfaro tools uv` fails with exit code 1 when uv is missing or older than
0.9.26. See [`pyfaro tools`](../commands/tools.md).

## Upgrade

=== "Cargo"

    ```console
    $ cargo install pyfaro --version <new-version> --locked --force
    ```

=== "Archive"

    Download the new archive, verify it and replace the executable.

## Uninstall

=== "Cargo"

    ```console
    $ cargo uninstall pyfaro
    ```

=== "Archive"

    Delete the `pyfaro` executable you copied onto your `PATH`.

Pyfaro keeps no global state. Your projects, `uv.lock` files and `.venv`
directories are untouched.

## Next steps

- Take the [five-minute tour](first-steps.md).
- Read about [platform support](../concepts/platform-support.md).
- See the [tested tool versions](../reference/tested-versions.md).
