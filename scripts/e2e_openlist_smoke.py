import json
import os
import sys
import time
import uuid
from pathlib import Path
from urllib import request


BASE_URL = os.environ.get("OPENLIST_URL", "http://127.0.0.1:5244").rstrip("/")
TOKEN = os.environ.get("OPENLIST_TOKEN", "").strip()
PARENT = os.environ.get("OPENLIST_E2E_PARENT", "/").strip() or "/"


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
            "File-Path": remote_path,
        },
        raw_body=body,
    )


def join_remote(*parts: str):
    text = "/".join(part.strip("/") for part in parts if part.strip("/"))
    return f"/{text}" if text else "/"


def main():
    if not TOKEN:
        print("Set OPENLIST_TOKEN before running this smoke test.")
        print("Optional: OPENLIST_URL=http://127.0.0.1:5244 OPENLIST_E2E_PARENT=/SomeMount")
        return 2

    run_id = f"_openlist_explorer_e2e_{int(time.time())}"
    root = join_remote(PARENT, run_id)
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
        api("POST", "/api/fs/search", {"parent": root, "keywords": "renamed", "page": 1, "per_page": 20})

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
        api("POST", "/api/fs/remove", {"dir": PARENT, "names": [run_id]})
        print("[OK] OpenList smoke test passed")
        return 0
    finally:
        sample.unlink(missing_ok=True)


if __name__ == "__main__":
    raise SystemExit(main())
