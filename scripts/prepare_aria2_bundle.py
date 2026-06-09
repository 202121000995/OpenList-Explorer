import argparse
import json
import shutil
import sys
import tempfile
import zipfile
from pathlib import Path
from urllib import request


ROOT = Path(__file__).resolve().parents[1]
BIN_DIR = ROOT / "src-tauri" / "binaries"
TARGET = BIN_DIR / "aria2c-x86_64-pc-windows-msvc.exe"
TAURI_CONF = ROOT / "src-tauri" / "tauri.conf.json"
LATEST_API = "https://api.github.com/repos/aria2/aria2/releases/latest"


def download_latest_zip(destination: Path) -> Path:
    print("Querying latest aria2 release...")
    req = request.Request(LATEST_API, headers={"User-Agent": "openlist-explorer-aria2-bundler"})
    with request.urlopen(req, timeout=60) as response:
        release = json.loads(response.read().decode("utf-8"))

    assets = release.get("assets", [])
    candidates = [
        asset for asset in assets
        if "win-64bit" in asset.get("name", "").lower()
        and asset.get("name", "").lower().endswith(".zip")
    ]
    if not candidates:
        raise RuntimeError("Could not find a Windows 64-bit aria2 zip asset in the latest release.")

    asset = candidates[0]
    url = asset["browser_download_url"]
    zip_path = destination / asset["name"]
    print(f"Downloading {asset['name']}...")
    request.urlretrieve(url, zip_path)
    return zip_path


def extract_aria2(zip_path: Path):
    with zipfile.ZipFile(zip_path) as archive:
        names = [name for name in archive.namelist() if name.lower().endswith("aria2c.exe")]
        if not names:
            raise RuntimeError("aria2c.exe was not found in the zip.")
        BIN_DIR.mkdir(parents=True, exist_ok=True)
        with archive.open(names[0]) as source, TARGET.open("wb") as target:
            shutil.copyfileobj(source, target)
    print(f"Copied: {TARGET}")


def patch_tauri_config():
    config = json.loads(TAURI_CONF.read_text(encoding="utf-8"))
    bundle = config.setdefault("bundle", {})
    external = bundle.setdefault("externalBin", [])
    if "binaries/aria2c" not in external:
        external.append("binaries/aria2c")
    TAURI_CONF.write_text(json.dumps(config, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print("Updated tauri.conf.json externalBin with binaries/aria2c")


def main():
    parser = argparse.ArgumentParser(description="Prepare aria2c.exe for OpenList Explorer packaging.")
    parser.add_argument("--zip", type=Path, help="Use an existing aria2 Windows x64 zip instead of downloading.")
    args = parser.parse_args()

    if args.zip:
        zip_path = args.zip
        if not zip_path.exists():
            raise SystemExit(f"Zip not found: {zip_path}")
        extract_aria2(zip_path)
    else:
        with tempfile.TemporaryDirectory() as temp_dir:
            zip_path = download_latest_zip(Path(temp_dir))
            extract_aria2(zip_path)

    patch_tauri_config()
    print("Done. Rebuild with: npm.cmd run tauri:build")


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(f"Failed: {error}", file=sys.stderr)
        raise SystemExit(1)
