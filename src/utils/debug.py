#!/usr/bin/env python3
"""
Debug system for OptiScaler-GUI
Controls when debug messages are shown
"""

import sys

# Global debug state
_debug_enabled = False

def set_debug_enabled(enabled):
    """Enable or disable debug output globally"""
    global _debug_enabled
    _debug_enabled = enabled

def is_debug_enabled():
    """Check if debug is enabled"""
    global _debug_enabled
    return _debug_enabled

def debug_print(*args, **kwargs):
    """Print debug message only if debug is enabled"""
    global _debug_enabled
    if _debug_enabled:
        print("DEBUG:", *args, **kwargs)

def debug_log(message):
    """Log debug message with DEBUG prefix"""
    global _debug_enabled
    if _debug_enabled:
        print(f"DEBUG: {message}")

# Monkey patch print to intercept DEBUG messages
_original_print = print

def conditional_print(*args, **kwargs):
    """Print function that filters DEBUG messages when debug is disabled"""
    global _debug_enabled
    
    # Check if first argument contains "DEBUG:"
    if args and len(args) > 0:
        first_arg = str(args[0])
        if first_arg.startswith("DEBUG:") and not _debug_enabled:
            return  # Skip debug messages when debug is disabled
    
    # Call original print for non-debug messages or when debug is enabled
    _original_print(*args, **kwargs)

# Replace built-in print with our conditional version
print = conditional_print
