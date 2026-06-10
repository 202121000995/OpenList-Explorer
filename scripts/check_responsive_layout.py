import http.server
import os
import shutil
import socketserver
import subprocess
import threading
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DIST = ROOT / "dist"
OUTPUT = ROOT / "src-tauri" / "target" / "responsive-layout"
PAGES = ["/files", "/uploads", "/downloads", "/settings", "/openlist"]
WIDTHS = [760, 1024, 1280]
HEIGHT = 720


class SpaHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(DIST), **kwargs)

    def log_message(self, *_args):
        pass

    def handle(self):
        try:
            super().handle()
        except (BrokenPipeError, ConnectionAbortedError, ConnectionResetError):
            pass

    def send_head(self):
        path = self.translate_path(self.path)
        if not Path(path).exists():
            self.path = "/index.html"
        return super().send_head()


def find_browser() -> str:
    env_browser = os.environ.get("BROWSER_EXE", "").strip()
    candidates = [
        env_browser,
        shutil.which("msedge"),
        shutil.which("chrome"),
        shutil.which("chromium"),
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    ]
    for candidate in candidates:
        if candidate and Path(candidate).exists():
            return str(candidate)
    raise RuntimeError("Chrome/Edge was not found. Set BROWSER_EXE to enable responsive screenshots.")


def start_server():
    server = socketserver.TCPServer(("127.0.0.1", 0), SpaHandler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    return server


def screenshot(browser: str, url: str, output: Path, width: int):
    output.parent.mkdir(parents=True, exist_ok=True)
    user_data_dir = OUTPUT / "browser-profile"
    command = [
        browser,
        "--headless=new",
        "--disable-gpu",
        "--hide-scrollbars",
        "--no-first-run",
        "--no-default-browser-check",
        "--virtual-time-budget=5000",
        f"--user-data-dir={user_data_dir}",
        f"--window-size={width},{HEIGHT}",
        f"--screenshot={output}",
        url,
    ]
    result = subprocess.run(command, capture_output=True, text=True, timeout=30, check=False)
    if result.returncode != 0:
        raise RuntimeError(result.stderr.strip() or result.stdout.strip() or f"browser exited {result.returncode}")
    if not output.exists() or output.stat().st_size < 2 * 1024:
        raise RuntimeError(f"screenshot is missing or too small: {output}")


def main() -> int:
    if not DIST.exists():
        print("[FAIL] dist does not exist. Run npm run build first.")
        return 1

    browser = find_browser()
    OUTPUT.mkdir(parents=True, exist_ok=True)
    server = start_server()
    base = f"http://127.0.0.1:{server.server_address[1]}"
    time.sleep(0.2)

    try:
        for width in WIDTHS:
            for page in PAGES:
                filename = f"{page.strip('/') or 'home'}-{width}.png"
                target = OUTPUT / filename
                screenshot(browser, f"{base}{page}", target, width)
                print(f"[PASS] {page} {width}px -> {target}")
    finally:
        server.shutdown()
        server.server_close()

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
