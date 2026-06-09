from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import urllib.request
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
TAURI_DIR = ROOT / "src-tauri"
PROJECT_CARGO_DIR = TAURI_DIR / ".cargo"
PROJECT_CARGO_CONFIG = PROJECT_CARGO_DIR / "config.toml"


RS_PROXY_CONFIG = """[source.crates-io]
replace-with = "rsproxy-sparse"

[source.rsproxy-sparse]
registry = "sparse+https://rsproxy.cn/index/"

[http]
check-revoke = false
timeout = 120

[net]
retry = 10
"""


DEFAULT_CARGO_CONFIG = """[http]
check-revoke = false
timeout = 120

[net]
retry = 10
"""


def run(command: list[str], cwd: Path = ROOT, env: dict[str, str] | None = None) -> None:
    print(f"\n> {' '.join(command)}")
    merged_env = os.environ.copy()
    if env:
        merged_env.update(env)

    completed = subprocess.run(command, cwd=str(cwd), env=merged_env)
    if completed.returncode != 0:
        raise SystemExit(completed.returncode)


def tool_exists(name: str) -> bool:
    return shutil.which(name) is not None


def request_status(url: str) -> str:
    try:
        request = urllib.request.Request(url, method="HEAD")
        with urllib.request.urlopen(request, timeout=12) as response:
            return f"OK {response.status}"
    except Exception as exc:
        return f"FAILED {exc}"


def ensure_tools() -> None:
    missing = [name for name in ("rustup", "cargo", "rustc", "npm.cmd") if not tool_exists(name)]
    if missing:
        raise SystemExit(f"Missing tools: {', '.join(missing)}")


def clear_cargo_cache_locks() -> None:
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    for name in (".package-cache", ".package-cache-mutate"):
        path = cargo_home / name
        if path.exists() and path.stat().st_size == 0:
            print(f"Removing stale Cargo cache lock: {path}")
            path.unlink()


def write_cargo_config(mirror: str) -> None:
    PROJECT_CARGO_DIR.mkdir(parents=True, exist_ok=True)
    content = RS_PROXY_CONFIG if mirror == "rsproxy" else DEFAULT_CARGO_CONFIG
    PROJECT_CARGO_CONFIG.write_text(content, encoding="utf-8")
    print(f"Wrote Cargo config: {PROJECT_CARGO_CONFIG}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Prepare and verify Tauri desktop build dependencies.")
    parser.add_argument(
        "--toolchain",
        default="1.88.0",
        help="Rust toolchain to use for this project. Default: 1.88.0",
    )
    parser.add_argument(
        "--mirror",
        choices=("none", "rsproxy"),
        default="rsproxy",
        help="Cargo registry mirror to use. Default: rsproxy",
    )
    parser.add_argument(
        "--package",
        action="store_true",
        help="Run npm.cmd run tauri:build after cargo check passes.",
    )
    parser.add_argument(
        "--proxy",
        default="",
        help="Proxy URL for Cargo/npm downloads, for example http://127.0.0.1:7890",
    )
    parser.add_argument(
        "--skip-fetch",
        action="store_true",
        help="Skip cargo fetch and run cargo check directly.",
    )
    args = parser.parse_args()

    print(f"Project: {ROOT}")
    print(f"Tauri:   {TAURI_DIR}")
    ensure_tools()

    print("\nNetwork probe:")
    print(f"  crates.io: {request_status('https://index.crates.io/config.json')}")
    print(f"  rsproxy:   {request_status('https://rsproxy.cn/index/config.json')}")

    clear_cargo_cache_locks()
    write_cargo_config(args.mirror)

    installed_toolchains = subprocess.run(
        ["rustup", "toolchain", "list"],
        cwd=str(ROOT),
        capture_output=True,
        text=True,
        check=False,
    ).stdout
    if args.toolchain not in installed_toolchains:
        run(["rustup", "toolchain", "install", args.toolchain], cwd=ROOT)

    run(["rustup", "override", "set", args.toolchain], cwd=ROOT)
    run(["rustc", "--version"], cwd=ROOT)
    run(["cargo", "--version"], cwd=ROOT)

    cargo_env = {
        "CARGO_HTTP_CHECK_REVOKE": "false",
        "CARGO_HTTP_TIMEOUT": "120",
    }
    if args.proxy:
        cargo_env.update(
            {
                "HTTP_PROXY": args.proxy,
                "HTTPS_PROXY": args.proxy,
                "ALL_PROXY": args.proxy,
                "NO_PROXY": "localhost,127.0.0.1,::1",
            }
        )
        print(f"\nUsing proxy for download commands: {args.proxy}")

    if not args.skip_fetch:
        run(["cargo", "fetch"], cwd=TAURI_DIR, env=cargo_env)

    run(["cargo", "check", "--locked"], cwd=TAURI_DIR, env=cargo_env)
    run(["npm.cmd", "run", "build"], cwd=ROOT)

    if args.package:
        run(["npm.cmd", "run", "tauri:build"], cwd=ROOT, env=cargo_env)
    else:
        print("\nReady for packaging. To build the desktop installer, rerun with --package.")


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\nInterrupted by user.")
        raise SystemExit(130)
