import customtkinter as ctk
import threading
import time
from utils.debug import get_debug_log_handler, debug_log
from utils.translation_manager import t

class LogFrame(ctk.CTkScrollableFrame):
    """Debug log viewer frame"""
    
    def __init__(self, master, **kwargs):
        super().__init__(master, **kwargs)
        self.grid_columnconfigure(0, weight=1)
        self.grid_rowconfigure(1, weight=1)
        
        self._setup_ui()
        self._start_log_monitoring()
    
    def _setup_ui(self):
        """Setup the log viewer UI"""
        # Title
        title_label = ctk.CTkLabel(self, text=t("ui.debug_log"), 
                                 font=("Arial", 20, "bold"))
        title_label.grid(row=0, column=0, padx=20, pady=(20, 10), sticky="w")
        
        # Controls frame
        controls_frame = ctk.CTkFrame(self)
        controls_frame.grid(row=1, column=0, padx=20, pady=5, sticky="ew")
        controls_frame.grid_columnconfigure(2, weight=1)
        
        # Clear button
        clear_btn = ctk.CTkButton(controls_frame, text=t("ui.clear_log"), 
                                command=self._clear_log, width=100)
        clear_btn.grid(row=0, column=0, padx=5, pady=5)
        
        # Auto-scroll toggle
        self.auto_scroll_var = ctk.BooleanVar(value=True)
        auto_scroll_cb = ctk.CTkCheckBox(controls_frame, text=t("ui.auto_scroll"),
                                       variable=self.auto_scroll_var)
        auto_scroll_cb.grid(row=0, column=1, padx=10, pady=5)
        
        # Log text area
        self.log_text = ctk.CTkTextbox(self, wrap="word", state="disabled")
        self.log_text.grid(row=2, column=0, padx=20, pady=(5, 20), sticky="nsew")
        
        # Initial log content
        self._load_existing_logs()
    
    def _load_existing_logs(self):
        """Load existing log entries"""
        try:
            log_handler = get_debug_log_handler()
            if log_handler and hasattr(log_handler, 'get_logs'):
                logs = log_handler.get_logs()
                if logs:
                    self.log_text.configure(state="normal")
                    self.log_text.insert("end", "\n".join(logs) + "\n")
                    self.log_text.configure(state="disabled")
                    if self.auto_scroll_var.get():
                        self.log_text.see("end")
        except Exception as e:
            debug_log(f"Error loading existing logs: {e}")
    
    def _start_log_monitoring(self):
        """Start monitoring for new log entries"""
        self.monitoring = True
        self.monitor_thread = threading.Thread(target=self._monitor_logs, daemon=True)
        self.monitor_thread.start()
    
    def _monitor_logs(self):
        """Monitor for new log entries in background thread"""
        last_check = time.time()
        
        while self.monitoring:
            try:
                log_handler = get_debug_log_handler()
                if log_handler and hasattr(log_handler, 'get_new_logs'):
                    new_logs = log_handler.get_new_logs(since=last_check)
                    if new_logs:
                        # Update UI in main thread
                        self.after(0, self._append_logs, new_logs)
                
                last_check = time.time()
                time.sleep(0.5)  # Check every 500ms
                
            except Exception as e:
                debug_log(f"Error in log monitoring: {e}")
                time.sleep(1)  # Wait longer on error
    
    def _append_logs(self, logs):
        """Append new logs to the text area (called from main thread)"""
        try:
            self.log_text.configure(state="normal")
            for log_entry in logs:
                self.log_text.insert("end", f"{log_entry}\n")
            self.log_text.configure(state="disabled")
            
            if self.auto_scroll_var.get():
                self.log_text.see("end")
                
        except Exception as e:
            print(f"Error appending logs: {e}")
    
    def _clear_log(self):
        """Clear the log display"""
        self.log_text.configure(state="normal")
        self.log_text.delete("1.0", "end")
        self.log_text.configure(state="disabled")
        debug_log("Log display cleared")
    
    def destroy(self):
        """Clean up when frame is destroyed"""
        self.monitoring = False
        if hasattr(self, 'monitor_thread'):
            self.monitor_thread.join(timeout=1.0)
        super().destroy()
