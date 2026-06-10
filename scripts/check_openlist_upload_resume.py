import json
import os
import sys
import time
from pathlib import Path
from urllib import parse, request

import e2e_openlist_smoke as smoke


def upload_bytes(remote_path: str, filename: str, data: bytes, content_range: str):
    boundary = f"----OpenListExplorerResume{int(time.time() * 1000)}"
    body = b"".join(
        [
            f"--{boundary}\r\n".encode(),
            f'Content-Disposition: form-data; name="file"; filename="{filename}"\r\n'.encode(),
            b"Content-Type: application/octet-stream\r\n\r\n",
            data,
            b"\r\n",
            f"--{boundary}--\r\n".encode(),
        ]
    )
    return smoke.api(
        "PUT",
        "/api/fs/form",
        headers={
            "Content-Type": f"multipart/form-data; boundary={boundary}",
            "File-Path": parse.quote(remote_path, safe="/"),
            "Content-Range": content_range,
        },
        raw_body=body,
    )


def main():
    if not smoke.TOKEN:
        smoke.TOKEN = smoke.load_token_from_openlist_bin()
    if not smoke.TOKEN:
        print("Set OPENLIST_TOKEN, or set OPENLIST_BIN with OPENLIST_FORCE_BIN_DIR=1.")
        return 2

    parent = os.environ.get("OPENLIST_E2E_PARENT", "").strip()
    candidates = [parent] if parent else smoke.parent_candidates()
    payload = b"OpenListResumeProbe"
    first = payload[:8]
    second = payload[8:]
    errors = []

    for candidate in candidates:
        run_id = f"_openlist_resume_probe_{int(time.time())}"
        root = smoke.join_remote(candidate, run_id)
        remote_path = smoke.join_remote(root, "resume.bin")
        try:
            smoke.api("POST", "/api/fs/mkdir", {"path": root})
            upload_bytes(remote_path, "resume.bin", first, f"bytes 0-{len(first) - 1}/{len(payload)}")
            upload_bytes(remote_path, "resume.bin", second, f"bytes {len(first)}-{len(payload) - 1}/{len(payload)}")
            detail = smoke.api("POST", "/api/fs/get", {"path": remote_path})
            raw_url = detail.get("raw_url") if isinstance(detail, dict) else ""
            if not raw_url:
                raise RuntimeError("raw_url is empty")
            with request.urlopen(raw_url, timeout=30) as response:
                actual = response.read()
            if actual == payload:
                print(f"[PASS] OpenList upload byte resume appears supported on parent: {candidate}")
                return 0
            print(f"[FAIL] OpenList upload byte resume is not supported on parent: {candidate}")
            print(f"Expected {len(payload)} bytes assembled from two ranges, got {len(actual)} bytes.")
            return 1
        except Exception as error:
            errors.append(f"{candidate}: {error}")
        finally:
            try:
                smoke.api("POST", "/api/fs/remove", {"dir": root, "names": ["resume.bin"]})
                smoke.api("POST", "/api/fs/remove", {"dir": candidate, "names": [run_id]})
            except Exception:
                pass

    print("[FAIL] Could not run upload resume probe:")
    print("\n".join(errors))
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
