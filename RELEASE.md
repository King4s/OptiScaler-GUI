# Release Procedure

Follow these steps to create a new release for OptiScaler-GUI.

1) Bump version in `src/__version__.py`.
2) Update `CHANGELOG.md` with a new entry for the version and date.
3) Ensure all tests pass:

```pwsh
py -3 -m pytest -q
```

4) Build the portable executable (Windows):

```pwsh
py -3 build.py
# or use start_gui and test: start_gui.bat
```

5) Tag & push the release (example using git):

```pwsh
git add -A
git commit -m "chore(release): v0.4.0"
git tag -a v0.4.0 -m "Release v0.4.0"
git push origin HEAD --tags
```

6) Create a GitHub release via the web UI or CLI, attaching the built portable/exe artifacts. Use the `CHANGELOG.md` entry as the release notes.

7) Update the `README.md` and an `assets/` banner if necessary.
8) Announce the release and update the release badge / description as needed.

Notes:
- Releases should include compiled artifacts for Windows (e.g., `OptiScaler-GUI.exe`) built via `build.py`/PyInstaller.
- Always run a full test suite before tagging and building artifacts.
- If distributing via portable ZIP, verify the built artifact size and that `tools` folder contains `7z.exe` if needed.
