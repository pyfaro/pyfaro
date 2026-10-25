#!/usr/bin/env python3
"""Reproducible release archives for Pyfaro.

`build` compiles the release binary for one target with a normalized build
environment and packs it into a byte-for-byte deterministic archive.
`compare` checks that two independent builds of every supported target
produced identical binaries and archives. `check-crate` and `check-assets`
check a `.crate` and a set of archives against a version. The release
workflows use all of them;
docs/releasing.md explains how to reproduce a published archive by hand.

Requires Python 3.11+, git, and the Rust toolchain pinned in
rust-toolchain.toml. Run it from a clean checkout of the release tag.
"""

import argparse
import gzip
import hashlib
import io
import json
import os
import shutil
import subprocess
import sys
import tarfile
import time
import tomllib
import zipfile
from pathlib import Path

# The source tree to build; --source overrides it, so a checkout of an older
# tag can be built with this script.
SOURCE = Path(__file__).resolve().parents[2]
BINARY = "pyfaro"
DOCS = ("README.md", "LICENSE", "CHANGELOG.md")
# Release targets; Windows gets a .zip, everything else a .tar.gz.
TARGETS = (
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "x86_64-pc-windows-msvc",
)
# The ZIP format cannot record times before 1980.
ZIP_MIN_EPOCH = 315532800


def fail(message):
    prefix = "::error::" if os.environ.get("GITHUB_ACTIONS") else "error: "
    print(f"{prefix}{message}", file=sys.stderr)
    sys.exit(1)


def notice(message):
    prefix = "::notice::" if os.environ.get("GITHUB_ACTIONS") else "note: "
    print(f"{prefix}{message}")


def run(args, **kwargs):
    kwargs.setdefault("cwd", SOURCE)
    return subprocess.run(args, check=True, **kwargs)


def sha256_bytes(data):
    return hashlib.sha256(data).hexdigest()


def sha256_file(path):
    return sha256_bytes(Path(path).read_bytes())


def manifest_version():
    with open(SOURCE / "Cargo.toml", "rb") as manifest:
        return tomllib.load(manifest)["package"]["version"]


def source_date_epoch():
    """The commit time, unless the caller already pinned SOURCE_DATE_EPOCH."""
    pinned = os.environ.get("SOURCE_DATE_EPOCH")
    if pinned:
        return int(pinned)
    out = run(
        ["git", "log", "-1", "--format=%ct", "HEAD"],
        capture_output=True,
        text=True,
    ).stdout
    return int(out.strip())


def require_clean_tree():
    dirty = subprocess.run(["git", "diff", "--quiet", "HEAD"], cwd=SOURCE)
    if dirty.returncode != 0:
        fail("tracked files differ from HEAD; build from a clean checkout")


def rustc_info():
    out = run(["rustc", "-vV"], capture_output=True, text=True).stdout
    info = dict(
        line.split(": ", 1) for line in out.splitlines() if ": " in line
    )
    return out.strip(), info.get("host", "")


def path_forms(path):
    """The path as given and fully resolved, without duplicates."""
    forms = {str(path), str(Path(path).resolve())}
    return sorted(forms)


def build_flags(target):
    """Rustc flags that remove build-machine paths from the binary."""
    cargo_home = Path(os.environ.get("CARGO_HOME") or Path.home() / ".cargo")
    rustup_home = Path(
        os.environ.get("RUSTUP_HOME") or Path.home() / ".rustup"
    )
    flags = []
    for prefix, replacement in (
        (cargo_home, "/cargo"),
        (rustup_home, "/rustup"),
        (SOURCE, "/pyfaro"),
    ):
        for form in path_forms(prefix):
            flags.append(f"--remap-path-prefix={form}={replacement}")
    if target.endswith("-msvc"):
        # Without /Brepro the linker stamps the build time into the PE header.
        flags += ["-C", "link-arg=/Brepro"]
    return flags


def build_binary(target, epoch):
    env = dict(os.environ)
    env.pop("RUSTFLAGS", None)
    env["CARGO_ENCODED_RUSTFLAGS"] = "\x1f".join(build_flags(target))
    env["CARGO_INCREMENTAL"] = "0"
    env["SOURCE_DATE_EPOCH"] = str(epoch)
    run(
        [
            "cargo",
            "build",
            "--locked",
            "--release",
            "--target",
            target,
            "--target-dir",
            str(SOURCE / "target"),
        ],
        env=env,
    )
    exe = BINARY + (".exe" if "windows" in target else "")
    binary = SOURCE / "target" / target / "release" / exe
    if not binary.is_file():
        fail(f"cargo did not produce {binary}")
    return binary


def version_command(binary, target, host):
    """How to run the binary here, or why it cannot be executed."""
    if target == host:
        return [str(binary)], None
    if target == "x86_64-apple-darwin" and host == "aarch64-apple-darwin":
        arch = shutil.which("arch")
        probe = (
            subprocess.run(
                [arch, "-x86_64", "/usr/bin/true"], capture_output=True
            )
            if arch
            else None
        )
        if probe is not None and probe.returncode == 0:
            return [arch, "-x86_64", str(binary)], None
        return None, (
            f"{target} is cross-compiled on {host} and this machine cannot "
            "run x86_64 code (Rosetta 2 is not installed)"
        )
    return None, f"{target} cannot run on {host}"


def check_version(binary, target, host, version):
    command, reason = version_command(binary, target, host)
    if command is None:
        notice(f"--version not checked: {reason}")
        return f"skipped: {reason}"
    shown = run(
        command + ["--version"], capture_output=True, text=True
    ).stdout.strip()
    if shown != f"{BINARY} {version}":
        fail(f"{binary.name} --version printed '{shown}', not '{BINARY} {version}'")
    print(f"--version: {shown}")
    return "checked"


def archive_entries(name, binary, target):
    exe = BINARY + (".exe" if "windows" in target else "")
    files = {doc: (SOURCE / doc, 0o644) for doc in DOCS}
    files[exe] = (binary, 0o755)
    return [
        (f"{name}/{leaf}", path.read_bytes(), mode)
        for leaf, (path, mode) in sorted(files.items())
    ]


def write_tar_gz(path, name, entries, epoch):
    """A tar.gz with sorted entries, fixed owner and times, and no gzip name."""
    with open(path, "wb") as raw:
        with gzip.GzipFile(
            filename="", mode="wb", fileobj=raw, compresslevel=9, mtime=0
        ) as compressed:
            with tarfile.open(
                fileobj=compressed, mode="w", format=tarfile.GNU_FORMAT
            ) as tar:
                top = tarfile.TarInfo(name)
                top.type = tarfile.DIRTYPE
                members = [(top, 0o755, None)] + [
                    (tarfile.TarInfo(arcname), mode, data)
                    for arcname, data, mode in entries
                ]
                for info, mode, data in members:
                    info.mode = mode
                    info.mtime = epoch
                    info.uid = info.gid = 0
                    info.uname = info.gname = ""
                    if data is None:
                        tar.addfile(info)
                    else:
                        info.size = len(data)
                        tar.addfile(info, fileobj=io.BytesIO(data))


def write_zip(path, name, entries, epoch):
    """A zip with sorted entries, fixed times, and Unix permissions."""
    stamp = time.gmtime(max(epoch, ZIP_MIN_EPOCH))[:6]

    def info(arcname, mode, directory=False):
        member = zipfile.ZipInfo(arcname, date_time=stamp)
        member.create_system = 3
        member.external_attr = (mode << 16) | (0x10 if directory else 0)
        member.compress_type = zipfile.ZIP_DEFLATED
        return member

    with zipfile.ZipFile(path, "w") as archive:
        archive.writestr(info(f"{name}/", 0o40755, directory=True), b"")
        for arcname, data, mode in entries:
            archive.writestr(
                info(arcname, 0o100000 | mode), data, compresslevel=9
            )


def archive_name(version, target):
    suffix = ".zip" if "windows" in target else ".tar.gz"
    return f"{BINARY}-{version}-{target}{suffix}"


def cmd_build(args):
    global SOURCE
    SOURCE = Path(args.source).resolve()
    target = args.target
    if target not in TARGETS:
        fail(f"unsupported target {target}; expected one of {', '.join(TARGETS)}")
    version = manifest_version()
    if args.expect_version and args.expect_version != version:
        fail(f"Cargo.toml has version {version}, not {args.expect_version}")
    require_clean_tree()
    epoch = source_date_epoch()
    verbose, host = rustc_info()
    print(f"pyfaro {version} for {target} on {host}, "
          f"SOURCE_DATE_EPOCH={epoch}\n{verbose}", flush=True)

    binary = build_binary(target, epoch)
    status = check_version(binary, target, host, version)

    name = f"{BINARY}-{version}-{target}"
    entries = archive_entries(name, binary, target)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    archive = out / archive_name(version, target)
    if "windows" in target:
        write_zip(archive, name, entries, epoch)
    else:
        write_tar_gz(archive, name, entries, epoch)

    report = {
        "target": target,
        "version": version,
        "source_date_epoch": epoch,
        "host": host,
        "rustc": verbose,
        "version_check": status,
        "binary_sha256": sha256_file(binary),
        "archive": archive.name,
        "archive_sha256": sha256_file(archive),
    }
    if args.report:
        report_dir = Path(args.report)
        report_dir.mkdir(parents=True, exist_ok=True)
        (report_dir / f"{target}.json").write_text(
            json.dumps(report, indent=2) + "\n"
        )
    print(f"binary  sha256 {report['binary_sha256']}  {binary.name}")
    print(f"archive sha256 {report['archive_sha256']}  {archive.name}")


def archived_binary_sha256(archive):
    leaf = BINARY + (".exe" if archive.suffix == ".zip" else "")
    if archive.suffix == ".zip":
        with zipfile.ZipFile(archive) as bundle:
            member = next(n for n in bundle.namelist() if n.endswith("/" + leaf))
            return sha256_bytes(bundle.read(member))
    with tarfile.open(archive, "r:gz") as bundle:
        member = next(m for m in bundle.getmembers() if m.name.endswith("/" + leaf))
        return sha256_bytes(bundle.extractfile(member).read())


def load_reports(root):
    reports = {}
    for path in sorted((Path(root) / "release-report").glob("*.json")):
        report = json.loads(path.read_text())
        reports[report["target"]] = (report, Path(root) / "dist")
    return reports


def cmd_compare(args):
    first = load_reports(args.first)
    second = load_reports(args.second)
    problems = []
    for label, reports in (("first", first), ("second", second)):
        missing = sorted(set(TARGETS) - set(reports))
        if missing:
            problems.append(f"{label} build is missing {', '.join(missing)}")
    rows = []
    for target in TARGETS:
        if target not in first or target not in second:
            continue
        (a, a_dist), (b, b_dist) = first[target], second[target]
        for label, report, dist in (("first", a, a_dist), ("second", b, b_dist)):
            archive = dist / report["archive"]
            if sha256_file(archive) != report["archive_sha256"]:
                problems.append(f"{target}: {label} archive differs from its report")
            elif archived_binary_sha256(archive) != report["binary_sha256"]:
                problems.append(f"{target}: {label} archive holds a different binary")
        same_binary = a["binary_sha256"] == b["binary_sha256"]
        same_archive = (
            a["archive"] == b["archive"]
            and a["archive_sha256"] == b["archive_sha256"]
        )
        if not same_binary:
            problems.append(
                f"{target}: binaries differ "
                f"({a['binary_sha256']} vs {b['binary_sha256']})"
            )
        if not same_archive:
            problems.append(
                f"{target}: archives differ "
                f"({a['archive_sha256']} vs {b['archive_sha256']})"
            )
        rows.append((target, a, same_binary and same_archive))

    versions = {r["version"] for _, r, _ in rows}
    if len(versions) > 1:
        problems.append(f"builds disagree on the version: {sorted(versions)}")

    lines = [
        "| Target | Archive SHA-256 | Binary SHA-256 | --version | Identical |",
        "| --- | --- | --- | --- | --- |",
    ]
    for target, report, same in rows:
        lines.append(
            f"| `{target}` | `{report['archive_sha256']}` | "
            f"`{report['binary_sha256']}` | {report['version_check']} | "
            f"{'yes' if same else '**NO**'} |"
        )
    table = "\n".join(lines)
    print(table)
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a") as handle:
            handle.write(
                "### Reproducibility: two independent builds per target\n\n"
                f"{table}\n"
            )
    if problems:
        for problem in problems:
            print(f"::error::{problem}" if os.environ.get("GITHUB_ACTIONS") else problem)
        sys.exit(1)

    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    for target in TARGETS:
        report, dist = first[target]
        shutil.copy2(dist / report["archive"], out / report["archive"])
    print(f"Both builds of all {len(TARGETS)} targets are identical; "
          f"archives copied to {out}")


def cmd_check_crate(args):
    """The .crate must be this version of the package, cut from this commit."""
    top = f"{BINARY}-{args.version}"
    with tarfile.open(args.crate, "r:gz") as crate:
        names = crate.getnames()
        stray = [n for n in names if n != top and not n.startswith(top + "/")]
        if stray:
            fail(f"{args.crate} holds files outside {top}/: {stray[:3]}")
        try:
            manifest = tomllib.loads(
                crate.extractfile(f"{top}/Cargo.toml").read().decode()
            )
            vcs = json.loads(
                crate.extractfile(f"{top}/.cargo_vcs_info.json").read()
            )
        except KeyError as missing:
            fail(f"{args.crate} has no {missing}")
    package = manifest["package"]
    if package["name"] != BINARY or package["version"] != args.version:
        fail(
            f"{args.crate} is {package['name']} {package['version']}, "
            f"not {BINARY} {args.version}"
        )
    if vcs.get("git", {}).get("sha1") != args.commit or vcs.get("dirty"):
        fail(
            f"{args.crate} was not packaged from a clean {args.commit} "
            f"(it records {vcs})"
        )
    print(f"{args.crate}: {BINARY} {args.version} from clean {args.commit}")


def cmd_check_assets(args):
    """The directory holds exactly one archive per supported target."""
    expected = {archive_name(args.version, target) for target in TARGETS}
    found = {path.name for path in Path(args.directory).iterdir()}
    if found != expected:
        fail(
            f"{args.directory} should hold {sorted(expected)}; "
            f"missing {sorted(expected - found)}, "
            f"unexpected {sorted(found - expected)}"
        )
    print(f"{args.directory}: {len(expected)} archives for {args.version}")


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    commands = parser.add_subparsers(dest="command", required=True)

    build = commands.add_parser("build", help="build and archive one target")
    build.add_argument("--target", required=True, choices=TARGETS)
    build.add_argument(
        "--source", default=str(SOURCE), help="checkout to build (default: this one)"
    )
    build.add_argument("--out", default="dist", help="archive directory")
    build.add_argument("--report", help="directory for the JSON build report")
    build.add_argument(
        "--expect-version", help="fail unless Cargo.toml has this version"
    )
    build.set_defaults(handler=cmd_build)

    compare = commands.add_parser(
        "compare", help="check two builds of all targets are identical"
    )
    compare.add_argument("--first", required=True, help="first build directory")
    compare.add_argument("--second", required=True, help="second build directory")
    compare.add_argument("--out", default="dist", help="directory for the archives")
    compare.set_defaults(handler=cmd_compare)

    check = commands.add_parser(
        "check-crate", help="check a .crate against its version and commit"
    )
    check.add_argument("crate")
    check.add_argument("--version", required=True)
    check.add_argument("--commit", required=True)
    check.set_defaults(handler=cmd_check_crate)

    assets = commands.add_parser(
        "check-assets", help="check a directory holds one archive per target"
    )
    assets.add_argument("directory")
    assets.add_argument("--version", required=True)
    assets.set_defaults(handler=cmd_check_assets)

    args = parser.parse_args()
    args.handler(args)


if __name__ == "__main__":
    main()
