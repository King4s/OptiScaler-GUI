# Debug System Documentation

## How the Debug System Works

The OptiScaler-GUI debug system provides user-controlled debug output through a smart print function override system.

### Core Components

#### 1. Debug Control Module (`utils/debug.py`)

**Purpose**: Centralized debug control with UI integration

**Key Functions**:
- `set_debug_enabled(enabled)`: Enable/disable debug output globally
- `debug_log(message)`: Output debug message (only if debug enabled)
- `conditional_print()`: Monkey-patched print function that filters DEBUG messages

#### 2. Monkey-Patched Print Function

**How it works**:
```python
# Original print function is saved
original_print = print

def conditional_print(*args, **kwargs):
    """Print function that filters DEBUG messages when debug is disabled"""
    # Check if this is a debug message
    if args and isinstance(args[0], str) and args[0].startswith("DEBUG:"):
        # Only print DEBUG messages if debug is enabled
        if debug_enabled:
            original_print(*args, **kwargs)
    else:
        # Always print non-debug messages
        original_print(*args, **kwargs)

# Override built-in print
print = conditional_print
```

**Result**: All `print("DEBUG: message")` calls are automatically controlled by the debug toggle.

#### 3. UI Integration

**Header Toggle**:
- Checkbox in main window header: "Debug"
- Toggles `debug_enabled` global variable
- Immediately affects all debug output

**Settings Tab**:
- Switch in global settings for persistent debug preference
- Syncs with header checkbox

### Usage Patterns

#### For Developers

**Option 1 - Direct debug_log() (Recommended)**:
```python
from utils.debug import debug_log

debug_log("This message is controlled by debug toggle")
debug_log(f"Variable value: {some_variable}")
```

**Option 2 - Legacy print() with DEBUG prefix**:
```python
print("DEBUG: This message is automatically controlled")
print(f"DEBUG: Processing {filename}")
```

**Option 3 - Regular print() (Always visible)**:
```python
print("ERROR: This message always shows")
print("Game installed successfully!")  # User feedback
```

#### For Users

**Enable Debug Output**:
1. Check "Debug" checkbox in header, OR
2. Enable "Debug Mode" in Settings tab
3. Debug messages will appear in console/terminal

**Disable Debug Output** (Default):
1. Uncheck "Debug" checkbox in header, OR  
2. Disable "Debug Mode" in Settings tab
3. Only important messages show (errors, success messages)

### Technical Implementation

#### Initialization Sequence

1. `main.py` imports `utils.debug` module
2. Debug module executes and monkey-patches `print()`
3. `set_debug_enabled(False)` sets initial state (disabled)
4. UI elements connect to debug control functions

#### Module Integration

**All modules automatically benefit**:
```python
# In any module
print("DEBUG: This is controlled")  # ✅ Automatically filtered
print("INFO: Always visible")       # ✅ Always shows
debug_log("Controlled message")     # ✅ Explicitly controlled
```

**No code changes needed** for existing debug statements starting with "DEBUG:"

#### Performance Impact

- **Debug Disabled**: Near-zero overhead (simple string prefix check)
- **Debug Enabled**: Normal print performance
- **Memory**: Minimal (one global boolean variable)

### Debugging the Debug System

#### Check Debug State

```python
from utils.debug import debug_enabled
print(f"Debug currently: {'enabled' if debug_enabled else 'disabled'}")
```

#### Force Debug Message

```python
from utils.debug import debug_log
debug_log("This message respects debug setting")

# OR bypass the system entirely
from utils.debug import original_print
original_print("This message always shows regardless of debug setting")
```

#### Verify Monkey Patch

```python
import builtins
print(f"Print function: {builtins.print}")
# Should show: <function conditional_print at 0x...>
```

### Troubleshooting

#### Debug Messages Not Showing
1. Check debug checkbox in header is enabled
2. Verify `debug_enabled` variable: `from utils.debug import debug_enabled; print(debug_enabled)`
3. Ensure message starts with "DEBUG:" or use `debug_log()`

#### Debug Messages Always Showing
1. Check if message starts with "DEBUG:" (it should to be filtered)
2. Verify monkey patch is active
3. Check if `set_debug_enabled(True)` was called somewhere

#### Import Errors
- Ensure `utils.debug` is imported before other modules that use debug
- Check import order in `main.py`

### Best Practices

#### For New Code
- Use `debug_log()` for debug messages
- Use regular `print()` for user-facing messages
- Use `print("ERROR: ...")` for error messages (always visible)

#### For Existing Code
- Leave existing `print("DEBUG: ...")` calls - they work automatically
- Convert to `debug_log()` when convenient
- Don't mix debug and user messages in same print call

#### Message Format
```python
# Good
debug_log("Processing game list")
debug_log(f"Found {count} games")

# Also good (legacy)
print("DEBUG: Processing game list")
print(f"DEBUG: Found {count} games")

# Don't do this (won't be filtered)
print(f"Found {count} games (DEBUG)")
```

### Architecture Benefits

1. **Backward Compatible**: Existing debug code works without changes
2. **User Friendly**: Clean UI without debug spam by default
3. **Developer Friendly**: Easy to enable debug output when needed
4. **Performance**: No overhead when debug disabled
5. **Centralized**: Single point of control for all debug output
6. **UI Integrated**: Debug state visible and controllable from UI

This system provides the perfect balance between development convenience and user experience.
