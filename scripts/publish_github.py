import base64
import json
import mimetypes
import os
import subprocess
import sys
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

OWNER = "202121000995"
REPO = "OpenList-Explorer"
BRANCH = "main"
ROOT = Path(__file__).resolve().parents[1]
APP_VERSION = json.loads((ROOT / "package.json").read_text(encoding="utf-8"))["version"]
TAG = f"v{APP_VERSION}"
INSTALLER = ROOT / "src-tauri" / "target" / "release" / "bundle" / "nsis" / f"OpenList Explorer_{APP_VERSION}_x64-setup.exe"

EXCLUDED_DIRS = {
    ".git",
    ".agents",
    ".codex",
    "login_gate_rewrite",
    "node_modules",
    "dist",
    "target",
    "__pycache__",
}

EXCLUDED_PATH_KEYWORDS = (
    "compare esp",
    "dump hooks",
    "esp dump",
    "login_gate",
    "c++ rewrite",
    "分析并用",
)

EXCLUDED_FILES = {
    "OpenList Explorer_0.1.0_x64-setup.exe",
}


def gh_token() -> str:
    env_token = os.environ.get("GH_TOKEN") or os.environ.get("GITHUB_TOKEN")
    if env_token and env_token.strip():
        return env_token.strip()

    result = subprocess.run(["gh", "auth", "token"], capture_output=True, text=True, check=False)
    token = result.stdout.strip()
    if not token:
        raise RuntimeError(
            "GitHub CLI is not logged in. Run: gh auth login, or set GH_TOKEN/GITHUB_TOKEN."
        )
    return token


TOKEN = gh_token()


def api(path: str, method: str = "GET", payload=None, headers=None, raw_body=None):
    url = f"https://api.github.com/{path.lstrip('/')}"
    body = raw_body
    req_headers = {
        "Authorization": f"Bearer {TOKEN}",
        "Accept": "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
        "User-Agent": "openlist-explorer-publish-script",
    }
    if headers:
        req_headers.update(headers)
    if payload is not None:
        body = json.dumps(payload).encode("utf-8")
        req_headers["Content-Type"] = "application/json"
    req = urllib.request.Request(url, data=body, method=method, headers=req_headers)
    try:
        with urllib.request.urlopen(req) as response:
            data = response.read()
            if not data:
                return None
            content_type = response.headers.get("Content-Type", "")
            if "application/json" in content_type:
                return json.loads(data.decode("utf-8"))
            return data
    except urllib.error.HTTPError as error:
        detail = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"GitHub API {method} {path} failed: {error.code} {detail}") from error


def api_optional(path: str, method: str = "GET", payload=None):
    try:
        return api(path, method, payload)
    except RuntimeError as error:
        if "failed: 404" in str(error) or "failed: 409" in str(error):
            return None
        raise


def should_include(path: Path) -> bool:
    rel = path.relative_to(ROOT)
    rel_text = rel.as_posix().lower()
    parts = set(rel.parts)
    if parts & EXCLUDED_DIRS:
        return False
    if any(keyword in rel_text for keyword in EXCLUDED_PATH_KEYWORDS):
        return False
    if path.name in EXCLUDED_FILES:
        return False
    if path.suffix.lower() in {".exe", ".msi"}:
        return False
    if path.stat().st_size > 95 * 1024 * 1024:
        return False
    return True


def source_files():
    for path in sorted(ROOT.rglob("*")):
        if path.is_file() and should_include(path):
            yield path


def create_blob(path: Path) -> str:
    content = base64.b64encode(path.read_bytes()).decode("ascii")
    blob = api(
        f"repos/{OWNER}/{REPO}/git/blobs",
        "POST",
        {"content": content, "encoding": "base64"},
    )
    return blob["sha"]


def remote_blob_paths(base_tree: str) -> set[str]:
    tree = api(f"repos/{OWNER}/{REPO}/git/trees/{base_tree}?recursive=1")
    return {
        item["path"]
        for item in tree.get("tree", [])
        if item.get("type") == "blob" and item.get("path")
    }


def initialize_empty_repo():
    readme = ROOT / "README.md"
    content = readme.read_bytes() if readme.exists() else b"# OpenList Explorer\n"
    api(
        f"repos/{OWNER}/{REPO}/contents/README.md",
        "PUT",
        {
            "message": "Initialize repository",
            "content": base64.b64encode(content).decode("ascii"),
            "branch": BRANCH,
        },
    )


def publish_source():
    ref = api_optional(f"repos/{OWNER}/{REPO}/git/ref/heads/{BRANCH}")
    if not ref:
        print("Repository is empty. Initializing first commit...")
        initialize_empty_repo()
        ref = api_optional(f"repos/{OWNER}/{REPO}/git/ref/heads/{BRANCH}")

    parent_sha = ref["object"]["sha"] if ref else None
    base_tree = None
    if parent_sha:
        commit = api(f"repos/{OWNER}/{REPO}/git/commits/{parent_sha}")
        base_tree = commit["tree"]["sha"]

    tree = []
    files = list(source_files())
    local_paths = {path.relative_to(ROOT).as_posix() for path in files}
    for index, path in enumerate(files, 1):
        rel = path.relative_to(ROOT).as_posix()
        print(f"[{index}/{len(files)}] upload {rel}")
        tree.append({
            "path": rel,
            "mode": "100644",
            "type": "blob",
            "sha": create_blob(path),
        })

    if base_tree:
        for remote_path in sorted(remote_blob_paths(base_tree) - local_paths):
            print(f"delete remote {remote_path}")
            tree.append({
                "path": remote_path,
                "mode": "100644",
                "type": "blob",
                "sha": None,
            })

    tree_payload = {"tree": tree}
    if base_tree:
        tree_payload["base_tree"] = base_tree
    tree_result = api(f"repos/{OWNER}/{REPO}/git/trees", "POST", tree_payload)

    message = "Publish OpenList Explorer source"
    commit_payload = {"message": message, "tree": tree_result["sha"]}
    if parent_sha:
        commit_payload["parents"] = [parent_sha]
    new_commit = api(f"repos/{OWNER}/{REPO}/git/commits", "POST", commit_payload)

    if parent_sha:
        api(f"repos/{OWNER}/{REPO}/git/refs/heads/{BRANCH}", "PATCH", {"sha": new_commit["sha"]})
    else:
        api(f"repos/{OWNER}/{REPO}/git/refs", "POST", {"ref": f"refs/heads/{BRANCH}", "sha": new_commit["sha"]})

    print(f"Published source commit: {new_commit['sha']}")
    return new_commit["sha"]


def ensure_release(commit_sha: str):
    release = api_optional(f"repos/{OWNER}/{REPO}/releases/tags/{TAG}")
    body = (
        "OpenList Explorer desktop installer.\n\n"
        "- Windows x64 NSIS installer\n"
        "- Includes bundled OpenList sidecar\n"
        "- Source repository excludes the large OpenList binary sidecar because GitHub normal git storage has a 100MB file limit\n"
    )
    if release:
        return release
    return api(
        f"repos/{OWNER}/{REPO}/releases",
        "POST",
        {
            "tag_name": TAG,
            "target_commitish": commit_sha,
            "name": f"OpenList Explorer {APP_VERSION}",
            "body": body,
            "draft": False,
            "prerelease": True,
        },
    )


def upload_installer(release):
    if not INSTALLER.exists():
        raise RuntimeError(f"Installer not found: {INSTALLER}")

    asset_name = INSTALLER.name
    assets = api(f"repos/{OWNER}/{REPO}/releases/{release['id']}/assets")
    for asset in assets:
        if asset["name"] == asset_name:
            print(f"Delete existing release asset: {asset_name}")
            api(f"repos/{OWNER}/{REPO}/releases/assets/{asset['id']}", "DELETE")

    upload_url = release["upload_url"].split("{", 1)[0]
    query = urllib.parse.urlencode({"name": asset_name})
    url = f"{upload_url}?{query}"
    mime = mimetypes.guess_type(asset_name)[0] or "application/octet-stream"
    req = urllib.request.Request(
        url,
        data=INSTALLER.read_bytes(),
        method="POST",
        headers={
            "Authorization": f"Bearer {TOKEN}",
            "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
            "User-Agent": "openlist-explorer-publish-script",
            "Content-Type": mime,
        },
    )
    with urllib.request.urlopen(req) as response:
        asset = json.loads(response.read().decode("utf-8"))
    print(f"Uploaded installer asset: {asset['browser_download_url']}")


def main():
    os.chdir(ROOT)
    commit_sha = publish_source()
    release = ensure_release(commit_sha)
    upload_installer(release)
    print(f"Release URL: {release['html_url']}")


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(f"Publish failed: {error}", file=sys.stderr)
        sys.exit(1)
