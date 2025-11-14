import sys
sys.path.insert(0, 'src')
from utils.debug import set_debug_enabled
set_debug_enabled(True)

from gui.main_window import MainWindow

# Create main window instance
mw = MainWindow()
print('MainWindow created')

# Show settings
mw.show_settings()
print('Show settings called')

# Click View Log button programmatically: find the current frame and call its _open_log if available
if hasattr(mw.current_frame, '_open_log'):
    print('Opening log from settings frame')
    mw.current_frame._open_log()
else:
    print('Settings frame has no _open_log attribute')

# Access the log frame to verify back callback is set
from gui.widgets.log_frame import LogFrame
if isinstance(mw.current_frame, LogFrame):
    print('Currently in LogFrame')
    if getattr(mw.current_frame, 'on_back', None):
        print('LogFrame has on_back callback; invoking to return')
        # Call the back function
        try:
            mw.current_frame.on_back()
            print('Returned from log view via on_back')
        except Exception as e:
            print('Failed to call on_back:', e)
else:
    # Otherwise try to find a LogFrame in content
    print('Not in LogFrame; searching for LogFrame among content frame children')
    gen = [c for c in mw.content_frame.winfo_children() if isinstance(c, LogFrame)]
    if gen:
        lframe = gen[0]
        print('Found log frame instance')
        if getattr(lframe, 'on_back', None):
            lframe.on_back()
            print('Invoked back callback on visible LogFrame')

# Go back to games
print('Show game list (not forcing refresh)')
# Inspect cached games in scanner before calling show_game_list
cached = getattr(mw.scanner, 'get_cached_games', lambda: None)()
print('Cached games before show_game_list:', 0 if not cached else len(cached))
mw.show_game_list(force_refresh=False)
print('Called show_game_list without force_refresh')
# Inspect whether cached games were used (print scanner cached size)
cached = getattr(mw.scanner, 'get_cached_games', lambda: None)()
print('Cached games after show_game_list call:', 0 if not cached else len(cached))

# Force a rescan and confirm cache cleared and new results exist
print('Forcing rescan...')
mw.rescan_games()

# After forcing, check cache is refreshed
import time
# Give some time for threaded scan to finish (not ideal but provides some buffer)
time.sleep(1)
cached = getattr(mw.scanner, 'get_cached_games', lambda: None)()
print('Cached games after forcing rescan:', 0 if not cached else len(cached))

print('Script completed')

# Close the window to avoid lingering window processes
try:
    mw.destroy()
    print('Destroyed main window')
except Exception as e:
    print('Destroy failed:', e)
