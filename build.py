#!/usr/bin/env python3
"""
Build the OptiScaler-GUI portable Windows distribution.

This script intentionally has one build path:
1. clean build outputs,
2. run PyInstaller with build_executable.spec,
3. copy runtime files that must live next to the executable,
4. optionally create the release ZIP.
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path


PROJECT_ROOT = Path(__file__).resolve().parent
DIST_DIR = PROJECT_ROOT / "dist"
BUILD_DIR = PROJECT_ROOT / "build"
PORTABLE_DIR = DIST_DIR / "OptiScaler-GUI-Portable"
SPEC_FILE = PROJECT_ROOT / "build_executable.spec"


def run(command: list[str]) -> None:
    subprocess.run(command, cwd=PROJECT_ROOT, check=True)


def ensure_pyinstaller() -> None:
    check = subprocess.run(
        [sys.executable, "-m", "PyInstaller", "--version"],
        cwd=PROJECT_ROOT,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    if check.returncode == 0:
        return

    run([sys.executable, "-m", "pip", "install", "--upgrade", "--force-reinstall", "pyinstaller"])

    check = subprocess.run(
        [sys.executable, "-m", "PyInstaller", "--version"],
        cwd=PROJECT_ROOT,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    if check.returncode != 0:
        raise RuntimeError("PyInstaller is installed but cannot be run with this Python interpreter")


def clean() -> None:
    for path in (DIST_DIR, BUILD_DIR):
        if path.exists():
            shutil.rmtree(path)

    for pycache in PROJECT_ROOT.rglob("__pycache__"):
        if pycache.is_dir():
            shutil.rmtree(pycache)


def find_7z() -> Path | None:
    candidates = [
        PROJECT_ROOT / "7z.exe",
        Path(r"C:\Program Files\7-Zip\7z.exe"),
        Path(r"C:\Program Files (x86)\7-Zip\7z.exe"),
    ]
    for candidate in candidates:
        if candidate.exists():
            return candidate

    path_match = shutil.which("7z")
    return Path(path_match) if path_match else None


def build() -> None:
    if not SPEC_FILE.exists():
        raise FileNotFoundError(f"Missing PyInstaller spec: {SPEC_FILE}")

    ensure_pyinstaller()
    run([sys.executable, "-m", "PyInstaller", str(SPEC_FILE), "--clean", "--noconfirm"])

    if not PORTABLE_DIR.exists():
        raise FileNotFoundError(f"Expected portable output was not created: {PORTABLE_DIR}")

    (PORTABLE_DIR / "cache").mkdir(exist_ok=True)

    for filename in ("README.md", "CHANGELOG.md", "requirements.txt"):
        src = PROJECT_ROOT / filename
        if src.exists():
            shutil.copy2(src, PORTABLE_DIR / filename)

    seven_zip = find_7z()
    if not seven_zip:
        raise FileNotFoundError(
            "7z.exe is required for current OptiScaler .7z archives. "
            "Install 7-Zip or place 7z.exe in the repository root before building."
        )
    shutil.copy2(seven_zip, PORTABLE_DIR / "7z.exe")


def make_zip(version: str | None) -> Path:
    tag = version or "dev"
    archive_base = DIST_DIR / f"OptiScaler-GUI-{tag}-Portable"
    archive_path = Path(shutil.make_archive(str(archive_base), "zip", DIST_DIR, PORTABLE_DIR.name))
    return archive_path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build OptiScaler-GUI portable distribution")
    parser.add_argument("--no-clean", action="store_true", help="Do not remove build/ and dist/ first")
    parser.add_argument("--zip", action="store_true", help="Create a portable ZIP after building")
    parser.add_argument("--version", help="Version/tag for ZIP filename, e.g. v0.4.3")
    return parser.parse_args()


def main() -> None:
    args = parse_args()

    if not (PROJECT_ROOT / "src" / "main.py").exists():
        raise FileNotFoundError("Run build.py from the OptiScaler-GUI repository root")

    if not args.no_clean:
        clean()

    build()

    print(f"Portable build: {PORTABLE_DIR}")
    print(f"Executable: {PORTABLE_DIR / 'OptiScaler-GUI.exe'}")
    print(f"Bundled 7z.exe: {(PORTABLE_DIR / '7z.exe').exists()}")

    if args.zip:
        archive = make_zip(args.version)
        size_mb = archive.stat().st_size / (1024 * 1024)
        print(f"ZIP: {archive} ({size_mb:.1f} MB)")


if __name__ == "__main__":
    main()
