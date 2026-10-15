#!/usr/bin/env python3
"""Import the official backlog with gh; dry-run unless --apply or --verify.

Resumes by stable backlog ID, with an exact-title fallback for the original
importer. Does not close issues, create repositories, or overwrite issue prose.
"""

import argparse
import json
import re
import subprocess
import sys
import time
from pathlib import Path

MANIFEST = Path(__file__).resolve().parent / "pyfaro-issues.json"
MARKER = re.compile(r"<!-- pyfaro-backlog-id: ([A-Z][0-9]+) -->")


def run(*args, payload=None):
    result = subprocess.run(
        ["gh", *args],
        input=json.dumps(payload) if payload is not None else None,
        text=True,
        capture_output=True,
        check=False,
        timeout=120,
    )
    if result.returncode:
        # Never retry an ambiguous write: resume by inspecting remote IDs.
        raise RuntimeError(
            f"gh {' '.join(args[:3])} failed: {result.stderr.strip()}"
        )
    return json.loads(result.stdout) if result.stdout.strip() else None


def api(path, method="GET", payload=None):
    args = ["api", "--method", method, path]
    if payload is not None:
        args += ["--input", "-"]
    result = run(*args, payload=payload)
    if method != "GET":
        # Stay below GitHub's content-generation secondary rate limits.
        time.sleep(1)
    return result


def pages(path):
    separator = "&" if "?" in path else "?"
    result = run(
        "api", "--paginate", "--slurp", f"{path}{separator}per_page=100"
    )
    return [item for page in result for item in page]


def validate(data):
    for key, field in (
        ("labels", "name"), ("milestones", "title"), ("issues", "id")
    ):
        values = [item[field] for item in data[key]]
        if len(values) != len(set(values)):
            raise ValueError(f"Duplicate {key} {field}")
    titles = [item["title"] for item in data["issues"]]
    if len(titles) != len(set(titles)):
        raise ValueError("Duplicate issue title")
    labels = {item["name"] for item in data["labels"]}
    milestones = {item["title"] for item in data["milestones"]}
    issues = {item["id"]: item for item in data["issues"]}
    for issue in issues.values():
        if issue["milestone"] not in milestones:
            raise ValueError(f"Unknown milestone: {issue['id']}")
        if len(issue["labels"]) != 3 or not set(issue["labels"]) <= labels:
            raise ValueError(f"Invalid labels: {issue['id']}")
        for prefix in ("type:", "area:", "priority:"):
            if sum(x.startswith(prefix) for x in issue["labels"]) != 1:
                raise ValueError(f"Expected one {prefix} label: {issue['id']}")
        if not issue["summary"] or not issue["acceptance"]:
            raise ValueError(f"Missing issue scope: {issue['id']}")
        if len(issue["depends_on"]) != len(set(issue["depends_on"])):
            raise ValueError(f"Duplicate dependency: {issue['id']}")
        if not set(issue["depends_on"]) <= issues.keys():
            raise ValueError(f"Unknown dependency: {issue['id']}")
    visited, visiting = set(), set()

    def visit(key):
        if key in visiting:
            raise ValueError(f"Dependency cycle at {key}")
        if key in visited:
            return
        visiting.add(key)
        for dependency in issues[key]["depends_on"]:
            visit(dependency)
        visiting.remove(key)
        visited.add(key)

    for key in issues:
        visit(key)


def dependencies(issue, mapping=None):
    if not issue["depends_on"]:
        return "None."
    if mapping is None:
        return "Backlog IDs (see `pyfaro-issues.json`): " + ", ".join(
            f"`{key}`" for key in issue["depends_on"]
        )
    return "\n".join(
        f"- `{key}`: {mapping[key]['html_url']}"
        for key in issue["depends_on"]
    )


def body(issue):
    lines = [
        f"<!-- pyfaro-backlog-id: {issue['id']} -->",
        f"**Backlog ID:** `{issue['id']}`", "", "## Goal", "",
        issue["summary"], "", "## Acceptance criteria", "",
    ]
    lines += [f"- [ ] {step}" for step in issue["acceptance"]]
    lines += ["", "## Dependencies", "", dependencies(issue), "",
              "## Notes", "",
              "Use existing uv/Ruff/ty/pytest capabilities; "
              "avoid reimplementing them."]
    return "\n".join(lines) + "\n"


def identify(specs, existing):
    by_id, by_title = {}, {}
    for remote in existing:
        if "pull_request" in remote:
            continue
        by_title.setdefault(remote["title"], []).append(remote)
        markers = MARKER.findall(remote.get("body") or "")
        if len(markers) > 1:
            raise ValueError(f"Ambiguous backlog markers: #{remote['number']}")
        for key in markers:
            if key in by_id:
                raise ValueError(f"Duplicate remote backlog ID: {key}")
            by_id[key] = remote
    mapping = {}
    for issue in specs:
        matches = by_title.get(issue["title"], [])
        if len(matches) > 1:
            raise ValueError(f"Duplicate remote title: {issue['title']}")
        remote = by_id.get(issue["id"])
        if remote and matches and remote["id"] != matches[0]["id"]:
            raise ValueError(f"ID/title collision: {issue['id']}")
        if remote is None and matches:
            remote = matches[0]
            # Exact title alone must not adopt an unrelated user's issue.
            if f"**Backlog ID:** `{issue['id']}`" not in (
                remote.get("body") or ""
            ):
                raise ValueError(f"Unmanaged title collision: {issue['id']}")
        if remote:
            mapping[issue["id"]] = remote
    return mapping


def linked_body(issue, remote, mapping):
    current = remote["body"]
    old = "## Dependencies\n\n" + dependencies(issue) + "\n"
    new = "## Dependencies\n\n" + dependencies(issue, mapping) + "\n"
    if new in current:
        return current
    if current.count(old) != 1:
        raise ValueError(f"Edited dependency section: {issue['id']}")
    # Preserve acceptance checkbox state and all contributor-written prose.
    return current.replace(old, new, 1)


def verify(data, mapping, labels, milestones, root):
    for label in data["labels"]:
        remote = labels[label["name"]]
        assert remote["color"].lower() == label["color"].lower(), label
        assert remote["description"] == label["description"], label
    for milestone in data["milestones"]:
        assert milestones[milestone["title"]]["description"] == (
            milestone["description"]
        ), milestone
    edges = 0
    for spec in data["issues"]:
        remote = mapping[spec["id"]]
        assert remote["title"] == spec["title"], spec["id"]
        assert remote["milestone"]["title"] == spec["milestone"], spec["id"]
        assert {x["name"] for x in remote["labels"]} == set(
            spec["labels"]
        ), spec["id"]
        assert MARKER.findall(remote["body"]) == [spec["id"]], spec["id"]
        assert spec["summary"] in remote["body"], spec["id"]
        for step in spec["acceptance"]:
            assert f"] {step}" in remote["body"], (spec["id"], step)
        assert dependencies(spec, mapping) in remote["body"], spec["id"]
        blocked_by = pages(
            f"{root}/issues/{remote['number']}/dependencies/blocked_by"
        )
        assert {x["id"] for x in blocked_by} == {
            mapping[key]["id"] for key in spec["depends_on"]
        }, spec["id"]
        edges += len(blocked_by)
    print(f"Verified {len(data['labels'])} labels, "
          f"{len(data['milestones'])} milestones, {len(mapping)} issues, "
          f"{edges} native dependency edges.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", default="pyfaro/pyfaro")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--apply", action="store_true")
    mode.add_argument("--verify", action="store_true")
    parser.add_argument("--through", help="Last included milestone version")
    args = parser.parse_args()
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", args.repo):
        parser.error("--repo must be OWNER/REPO")
    data = json.loads(MANIFEST.read_text(encoding="utf-8"))
    validate(data)
    if args.through:
        versions = [x["title"].split(" — ")[0] for x in data["milestones"]]
        if args.through not in versions:
            parser.error("--through must be one of: " + ", ".join(versions))
        data["milestones"] = data["milestones"][:versions.index(args.through)+1]
        selected = {x["title"] for x in data["milestones"]}
        data["issues"] = [
            x for x in data["issues"] if x["milestone"] in selected
        ]
        validate(data)
    print(f"Target: {args.repo}")
    print(f"Plan: {len(data['labels'])} labels, "
          f"{len(data['milestones'])} milestones, {len(data['issues'])} issues")
    if not args.apply and not args.verify:
        print("DRY RUN: no GitHub changes. Pass --apply to import everything.")
        for milestone in data["milestones"]:
            count = sum(x["milestone"] == milestone["title"]
                        for x in data["issues"])
            print(f"  {milestone['title']}: {count} issues")
        return 0
    root = f"repos/{args.repo}"
    repo = api(root)
    if args.apply and not repo.get("permissions", {}).get("push"):
        raise ValueError("Repository write permission required")
    existing = pages(f"{root}/issues?state=all")
    mapping = identify(data["issues"], existing)
    labels = {x["name"]: x for x in pages(f"{root}/labels")}
    milestones = {
        x["title"]: x for x in pages(f"{root}/milestones?state=all")
    }
    created = 0
    if args.apply:
        # Preflight existing managed bodies before creating any resources.
        for spec in data["issues"]:
            if spec["id"] in mapping:
                current = mapping[spec["id"]]
                if spec["summary"] not in current["body"]:
                    raise ValueError(f"Edited managed issue: {spec['id']}")
        for label in data["labels"]:
            name = label["name"]
            remote = labels.get(name)
            if remote is None:
                labels[name] = api(f"{root}/labels", "POST", label)
                print("Created label:", name, flush=True)
            elif any(remote[key] != label[key]
                     for key in ("color", "description")):
                from urllib.parse import quote
                labels[name] = api(
                    f"{root}/labels/{quote(name, safe='')}", "PATCH", label
                )
        for milestone in data["milestones"]:
            title = milestone["title"]
            if title not in milestones:
                milestones[title] = api(
                    f"{root}/milestones", "POST", milestone
                )
                print("Created milestone:", title, flush=True)
            elif milestones[title]["description"] != milestone["description"]:
                raise ValueError(f"Milestone description differs: {title}")
        for spec in data["issues"]:
            fields = {
                "title": spec["title"], "labels": spec["labels"],
                "milestone": milestones[spec["milestone"]]["number"],
            }
            remote = mapping.get(spec["id"])
            if remote is None:
                remote = api(
                    f"{root}/issues", "POST", {**fields, "body": body(spec)}
                )
                mapping[spec["id"]] = remote
                created += 1
                print("Created:", spec["id"], remote["html_url"], flush=True)
            else:
                if (remote["title"] != spec["title"] or
                    {x["name"] for x in remote["labels"]} !=
                    set(spec["labels"]) or
                    not remote["milestone"] or
                    remote["milestone"]["title"] != spec["milestone"]):
                    mapping[spec["id"]] = api(
                        f"{root}/issues/{remote['number']}", "PATCH", fields
                    )
                print("Reused:", spec["id"], remote["html_url"], flush=True)
        for spec in data["issues"]:
            remote = mapping[spec["id"]]
            path = f"{root}/issues/{remote['number']}"
            linked = linked_body(spec, remote, mapping)
            if linked != remote["body"]:
                remote = api(path, "PATCH", {"body": linked})
                mapping[spec["id"]] = remote
            dependency_path = f"{path}/dependencies/blocked_by"
            known = {x["id"] for x in pages(dependency_path)}
            for key in spec["depends_on"]:
                if mapping[key]["id"] not in known:
                    api(dependency_path, "POST", {"issue_id": mapping[key]["id"]})
            print("Linked dependencies:", spec["id"], flush=True)
        print(f"Created {created} issues; reused {len(mapping)-created}.")
    # Fresh reads verify remote results, rather than cached mutation responses.
    mapping = identify(data["issues"], pages(f"{root}/issues?state=all"))
    labels = {x["name"]: x for x in pages(f"{root}/labels")}
    milestones = {
        x["title"]: x for x in pages(f"{root}/milestones?state=all")
    }
    verify(data, mapping, labels, milestones, root)
    if args.apply:
        output = {
            key: {"number": remote["number"], "url": remote["html_url"]}
            for key, remote in mapping.items()
        }
        MANIFEST.with_name("issue-map.json").write_text(
            json.dumps(output, indent=2) + "\n", encoding="utf-8"
        )
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, RuntimeError, ValueError, KeyError, AssertionError,
            subprocess.TimeoutExpired) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        sys.exit(1)
