import sys
sys.path.insert(0, 'src')
from utils.debug import set_debug_enabled
set_debug_enabled(True)

from scanner.game_scanner import GameScanner
from gui.main_window import MainWindow

print('Creating GameScanner and running an explicit scan to populate cache')
scanner = GameScanner()
res = scanner.scan_games(force_refresh=True)
print('Scan completed, found', len(res), 'games')

print('Creating MainWindow and injecting cached scanner into it')
mw = MainWindow()
# Inject our scanner with cache to the main window's scanner so it uses cached data
mw.scanner = scanner

print('Calling show_game_list without forcing to use cached results')
mw.show_game_list(force_refresh=False)

print('Sleeping briefly to allow logs...')
import time
time.sleep(1)

cached = mw.scanner.get_cached_games()
print('Cached games visible to main window after initial display:', 0 if not cached else len(cached))

# Force a rescan and check
print('Forcing rescan via rescan_games()')
mw.rescan_games()
# Wait a bit for the thread to run
time.sleep(2)

cached = mw.scanner.get_cached_games()
print('Cached games after forced rescan:', 0 if not cached else len(cached))

print('Cleanup: destroy main window')
try:
    mw.destroy()
    print('Destroyed MainWindow')
except Exception as e:
    print('Destroy failed:', e)
