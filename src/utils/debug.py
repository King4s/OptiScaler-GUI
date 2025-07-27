#!/usr/bin/env python3
"""
Debug system for OptiScaler-GUI
Controls when debug messages are shown and provides log storage
"""

import sys
import time
import threading
from collections import deque

# Global debug state
_debug_enabled = False
_log_storage = deque(maxlen=1000)  # Store last 1000 log entries
_log_lock = threading.Lock()

class LogHandler:
    """Handles log storage and retrieval"""
    
    def __init__(self):
        self.last_retrieved = 0
    
    def add_log(self, message):
        """Add a log entry with timestamp"""
        timestamp = time.strftime("%H:%M:%S")
        log_entry = f"[{timestamp}] {message}"
        
        with _log_lock:
            _log_storage.append((time.time(), log_entry))
    
    def get_logs(self):
        """Get all stored logs"""
        with _log_lock:
            return [entry[1] for entry in _log_storage]
    
    def get_new_logs(self, since=0):
        """Get logs since a specific timestamp"""
        with _log_lock:
            return [entry[1] for entry in _log_storage if entry[0] > since]

# Global log handler instance
_log_handler = LogHandler()

def set_debug_enabled(enabled):
    """Enable or disable debug output globally"""
    global _debug_enabled
    _debug_enabled = enabled

def is_debug_enabled():
    """Check if debug is enabled"""
    global _debug_enabled
    return _debug_enabled

def get_debug_log_handler():
    """Get the debug log handler"""
    return _log_handler

def debug_print(*args, **kwargs):
    """Print debug message only if debug is enabled"""
    global _debug_enabled
    if _debug_enabled:
        message = " ".join(str(arg) for arg in args)
        _log_handler.add_log(f"DEBUG: {message}")
        print("DEBUG:", *args, **kwargs)

def debug_log(message):
    """Log debug message with DEBUG prefix"""
    global _debug_enabled
    _log_handler.add_log(f"DEBUG: {message}")
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
