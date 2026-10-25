#!/usr/bin/env python3
"""Record the terminal demo of Pyfaro as an asciicast (version 2).

A development tool, not part of Pyfaro: it uses only the Python standard
library and is never packaged. It drives a real interactive bash in a real
pseudo-terminal, types the commands of the demo, waits for their real output,
and records everything the terminal prints, with the timestamps of that run.
Every command runs the real `pyfaro` binary against the real uv, Ruff, ty,
pytest and Uvicorn, so the recording is regenerated for each release.

Two things are staged and nothing else:

* Typing is simulated: the keys are written to the terminal at a steady pace.
* The edit that makes the dev server reload is made by this script, as an
  editor in another window would, while `pyfaro dev` runs. Ctrl-C is a real
  ETX byte written to the terminal, so the terminal itself sends the signal.

Long waits (downloads, installs) are shortened to at most `--max-gap` seconds
in the file. See docs/releasing.md for how to regenerate the demo.
"""

import argparse
import codecs
import fcntl
import json
import os
import pty
import re
import select
import shutil
import socket
import struct
import subprocess
import sys
import tempfile
import termios
import time
from pathlib import Path

COLUMNS = 112
ROWS = 30
TERMTOSVG = "termtosvg==1.1.0"
TITLE = "Pyfaro terminal demo"
DESCRIPTION = (
    "A terminal session: pyfaro create fastapi, sync, check, test, then dev, "
    "which reloads after an edit and stops on Ctrl-C."
)
# The bright colors 10 to 14 of termtosvg's window_frame template are nearly
# as dark as its background, which hides bold green text such as Ruff's
# "All checks passed!". They get the hue of their normal color instead.
BRIGHT_COLORS = {
    10: "#a6e22e",
    11: "#f4bf75",
    12: "#66d9ef",
    13: "#ae81ff",
    14: "#a1efe4",
}
TYPING_DELAY = 0.07
READING_PAUSE = 1.6
PROMPT = re.compile(r"\n\$ $")
RELOAD_EDIT = ('"message": "Hello from acme-api!"', '"message": "Hello, Pyfaro!"')


class Terminal:
    """A bash in a pseudo-terminal whose output is recorded with timestamps."""

    def __init__(self, directory, env):
        self.events = []
        self.buffer = ""
        self.decoder = codecs.getincrementaldecoder("utf-8")("replace")
        self.start = time.monotonic()
        pid, self.master = pty.fork()
        if pid == 0:
            os.chdir(directory)
            os.execvpe("bash", ["bash", "--noprofile", "--norc", "-i"], env)
        self.pid = pid
        size = struct.pack("HHHH", ROWS, COLUMNS, 0, 0)
        fcntl.ioctl(self.master, termios.TIOCSWINSZ, size)

    def pump(self, timeout):
        """Read what the terminal printed, waiting up to `timeout` seconds."""
        ready, _, _ = select.select([self.master], [], [], timeout)
        if not ready:
            return
        try:
            data = os.read(self.master, 65536)
        except OSError:
            return
        text = self.decoder.decode(data)
        if text:
            self.events.append((time.monotonic() - self.start, text))
            self.buffer += text

    def wait_for(self, pattern, timeout=300, count=1):
        """Block until `pattern` matched `count` times in the new output."""
        regex = re.compile(pattern)
        deadline = time.monotonic() + timeout
        while len(regex.findall(self.buffer)) < count:
            if time.monotonic() > deadline:
                tail = self.buffer[-2000:]
                raise SystemExit(
                    f"timed out waiting for {pattern!r}; last output:\n{tail}"
                )
            self.pump(0.05)

    def pause(self, seconds):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            self.pump(0.05)

    def send(self, data):
        os.write(self.master, data.encode())

    def type(self, text):
        for character in text:
            self.send(character)
            self.pause(TYPING_DELAY)

    def run(self, command, timeout=300):
        """Type `command`, press Enter and wait for the next prompt."""
        self.pause(READING_PAUSE)
        self.type(command)
        self.pause(0.4)
        self.buffer = ""
        self.send("\r")
        self.wait_for(PROMPT.pattern, timeout)

    def close(self):
        self.send("\x04")
        try:
            os.waitpid(self.pid, 0)
        except ChildProcessError:
            pass


def shorten(events, max_gap):
    """Return events with every gap between them at most `max_gap` seconds."""
    shortened, clock, previous = [], 0.0, 0.0
    for stamp, text in events:
        clock += min(stamp - previous, max_gap)
        previous = stamp
        shortened.append((clock, text))
    return shortened


def write_cast(path, events, title):
    header = {
        "version": 2,
        "width": COLUMNS,
        "height": ROWS,
        "timestamp": int(time.time()),
        "title": title,
        "env": {"SHELL": "/bin/bash", "TERM": "xterm-256color"},
    }
    with open(path, "w", encoding="utf-8") as cast:
        cast.write(json.dumps(header) + "\n")
        cast.writelines(
            json.dumps([round(stamp, 3), "o", text]) + "\n" for stamp, text in events
        )


def render_svg(cast, svg):
    """Render the asciicast as a CSS-animated SVG, with a title for screen readers."""
    subprocess.run(
        [
            "uv",
            "tool",
            "run",
            "--from",
            TERMTOSVG,
            "termtosvg",
            "render",
            str(cast),
            str(svg),
            "-t",
            "window_frame",
        ],
        check=True,
    )
    text = svg.read_text(encoding="utf-8")
    for number, color in BRIGHT_COLORS.items():
        text, count = re.subn(
            rf"(\.color{number} \{{fill: *)#[0-9a-fA-F]+", rf"\g<1>{color}", text
        )
        if count != 1:
            raise SystemExit(
                f"termtosvg's template changed: no single .color{number} rule"
            )
    opening = re.search(r"<svg[^>]*>", text)
    labels = f"<title>{TITLE}</title><desc>{DESCRIPTION}</desc>"
    svg.write_text(
        text[: opening.end()] + labels + text[opening.end() :], encoding="utf-8"
    )


def install_release(version, directory):
    """Install `pyfaro==version` from PyPI into `directory` and return its bin."""
    env = dict(
        os.environ,
        UV_TOOL_DIR=str(directory / "tools"),
        UV_TOOL_BIN_DIR=str(directory / "bin"),
    )
    subprocess.run(["uv", "tool", "install", f"pyfaro=={version}"], env=env, check=True)
    return directory / "bin"


def require_free_port(port):
    """`pyfaro dev` refuses to start when its port is taken: fail early."""
    with socket.socket() as probe:
        try:
            probe.bind(("127.0.0.1", port))
        except OSError as error:
            raise SystemExit(
                f"port {port} is in use ({error}); the demo runs `pyfaro dev` on it"
            )


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument(
        "--release", metavar="VERSION", help="install this PyPI version, e.g. 0.5.0b1"
    )
    source.add_argument(
        "--pyfaro", metavar="PATH", type=Path, help="use this pyfaro executable"
    )
    parser.add_argument(
        "--workdir",
        type=Path,
        help="new directory to record in, shown in the output (default: a temporary one)",
    )
    parser.add_argument(
        "--out", type=Path, default=Path("demo.cast"), help="asciicast file to write"
    )
    parser.add_argument(
        "--svg",
        type=Path,
        help="also render the recording to this animated SVG (needs uv)",
    )
    parser.add_argument(
        "--max-gap", type=float, default=2.0, help="longest pause kept, in seconds"
    )
    args = parser.parse_args()

    require_free_port(8000)
    if args.workdir:
        work = args.workdir.resolve()
        if work.exists():
            raise SystemExit(f"{work} exists; --workdir must name a new directory")
        work.mkdir(parents=True)
    else:
        work = Path(tempfile.mkdtemp(prefix="pf"))
    try:
        if args.release:
            bin_dir = install_release(args.release, work)
        else:
            if not args.pyfaro.is_file():
                raise SystemExit(f"{args.pyfaro} is not a file")
            bin_dir = work / "bin"
            bin_dir.mkdir()
            shutil.copy2(args.pyfaro, bin_dir / "pyfaro")
        env = {
            "PATH": f"{bin_dir}{os.pathsep}{os.environ['PATH']}",
            "HOME": str(work),
            "TERM": "xterm-256color",
            "LANG": "C.UTF-8",
            "PS1": "$ ",
            "INPUTRC": str(work / "inputrc"),
            "UV_LINK_MODE": "copy",
            "UV_CACHE_DIR": os.environ.get(
                "UV_CACHE_DIR", str(Path.home() / ".cache" / "uv")
            ),
        }
        (work / "inputrc").write_text("set enable-bracketed-paste off\n")
        version = subprocess.run(
            ["pyfaro", "--version"], env=env, capture_output=True, text=True, check=True
        ).stdout.strip()
        print(f"recording {version} in {work}", file=sys.stderr)

        terminal = Terminal(work, env)
        terminal.wait_for(PROMPT.pattern.replace("\\n", ""))
        terminal.run("pyfaro create fastapi acme-api")
        terminal.run("cd acme-api")
        terminal.run("pyfaro sync")
        terminal.run("pyfaro check")
        terminal.run("pyfaro test")

        terminal.pause(READING_PAUSE)
        terminal.type("pyfaro dev")
        terminal.pause(0.4)
        terminal.buffer = ""
        terminal.send("\r")
        terminal.wait_for(r"Application startup complete")
        terminal.pause(2.5)
        main_py = work / "acme-api" / "src" / "acme_api" / "main.py"
        source_text = main_py.read_text()
        if RELOAD_EDIT[0] not in source_text:
            raise SystemExit(f"{main_py} no longer contains {RELOAD_EDIT[0]}")
        main_py.write_text(source_text.replace(*RELOAD_EDIT))
        terminal.wait_for(r"Application startup complete", count=2)
        terminal.pause(2.5)
        terminal.send("\x03")
        terminal.wait_for(PROMPT.pattern, timeout=30)
        terminal.run("echo $?")
        terminal.pause(2.5)
        terminal.close()

        events = shorten(terminal.events, args.max_gap)
        write_cast(args.out, events, f"Pyfaro {version}")
        print(f"wrote {args.out}: {events[-1][0]:.1f} s", file=sys.stderr)
        if args.svg:
            render_svg(args.out, args.svg)
            print(f"wrote {args.svg}: {args.svg.stat().st_size} bytes", file=sys.stderr)
    finally:
        shutil.rmtree(work, ignore_errors=True)


if __name__ == "__main__":
    main()
