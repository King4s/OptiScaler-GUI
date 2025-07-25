#!/usr/bin/env python3
"""
Progress Bar component for OptiScaler-GUI
Shows download and installation progress
"""

import customtkinter as ctk
import threading
import time

class ProgressFrame(ctk.CTkFrame):
    def __init__(self, master, **kwargs):
        super().__init__(master, **kwargs)
        
        # Configure grid
        self.grid_columnconfigure(0, weight=1)
        
        # Progress bar
        self.progress_bar = ctk.CTkProgressBar(self)
        self.progress_bar.grid(row=0, column=0, padx=10, pady=5, sticky="ew")
        self.progress_bar.set(0)
        
        # Status label
        self.status_label = ctk.CTkLabel(self, text="Ready")
        self.status_label.grid(row=1, column=0, padx=10, pady=5)
        
        # Hide by default
        self.grid_remove()
        
    def show_progress(self, message="Processing..."):
        """Show the progress bar with a message"""
        self.status_label.configure(text=message)
        self.progress_bar.set(0)
        self.grid(row=0, column=0, sticky="ew")
        
    def update_progress(self, value, message=None):
        """Update progress bar value (0.0 to 1.0)"""
        self.progress_bar.set(value)
        if message:
            self.status_label.configure(text=message)
        self.update()
        
    def hide_progress(self):
        """Hide the progress bar"""
        self.grid_remove()
        
    def start_indeterminate(self, message="Processing..."):
        """Start indeterminate progress animation"""
        self.show_progress(message)
        self._animate_indeterminate()
        
    def _animate_indeterminate(self):
        """Animate indeterminate progress"""
        def animate():
            value = 0
            direction = 1
            while self.winfo_viewable():
                value += direction * 0.02
                if value >= 1.0:
                    value = 1.0
                    direction = -1
                elif value <= 0.0:
                    value = 0.0
                    direction = 1
                
                try:
                    self.progress_bar.set(value)
                    self.update()
                    time.sleep(0.05)
                except:
                    break
        
        # Run animation in thread to avoid blocking UI
        thread = threading.Thread(target=animate, daemon=True)
        thread.start()

class ProgressManager:
    """Manages progress bars across the application"""
    
    def __init__(self):
        self.progress_frames = {}
        
    def register_progress_frame(self, name, progress_frame):
        """Register a progress frame with a name"""
        self.progress_frames[name] = progress_frame
        
    def show_progress(self, name, message="Processing..."):
        """Show progress for a named frame"""
        if name in self.progress_frames:
            self.progress_frames[name].show_progress(message)
            
    def update_progress(self, name, value, message=None):
        """Update progress for a named frame"""
        if name in self.progress_frames:
            self.progress_frames[name].update_progress(value, message)
            
    def hide_progress(self, name):
        """Hide progress for a named frame"""
        if name in self.progress_frames:
            self.progress_frames[name].hide_progress()
            
    def start_indeterminate(self, name, message="Processing..."):
        """Start indeterminate progress for a named frame"""
        if name in self.progress_frames:
            self.progress_frames[name].start_indeterminate(message)

# Global progress manager
progress_manager = ProgressManager()
