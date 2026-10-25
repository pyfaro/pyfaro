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
import re
import shutil
import subprocess
import sys
import tarfile
import time
import tomllib
import venv
import zipfile
from email.parser import BytesParser
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
# The platform tag of each target's wheel. Linux wheels target the oldest
# glibc Rust 1.90 supports on both architectures (manylinux 2.17, a.k.a.
# manylinux2014); macOS and Windows tags carry the deployment target.
WHEEL_PLATFORMS = {
    "x86_64-unknown-linux-gnu": r"manylinux_2_17_x86_64(\.manylinux2014_x86_64)?",
    "aarch64-unknown-linux-gnu": r"manylinux_2_17_aarch64(\.manylinux2014_aarch64)?",
    "aarch64-apple-darwin": r"macosx_\d+_\d+_arm64",
    "x86_64-apple-darwin": r"macosx_\d+_\d+_x86_64",
    "x86_64-pc-windows-msvc": r"win_amd64",
}
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


def pep440(version):
    """The PEP 440 form of a Cargo version, as maturin writes it."""
    match = re.fullmatch(
        r"(\d+\.\d+\.\d+)(?:-(alpha|beta|rc)\.(\d+))?", version
    )
    if not match:
        fail(f"cannot map Cargo version {version} to PEP 440")
    release, kind, number = match.groups()
    if kind is None:
        return release
    return release + {"alpha": "a", "beta": "b", "rc": "rc"}[kind] + number


def cmd_pep440(args):
    print(pep440(args.version))


def wheel_target(wheel):
    """The release target a wheel file name belongs to, or None."""
    for target, platform_tag in WHEEL_PLATFORMS.items():
        if re.fullmatch(
            rf"{BINARY}-[^-]+-py3-none-{platform_tag}\.whl", wheel.name
        ):
            return target
    return None


def check_wheel_contents(wheel, version):
    """A wheel ships exactly the executable, with this package's metadata."""
    leaf = BINARY + (".exe" if "win_amd64" in wheel.name else "")
    with zipfile.ZipFile(wheel) as archive:
        names = archive.namelist()
        scripts = [n for n in names if ".data/scripts/" in n]
        if scripts != [f"{BINARY}-{pep440(version)}.data/scripts/{leaf}"]:
            fail(f"{wheel.name} should ship only {leaf} as a script: {scripts}")
        code = [n for n in names if n.endswith((".py", ".so", ".pyd"))]
        if code:
            fail(f"{wheel.name} ships Python code or extensions: {code}")
        info = f"{BINARY}-{pep440(version)}.dist-info"
        if f"{info}/licenses/LICENSE-APACHE" not in names:
            fail(f"{wheel.name} does not ship the license text")
        metadata = BytesParser().parsebytes(archive.read(f"{info}/METADATA"))
    if metadata["Name"] != BINARY or metadata["Version"] != pep440(version):
        fail(
            f"{wheel.name} is {metadata['Name']} {metadata['Version']}, "
            f"not {BINARY} {pep440(version)}"
        )


def cmd_check_wheels(args):
    """The directory holds the sdist and one wheel per target, nothing else."""
    wheels = Path(args.directory)
    found = {path.name: path for path in wheels.iterdir()}
    sdist = f"{BINARY}-{pep440(args.version)}.tar.gz"
    by_target = {}
    problems = []
    for name, path in sorted(found.items()):
        if name == sdist:
            continue
        target = wheel_target(path) if name.endswith(".whl") else None
        if target is None or target in by_target:
            problems.append(f"unexpected file {name}")
        else:
            by_target[target] = path
    if sdist not in found:
        problems.append(f"missing {sdist}")
    problems += [
        f"missing the wheel for {target}"
        for target in TARGETS
        if target not in by_target
    ]
    if problems:
        fail(f"{wheels}: " + "; ".join(problems))
    for wheel in by_target.values():
        if f"-{pep440(args.version)}-" not in wheel.name:
            fail(f"{wheel.name} is not version {pep440(args.version)}")
        check_wheel_contents(wheel, args.version)
    with tarfile.open(wheels / sdist) as archive:
        if f"{BINARY}-{pep440(args.version)}/pyproject.toml" not in archive.getnames():
            fail(f"{sdist} has no pyproject.toml")
    print(f"{wheels}: sdist and {len(by_target)} wheels for {pep440(args.version)}")


def cmd_cargo_version(args):
    global SOURCE
    SOURCE = Path(args.source).resolve()
    print(manifest_version())


def cmd_smoke_wheel(args):
    """Install this target's wheel into a fresh venv and run --version."""
    global SOURCE
    SOURCE = Path(args.source).resolve()
    version = args.version or manifest_version()
    wheels = [
        path
        for path in Path(args.directory).glob("*.whl")
        if wheel_target(path) == args.target
    ]
    if len(wheels) != 1:
        fail(f"expected one {args.target} wheel in {args.directory}: {wheels}")
    wheel = wheels[0]
    _, host = rustc_info()
    exe = BINARY + (".exe" if "windows" in args.target else "")
    scratch = Path(args.scratch)
    shutil.rmtree(scratch, ignore_errors=True)
    if args.target == host:
        venv.create(scratch, with_pip=True)
        bin_dir = scratch / ("Scripts" if os.name == "nt" else "bin")
        python = bin_dir / ("python.exe" if os.name == "nt" else "python")
        run(
            [str(python), "-m", "pip", "install", "--no-index", "--no-deps",
             "--disable-pip-version-check", str(wheel)],
            cwd=scratch.parent,
        )
        binary = bin_dir / exe
    else:
        # pip refuses a wheel for another architecture; unpack the script.
        with zipfile.ZipFile(wheel) as archive:
            member = f"{BINARY}-{pep440(version)}.data/scripts/{exe}"
            scratch.mkdir(parents=True)
            binary = scratch / exe
            binary.write_bytes(archive.read(member))
            binary.chmod(0o755)
    check_version(binary, args.target, host, version)


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

    version_cmd = commands.add_parser(
        "pep440", help="print the PEP 440 form of a Cargo version"
    )
    version_cmd.add_argument("version")
    version_cmd.set_defaults(handler=cmd_pep440)

    cargo_version = commands.add_parser(
        "cargo-version", help="print the Cargo.toml version of a checkout"
    )
    cargo_version.add_argument("--source", default=str(SOURCE))
    cargo_version.set_defaults(handler=cmd_cargo_version)

    wheels = commands.add_parser(
        "check-wheels", help="check a directory holds the sdist and all wheels"
    )
    wheels.add_argument("directory")
    wheels.add_argument("--version", required=True, help="Cargo version")
    wheels.set_defaults(handler=cmd_check_wheels)

    smoke = commands.add_parser(
        "smoke-wheel", help="install a wheel in a fresh venv and run it"
    )
    smoke.add_argument("directory", help="directory with the wheels")
    smoke.add_argument("--target", required=True, choices=TARGETS)
    smoke.add_argument("--version", help="Cargo version (default: Cargo.toml)")
    smoke.add_argument("--source", default=str(SOURCE), help="checkout with Cargo.toml")
    smoke.add_argument("--scratch", default="wheel-venv")
    smoke.set_defaults(handler=cmd_smoke_wheel)

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
