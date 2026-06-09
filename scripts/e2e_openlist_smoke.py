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


def run_smoke(parent: str):
    run_id = f"_openlist_explorer_e2e_{int(time.time())}"
    root = join_remote(parent, run_id)
    upload_path = join_remote(root, "sample.txt")
    renamed_path = join_remote(root, "renamed.txt")
    sample = Path(os.environ.get("TEMP", ".")).joinpath(f"{run_id}.txt")
    sample.write_text("OpenList Explorer smoke test\n", encoding="utf-8")

    try:
        print(f"[1/8] mkdir {root}")
        api("POST", "/api/fs/mkdir", {"path": root})

        print(f"[2/8] upload {upload_path}")
        multipart_upload(sample, upload_path)

        print(f"[3/8] list {root}")
        listed = api("POST", "/api/fs/list", {"path": root, "page": 1, "per_page": 20, "refresh": True})
        names = [item.get("name") for item in listed.get("content", [])]
        if "sample.txt" not in names:
            fail("uploaded file was not found in list response")

        print("[4/8] rename")
        api("POST", "/api/fs/rename", {"path": upload_path, "name": "renamed.txt"})

        print("[5/8] search")
        try:
            api("POST", "/api/fs/search", {"parent": root, "keywords": "renamed", "page": 1, "per_page": 20})
        except Exception as error:
            print(f"[WARN] search unavailable on this storage: {error}")

        print("[6/8] get raw url")
        detail = api("POST", "/api/fs/get", {"path": renamed_path})
        raw_url = detail.get("raw_url") if isinstance(detail, dict) else ""

        print("[7/8] browser download probe")
        if raw_url:
            with request.urlopen(raw_url, timeout=30) as response:
                if response.status >= 400:
                    fail(f"raw url returned HTTP {response.status}")
        else:
            print("[WARN] raw_url is empty; skipping direct download probe")

        print("[8/8] delete")
        api("POST", "/api/fs/remove", {"dir": root, "names": ["renamed.txt"]})
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
