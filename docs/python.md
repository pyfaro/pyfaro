# Managing Python versions

`pyfaro python list|install|pin` delegates to uv. Install uv separately;
Pyfaro accepts stable uv versions at least `0.9.26` and tests against
`0.9.26`. It does not maintain a Python installer, download catalog, or cache.
These commands are available in published `0.2.0-alpha.1` and later versions.

```sh
pyfaro python list
pyfaro python list --only-installed 3.12
pyfaro python install 3.12
pyfaro python install
pyfaro python pin 3.12
pyfaro python pin
pyfaro python pin --rm
```

`list` shows uv's installed and available versions. An optional Python request
filters the output; `--only-installed` hides available downloads and
`--all-versions` includes older patch versions. Listing does not install
anything. `install` explicitly asks uv to install one or more requests. With
no target, uv uses `.python-version` (or `.python-versions`), if present;
otherwise uv applies its own default. This is the only Pyfaro command that
permits uv to download an interpreter. `--offline` disables network access,
including for an explicit install.

`pin <request>` writes `.python-version` through uv. `pin` without a request
displays the current pin; `pin --rm` removes it. uv validates a new pin against
the discovered project's `requires-python` range. uv also reads the pin and
`requires-python` during later project workflows. An explicit `install`
request controls which version is installed; it is not a change to the project
pin. `list` is a catalog query and is not filtered by `requires-python`.

By default, uv starts discovery in the current directory, including its
ancestors. `--project <PATH>` selects an exact directory containing a valid
`pyproject.toml` and runs uv there. This lets `pin` update that project's
`.python-version` even when invoked elsewhere. `--uv <executable>` selects uv;
`--offline` applies to all three commands. Requests are passed as separate
OS-native arguments with no shell. Use `--` before a request that starts with
a dash. Pyfaro streams uv's output and preserves its numeric exit status.

The normal `init`, `sync`, and `run` workflows continue to pass
`--no-python-downloads` to uv, preventing automatic interpreter downloads.
The command behavior above follows [uv's Python version guide](https://docs.astral.sh/uv/concepts/python-versions/)
and [uv's CLI reference](https://docs.astral.sh/uv/reference/cli/).
