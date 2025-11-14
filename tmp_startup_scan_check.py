import sys
sys.path.insert(0, 'src')
from utils.debug import set_debug_enabled
set_debug_enabled(True)

from gui.main_window import MainWindow
import time

mw = MainWindow()
print('Created MainWindow, waiting for scan...')

# Wait a bit to let background scans start
for i in range(5):
    time.sleep(0.5)
    sc = mw.scanner.get_cached_games() if hasattr(mw.scanner, 'get_cached_games') else None
    print('Scan cache at', round((i+1)*0.5,1),'s:', 0 if not sc else len(sc))

print('Attempting to close app now')
try:
    mw.destroy()
    print('Destroyed main window')
except Exception as e:
    print('Destroy failed', e)
