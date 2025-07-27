#!/usr/bin/env python3
"""
Progress Overlay component for OptiScaler-GUI
Shows centered progress overlay during operations
"""

import customtkinter as ctk
import threading
import time

class ProgressOverlay(ctk.CTkFrame):
    def __init__(self, master, **kwargs):
        super().__init__(master, 
                         width=300,
                         height=120,
                         fg_color=("gray90", "gray10"),
                         border_width=2,
                         border_color=("gray70", "gray30"),
                         corner_radius=10,
                         **kwargs)
        
        # Configure grid
        self.grid_columnconfigure(0, weight=1)
        
        # Title label
        self.title_label = ctk.CTkLabel(self, text="Processing", 
                                       font=("Arial", 16, "bold"))
        self.title_label.grid(row=0, column=0, padx=20, pady=(15, 5))
        
        # Progress bar - smaller and centered
        self.progress_bar = ctk.CTkProgressBar(self, width=250, height=10)
        self.progress_bar.grid(row=1, column=0, padx=20, pady=5)
        self.progress_bar.set(0)
        
        # Status label
        self.status_label = ctk.CTkLabel(self, text="Please wait...", 
                                        font=("Arial", 12))
        self.status_label.grid(row=2, column=0, padx=20, pady=(5, 15))
        
        # Hide by default
        self.place_forget()
        self._animation_running = False
        
    def show_overlay(self, title="Processing", message="Please wait..."):
        """Show the centered progress overlay"""
        self.title_label.configure(text=title)
        self.status_label.configure(text=message)
        self.progress_bar.set(0)
        
        # Center the overlay on the parent
        self.update_idletasks()
        parent_width = self.master.winfo_width()
        parent_height = self.master.winfo_height()
        overlay_width = 300
        overlay_height = 120
        
        x = (parent_width - overlay_width) // 2
        y = (parent_height - overlay_height) // 2
        
        self.place(x=x, y=y)
        
    def update_status(self, message):
        """Update the status message"""
        self.status_label.configure(text=message)
        self.update()
        
    def hide_overlay(self):
        """Hide the progress overlay"""
        self._animation_running = False
        self.place_forget()
        
    def start_indeterminate(self, title="Processing", message="Please wait..."):
        """Start indeterminate progress animation"""
        self.show_overlay(title, message)
        self._animation_running = True
        self._animate_indeterminate()
        
    def _animate_indeterminate(self):
        """Animate indeterminate progress"""
        def animate():
            value = 0
            direction = 1
            while self._animation_running and self.winfo_viewable():
                value += direction * 0.03
                if value >= 1.0:
                    value = 1.0
                    direction = -1
                elif value <= 0.0:
                    value = 0.0
                    direction = 1
                
                try:
                    if self._animation_running:
                        self.progress_bar.set(value)
                        self.update()
                    time.sleep(0.05)
                except:
                    break
        
        # Run animation in thread to avoid blocking UI
        thread = threading.Thread(target=animate, daemon=True)
        thread.start()

class ProgressManager:
    """Manages progress overlays across the application"""
    
    def __init__(self):
        self.progress_overlays = {}
        
    def register_progress_overlay(self, name, progress_overlay):
        """Register a progress overlay with a name"""
        self.progress_overlays[name] = progress_overlay
        
    def show_progress(self, name, title="Processing", message="Please wait..."):
        """Show progress for a named overlay"""
        if name in self.progress_overlays:
            self.progress_overlays[name].show_overlay(title, message)
            
    def update_status(self, name, message):
        """Update status message for a named overlay"""
        if name in self.progress_overlays:
            self.progress_overlays[name].update_status(message)
            
    def hide_progress(self, name):
        """Hide progress for a named overlay"""
        if name in self.progress_overlays:
            self.progress_overlays[name].hide_overlay()
            
    def start_indeterminate(self, name, title="Processing", message="Please wait..."):
        """Start indeterminate progress for a named overlay"""
        if name in self.progress_overlays:
            self.progress_overlays[name].start_indeterminate(title, message)

# Global progress manager
progress_manager = ProgressManager()
