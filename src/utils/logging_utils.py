import os
import traceback
from datetime import datetime

LOG_DIR = os.path.join(os.getcwd(), "logs")

def ensure_log_dir():
    try:
        os.makedirs(LOG_DIR, exist_ok=True)
    except Exception:
        pass

def log_exception(exc_info=None, prefix="crash"):
    """Write exception traceback to a timestamped crash log file.
    exc_info: as returned by sys.exc_info() or None to use current.
    Returns the filename of the written log (or None if failed).
    """
    ensure_log_dir()
    if exc_info is None:
        exc_info = None
    try:
        now = datetime.utcnow().strftime("%Y%m%d-%H%M%S")
        filename = os.path.join(LOG_DIR, f"{prefix}-{now}.log")
        with open(filename, "w", encoding="utf-8") as f:
            f.write("Crash log - UTC Time: " + now + "\n\n")
            if exc_info is None:
                import traceback as _tb
                f.write(_tb.format_exc())
            else:
                f.write("".join(traceback.format_exception(*exc_info)))
        return filename
    except Exception:
        return None

def log_message(msg, filename="application.log"):
    ensure_log_dir()
    try:
        path = os.path.join(LOG_DIR, filename)
        with open(path, "a", encoding="utf-8") as f:
            f.write(f"{datetime.utcnow().isoformat()} - {msg}\n")
        return path
    except Exception:
        return None
