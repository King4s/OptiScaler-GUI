"""
PyInstaller runtime hook - runs before any app code.
Ensures _MEIPASS is on sys.path and logs diagnostic info.
"""
import sys
import os

def _setup_and_log():
    meipass = getattr(sys, '_MEIPASS', None)
    frozen = getattr(sys, 'frozen', False)

    log_lines = [
        f"[rthook] frozen={frozen}",
        f"[rthook] _MEIPASS={meipass}",
        f"[rthook] sys.path={sys.path[:6]}",
    ]

    if meipass:
        # Ensure _MEIPASS is first on sys.path
        if meipass not in sys.path:
            sys.path.insert(0, meipass)
            log_lines.append(f"[rthook] Inserted _MEIPASS into sys.path")

        # Check for expected package dirs
        import pathlib
        for pkg in ('scanner', 'utils', 'gui', 'optiscaler'):
            p = pathlib.Path(meipass) / pkg
            has_init = (p / '__init__.py').exists()
            log_lines.append(f"[rthook] {pkg}/ exists={p.exists()} has_init={has_init}")
            if p.exists():
                files = list(p.iterdir())
                log_lines.append(f"[rthook]   files: {[f.name for f in files[:10]]}")

    # Try importing the problematic module directly to get the real error
    import traceback
    try:
        import scanner.game_scanner
        log_lines.append("[rthook] scanner.game_scanner import: OK")
        log_lines.append(f"[rthook] scanner in sys.modules: {'scanner' in sys.modules}")
        log_lines.append(f"[rthook] scanner.game_scanner in sys.modules: {'scanner.game_scanner' in sys.modules}")
        # Log any error that occurred during import (stored in __spec__ or similar)
        import scanner
        log_lines.append(f"[rthook] scanner.__path__: {getattr(scanner, '__path__', 'NO __path__')}")
        log_lines.append(f"[rthook] scanner.__spec__: {getattr(scanner, '__spec__', 'NO __spec__')}")
    except Exception as e:
        log_lines.append(f"[rthook] scanner.game_scanner import FAILED: {type(e).__name__}: {e}")
        log_lines.append("[rthook] Full traceback:")
        log_lines.append(traceback.format_exc())

    # Write diagnostics to a log file next to the exe
    try:
        exe_dir = pathlib.Path(sys.executable).parent
        log_path = exe_dir / 'rthook_diag.log'
        with open(log_path, 'w', encoding='utf-8') as f:
            f.write('\n'.join(log_lines) + '\n')
    except Exception as e:
        pass  # Never crash in a runtime hook

_setup_and_log()
