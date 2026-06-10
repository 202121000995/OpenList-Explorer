import json
import os
import subprocess
import sys
import time
import uuid
from pathlib import Path
from urllib import parse, request


BASE_URL = os.environ.get("OPENLIST_URL", "http://127.0.0.1:5244").rstrip("/")
TOKEN = os.environ.get("OPENLIST_TOKEN", "").strip()
PARENT = os.environ.get("OPENLIST_E2E_PARENT", "").strip()
REQUIRE_CLOUD = os.environ.get("OPENLIST_E2E_REQUIRE_CLOUD", "").strip().lower() in {"1", "true", "yes"}


def fail(message: str):
    print(f"[FAIL] {message}", file=sys.stderr)
    raise SystemExit(1)


def api(method: str, path: str, payload=None, headers=None, raw_body=None):
    body = raw_body
    req_headers = {"Authorization": TOKEN}
    if headers:
        req_headers.update(headers)
    if payload is not None:
        body = json.dumps(payload).encode("utf-8")
        req_headers["Content-Type"] = "application/json"

    req = request.Request(f"{BASE_URL}{path}", data=body, headers=req_headers, method=method)
    with request.urlopen(req, timeout=30) as response:
        data = response.read()
    if not data:
        return None
    result = json.loads(data.decode("utf-8"))
    if isinstance(result, dict) and result.get("code") not in (None, 200):
        raise RuntimeError(result.get("message") or result.get("msg") or result)
    return result.get("data") if isinstance(result, dict) and "data" in result else result


def load_token_from_openlist_bin() -> str:
    binary = os.environ.get("OPENLIST_BIN", "").strip()
    data_dir = os.environ.get("OPENLIST_DATA", "").strip()
    force_bin_dir = os.environ.get("OPENLIST_FORCE_BIN_DIR", "").strip().lower() in {"1", "true", "yes"}
    if not binary:
        return ""

    command = [binary, "admin", "token"]
    if force_bin_dir:
        command.append("--force-bin-dir")
    if data_dir:
        command.extend(["--data", data_dir])

    result = subprocess.run(command, capture_output=True, text=True, check=False)
    if result.returncode != 0:
        return ""

    for line in result.stdout.splitlines():
        if line.strip().startswith("Admin token:"):
            return line.split(":", 1)[1].strip()
    return ""


def multipart_upload(file_path: Path, remote_path: str):
    boundary = f"----OpenListExplorerE2E{uuid.uuid4().hex}"
    file_bytes = file_path.read_bytes()
    body = b"".join(
        [
            f"--{boundary}\r\n".encode(),
            f'Content-Disposition: form-data; name="file"; filename="{file_path.name}"\r\n'.encode(),
            b"Content-Type: application/octet-stream\r\n\r\n",
            file_bytes,
            b"\r\n",
            f"--{boundary}--\r\n".encode(),
        ]
    )
    return api(
        "PUT",
        "/api/fs/form",
        headers={
            "Content-Type": f"multipart/form-data; boundary={boundary}",
            "File-Path": parse.quote(remote_path, safe="/"),
        },
        raw_body=body,
    )


def join_remote(*parts: str):
    text = "/".join(part.strip("/") for part in parts if part.strip("/"))
    return f"/{text}" if text else "/"


def parent_candidates():
    if PARENT:
        return [PARENT]

    candidates = []
    try:
        root = api("POST", "/api/fs/list", {"path": "/", "page": 1, "per_page": 200, "refresh": True})
        for item in root.get("content", []):
            if item.get("is_dir") and item.get("name"):
                candidates.append(join_remote(item["name"]))
    except Exception as error:
        print(f"[WARN] Could not list root mounts: {error}")

    candidates.append("/")
    return candidates


def cleanup(parent: str, run_id: str):
    try:
        api("POST", "/api/fs/remove", {"dir": parent, "names": [run_id]})
    except Exception:
        pass


def list_names(path: str):
    listed = api("POST", "/api/fs/list", {"path": path, "page": 1, "per_page": 200, "refresh": True})
    return [item.get("name") for item in (listed.get("content") or [])]


def require_name(path: str, name: str):
    names = list_names(path)
    if name not in names:
        fail(f"{name} was not found in {path}; names={names}")


def require_missing(path: str, name: str):
    names = list_names(path)
    if name in names:
        fail(f"{name} should not exist in {path}; names={names}")


def offline_tools():
    try:
        value = api("GET", "/api/public/offline_download_tools")
        return value if isinstance(value, list) else []
    except Exception as error:
        print(f"[WARN] could not query offline download tools: {error}")
        return []


def task_items(value):
    if isinstance(value, list):
        return value
    if isinstance(value, dict) and isinstance(value.get("tasks"), list):
        return value["tasks"]
    return []


def task_id(task):
    return str(task.get("id") or task.get("tid") or "")


def task_state(task):
    return str(task.get("status") or task.get("state") or task.get("phase") or "").lower()


def run_cloud_download_smoke(root: str, raw_url: str):
    if not raw_url:
        print("[WARN] cloud download smoke skipped: raw_url is empty")
        if REQUIRE_CLOUD:
            fail("cloud download smoke requires a raw_url")
        return

    tools = offline_tools()
    if not tools:
        print("[WARN] cloud download smoke skipped: OpenList has no offline download tools enabled")
        if REQUIRE_CLOUD:
            fail("cloud download smoke requires an enabled OpenList offline download tool")
        return

    target = join_remote(root, "offline")
    print(f"[cloud] submit with {tools[0]} -> {target}")
    try:
        api("POST", "/api/fs/mkdir", {"path": target})
        api(
            "POST",
            "/api/fs/add_offline_download",
            {
                "path": target,
                "urls": [raw_url],
                "tool": tools[0],
                "delete_policy": "delete_never",
            },
        )
    except Exception as error:
        print(f"[WARN] cloud download submit failed with {tools[0]}: {error}")
        if REQUIRE_CLOUD:
            fail(f"cloud download submit failed: {error}")
        return

    seen_task = ""
    for _ in range(12):
        undone = task_items(api("GET", "/api/admin/task/offline_download/undone"))
        done = task_items(api("GET", "/api/admin/task/offline_download/done"))
        candidates = undone + done
        for item in candidates:
            text = json.dumps(item, ensure_ascii=False)
            if raw_url in text or target in text:
                seen_task = task_id(item) or seen_task or "<unknown>"
                state = task_state(item)
                if item in done or "success" in state or "complete" in state or "完成" in state:
                    print(f"[OK] cloud download task reached done list: {seen_task}")
                    return
        time.sleep(2)

    if seen_task:
        print(f"[WARN] cloud download task was submitted but did not finish during smoke window: {seen_task}")
        return
    print("[WARN] cloud download task was submitted but not found in OpenList task lists")
    if REQUIRE_CLOUD:
        fail("cloud download task did not appear in OpenList task lists")


def run_smoke(parent: str):
    run_id = f"_openlist_explorer_e2e_{int(time.time())}"
    root = join_remote(parent, run_id)
    copy_dir = join_remote(root, "copy-target")
    move_dir = join_remote(root, "move-target")
    upload_path = join_remote(root, "sample.txt")
    moved_path = join_remote(move_dir, "renamed.txt")
    sample = Path(os.environ.get("TEMP", ".")).joinpath(f"{run_id}.txt")
    sample.write_text("OpenList Explorer smoke test\n", encoding="utf-8")

    try:
        print(f"[1/12] mkdir {root}")
        api("POST", "/api/fs/mkdir", {"path": root})
        api("POST", "/api/fs/mkdir", {"path": copy_dir})
        api("POST", "/api/fs/mkdir", {"path": move_dir})

        print(f"[2/12] upload {upload_path}")
        multipart_upload(sample, upload_path)

        print(f"[3/12] list {root}")
        require_name(root, "sample.txt")

        print("[4/12] rename")
        api("POST", "/api/fs/rename", {"path": upload_path, "name": "renamed.txt"})
        require_name(root, "renamed.txt")

        print("[5/12] copy")
        api("POST", "/api/fs/copy", {"src_dir": root, "dst_dir": copy_dir, "names": ["renamed.txt"]})
        require_name(copy_dir, "renamed.txt")

        print("[6/12] move")
        api("POST", "/api/fs/move", {"src_dir": root, "dst_dir": move_dir, "names": ["renamed.txt"]})
        require_name(move_dir, "renamed.txt")
        require_missing(root, "renamed.txt")

        print("[7/12] delete copied file")
        api("POST", "/api/fs/remove", {"dir": copy_dir, "names": ["renamed.txt"]})
        require_missing(copy_dir, "renamed.txt")

        print("[8/12] search")
        try:
            api("POST", "/api/fs/search", {"parent": root, "keywords": "renamed", "page": 1, "per_page": 20})
        except Exception as error:
            print(f"[WARN] search unavailable on this storage: {error}")

        print("[9/12] get raw url")
        detail = api("POST", "/api/fs/get", {"path": moved_path})
        raw_url = detail.get("raw_url") if isinstance(detail, dict) else ""

        print("[10/12] browser download probe")
        if raw_url:
            with request.urlopen(raw_url, timeout=30) as response:
                if response.status >= 400:
                    fail(f"raw url returned HTTP {response.status}")
        else:
            print("[WARN] raw_url is empty; skipping direct download probe")

        print("[11/12] cloud download status")
        run_cloud_download_smoke(root, raw_url)

        print("[12/12] delete")
        api("POST", "/api/fs/remove", {"dir": parent, "names": [run_id]})
        print(f"[OK] OpenList smoke test passed on parent: {parent}")
    finally:
        sample.unlink(missing_ok=True)
        cleanup(parent, run_id)


def main():
    global TOKEN
    if not TOKEN:
        TOKEN = load_token_from_openlist_bin()

    if not TOKEN:
        print("Set OPENLIST_TOKEN before running this smoke test.")
        print("Optional: OPENLIST_URL=http://127.0.0.1:5244 OPENLIST_E2E_PARENT=/SomeMount")
        print("Or set OPENLIST_BIN and OPENLIST_DATA to read the admin token from an OpenList data directory.")
        return 2

    errors = []
    for parent in parent_candidates():
        try:
            print(f"[TRY] parent {parent}")
            run_smoke(parent)
            return 0
        except Exception as error:
            errors.append(f"{parent}: {error}")
            print(f"[WARN] parent failed: {parent}: {error}")

    fail("no writable OpenList parent found:\n" + "\n".join(errors))


if __name__ == "__main__":
    raise SystemExit(main())
