import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
TAURI_CONF = ROOT / "src-tauri" / "tauri.conf.json"
PACKAGE_JSON = ROOT / "package.json"
CARGO_TOML = ROOT / "src-tauri" / "Cargo.toml"
OPENLIST_BIN = ROOT / "src-tauri" / "binaries" / "openlist-x86_64-pc-windows-msvc.exe"
ARIA2_BIN = ROOT / "src-tauri" / "binaries" / "aria2c-x86_64-pc-windows-msvc.exe"


def result(status: str, message: str):
    print(f"[{status}] {message}")


def run(command: list[str], timeout=60):
    return subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=timeout, check=False)


def check(condition: bool, message: str) -> bool:
    result("PASS" if condition else "FAIL", message)
    return condition


def main():
    failures = 0
    package = json.loads(PACKAGE_JSON.read_text(encoding="utf-8"))
    tauri = json.loads(TAURI_CONF.read_text(encoding="utf-8"))
    cargo_toml = CARGO_TOML.read_text(encoding="utf-8")
    version = package["version"]

    failures += not check(version == tauri["version"], f"package.json and tauri.conf.json version match: {version}")
    failures += not check(f'version = "{version}"' in cargo_toml, f"Cargo.toml version matches: {version}")

    external = set(tauri.get("bundle", {}).get("externalBin", []))
    failures += not check("binaries/openlist" in external, "Tauri externalBin includes bundled OpenList")
    failures += not check("binaries/aria2c" in external, "Tauri externalBin includes bundled Aria2")
    failures += not check(OPENLIST_BIN.exists(), f"OpenList sidecar exists: {OPENLIST_BIN}")
    failures += not check(ARIA2_BIN.exists(), f"Aria2 sidecar exists: {ARIA2_BIN}")

    installer = ROOT / "src-tauri" / "target" / "release" / "bundle" / "nsis" / f"OpenList Explorer_{version}_x64-setup.exe"
    failures += not check(installer.exists(), f"Installer exists: {installer}")

    if OPENLIST_BIN.exists():
        version_run = run([str(OPENLIST_BIN), "version"])
        failures += not check(version_run.returncode == 0 and "Version:" in version_run.stdout, "OpenList sidecar can execute")

        temp_dir = Path(tempfile.mkdtemp(prefix="openlist-explorer-p0-"))
        try:
            token_run = run([str(OPENLIST_BIN), "admin", "token", "--data", str(temp_dir)])
            has_token = "Admin token:" in token_run.stdout
            has_password = "initial password" in token_run.stdout
            failures += not check(token_run.returncode == 0 and has_token and has_password, "OpenList first-run admin token/password initialization works")
        finally:
            shutil.rmtree(temp_dir, ignore_errors=True)

    if ARIA2_BIN.exists():
        aria2_run = run([str(ARIA2_BIN), "--version"])
        failures += not check(aria2_run.returncode == 0 and "aria2 version" in aria2_run.stdout, "Aria2 sidecar can execute")

    multi_openlist = run([str(ROOT / "scripts" / "check_multi_openlist_state.cmd")], timeout=60)
    failures += not check(multi_openlist.returncode == 0, "Multi OpenList instance isolation checks pass")
    for line in multi_openlist.stdout.splitlines():
        print(line)
    for line in multi_openlist.stderr.splitlines():
        print(line, file=sys.stderr)

    has_e2e_inputs = bool(os.environ.get("OPENLIST_TOKEN") or os.environ.get("OPENLIST_BIN"))
    if has_e2e_inputs:
        e2e = run([str(ROOT / "scripts" / "e2e_openlist_smoke.cmd")], timeout=180)
        if e2e.returncode == 0 and "[OK] OpenList smoke test passed" in e2e.stdout:
            result("PASS", "Real OpenList API smoke test passed")
        else:
            failures += 1
            result("FAIL", "Real OpenList API smoke test failed")
            for line in e2e.stdout.splitlines():
                if "token" not in line.lower():
                    print(line)
            for line in e2e.stderr.splitlines():
                if "token" not in line.lower():
                    print(line, file=sys.stderr)

        resume = run([str(ROOT / "scripts" / "check_openlist_upload_resume.cmd")], timeout=180)
        require_resume = os.environ.get("OPENLIST_REQUIRE_UPLOAD_RESUME", "").strip().lower() in {"1", "true", "yes"}
        if resume.returncode == 0:
            result("PASS", "OpenList upload byte resume probe passed")
        else:
            if require_resume:
                failures += 1
                result("FAIL", "OpenList upload byte resume probe failed")
            else:
                result("WARN", "OpenList upload byte resume is unavailable; Explorer falls back to task-level retry")
            for line in resume.stdout.splitlines():
                if "token" not in line.lower():
                    print(line)
            for line in resume.stderr.splitlines():
                if "token" not in line.lower():
                    print(line, file=sys.stderr)
    else:
        result("WARN", "Real OpenList API smoke test skipped; set OPENLIST_TOKEN or OPENLIST_BIN")

    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
