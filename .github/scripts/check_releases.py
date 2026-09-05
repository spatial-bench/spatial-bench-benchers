#!/usr/bin/env python3
"""Scan every subject manifest for a newer upstream release than its pin,
and open a PR per subject when one exists.

One implementation covers all three pin kinds:
  - cargo-git / git → GitHub releases API (the tag's commit sha feeds S1)
  - pypi            → PyPI JSON API (the version is the pin; immutable)
"""

import json
import subprocess
import sys
import urllib.request
from pathlib import Path

REPO = "spatial-bench/spatial-bench-benchers"
SUBJECTS_DIR = Path("subjects")
# The GitHub repo that hosts each git-pinned library. Inferred from the
# manifest's `repo` URL — no hardcoded list.
GITHUB_REPO_PREFIX = "https://github.com/"


def parse_manifest(path: Path) -> dict:
    """Minimal TOML parse for the fields the watcher needs. The `toml` crate
    and Python's stdlib `tomllib` (3.11+) both exist; tomllib keeps this
    dependency-free."""
    import tomllib
    with open(path, "rb") as f:
        return tomllib.load(f)


def github_latest_tag(repo_url: str) -> tuple[str, str] | None:
    """The latest GitHub release: (tag, commit sha)."""
    if not repo_url.startswith(GITHUB_REPO_PREFIX):
        return None
    repo_path = repo_url[len(GITHUB_REPO_PREFIX):].removesuffix(".git")
    url = f"https://api.github.com/repos/{repo_path}/releases/latest"
    req = urllib.request.Request(url, headers={
        "Accept": "application/vnd.github+json",
        "User-Agent": "spatial-bench-release-watcher",
    })
    try:
        with urllib.request.urlopen(req) as resp:
            data = json.loads(resp.read())
    except urllib.error.HTTPError as e:
        if e.code == 404:
            return None  # no releases
        raise
    tag = data.get("tag_name")
    sha = data.get("target_commitish")
    if not tag or not sha:
        return None
    # The API's target_commitish for a release is the branch, not the tag's
    # commit. Resolve the actual tag sha via the ref API.
    ref_url = f"https://api.github.com/repos/{repo_path}/git/ref/tags/{tag}"
    req = urllib.request.Request(ref_url, headers={
        "Accept": "application/vnd.github+json",
        "User-Agent": "spatial-bench-release-watcher",
    })
    with urllib.request.urlopen(req) as resp:
        ref = json.loads(resp.read())
    sha = ref["object"]["sha"]
    # Annotated tags point at the tag object, not the commit — dereference.
    if ref["object"]["type"] == "tag":
        obj_url = f"https://api.github.com/repos/{repo_path}/git/tags/{sha}"
        req = urllib.request.Request(obj_url, headers={
            "Accept": "application/vnd.github+json",
            "User-Agent": "spatial-bench-release-watcher",
        })
        with urllib.request.urlopen(req) as resp:
            sha = json.loads(resp.read())["object"]["sha"]
    return (tag, sha)


def pypi_latest_version(package: str) -> str | None:
    """The latest non-prerelease version on PyPI."""
    url = f"https://pypi.org/pypi/{package}/json"
    req = urllib.request.Request(url, headers={
        "User-Agent": "spatial-bench-release-watcher",
    })
    with urllib.request.urlopen(req) as resp:
        data = json.loads(resp.read())
    version = data.get("info", {}).get("version")
    return version


def semver_gt(a: str, b: str) -> bool:
    """Loose semver comparison — enough to tell 1.7.1 from 1.8.0."""
    def key(v: str):
        parts = []
        for piece in v.lstrip("vV").split("-")[0].split("."):
            try:
                parts.append(int(piece))
            except ValueError:
                parts.append(0)
        while len(parts) < 3:
            parts.append(0)
        return tuple(parts)
    return key(a) > key(b)


def git(command: list[str]) -> str:
    result = subprocess.run(["git"] + command, capture_output=True, text=True)
    if result.returncode != 0:
        raise RuntimeError(f"git {command}: {result.stderr}")
    return result.stdout.strip()


def open_pin_bump_pr(subject: str, manifest_path: Path, updates: dict[str, str],
                     new_ref: str) -> None:
    branch = f"pin-bump/{subject}-{new_ref}"
    # Reset to main and create the branch.
    git(["checkout", "main"])
    git(["checkout", "-B", branch, "origin/main"])

    # Apply the manifest updates.
    text = manifest_path.read_text()
    for old, new in updates.items():
        text = text.replace(old, new)
    manifest_path.write_text(text)
    git(["add", str(manifest_path)])
    git(["commit", "-m",
         f"chore({subject}): bump pin to {new_ref}"])

    git(["push", "-u", "origin", branch, "--force"])

    # Open the PR.
    title = f"chore({subject}): bump pin to {new_ref}"
    body = (
        f"Automated pin bump detected by the release watcher.\n\n"
        f"Updated: {', '.join(updates.keys())}\n\n"
        f"CI on this PR benches the new version against the previous one."
    )
    subprocess.run(
        ["gh", "pr", "create",
         "--repo", REPO,
         "--base", "main",
         "--head", branch,
         "--title", title,
         "--body", body],
        capture_output=True, text=True,
    )


def main() -> int:
    opened = 0
    for subject_dir in sorted(SUBJECTS_DIR.iterdir()):
        manifest_path = subject_dir / "subject.toml"
        if not manifest_path.exists():
            continue
        manifest = parse_manifest(manifest_path)
        name = manifest["name"]
        source = manifest.get("source", {})
        kind = source.get("kind", "")
        old_ref = source.get("pinned_ref", "")
        if not old_ref:
            continue

        updates = {}

        if kind in ("cargo-git", "git"):
            repo_url = source.get("repo")
            if not repo_url:
                continue
            latest = github_latest_tag(repo_url)
            if latest is None:
                continue
            new_tag, new_sha = latest
            if not semver_gt(new_tag, old_ref):
                continue
            updates[f'pinned_ref = "{old_ref}"'] = f'pinned_ref = "{new_tag}"'
            if source.get("sha"):
                updates[f'sha = "{source["sha"]}'] = f'sha = "{new_sha}"'
            new_ref = new_tag

        elif kind == "pypi":
            package = source.get("package")
            if not package:
                continue
            latest = pypi_latest_version(package)
            if latest is None or not semver_gt(latest, old_ref):
                continue
            updates[f'pinned_ref = "{old_ref}"'] = f'pinned_ref = "{latest}"'
            new_ref = latest

        else:
            print(f"  {name}: source kind `{kind}` — not monitored, skipping")
            continue

        if not updates:
            print(f"  {name}: {old_ref} is current")
            continue

        print(f"  {name}: {old_ref} → {new_ref}, opening PR")
        open_pin_bump_pr(name, manifest_path, updates, new_ref)
        opened += 1

    print(f"\n{opened} pin-bump PR(s) opened")
    return 0


if __name__ == "__main__":
    sys.exit(main())
