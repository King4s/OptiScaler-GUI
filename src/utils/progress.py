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
        
        # Use relative positioning for better centering
        self.place(relx=0.5, rely=0.5, anchor="center")
        
        # If relative positioning doesn't work, fallback to absolute
        self.after(10, self._verify_positioning)
        
    def _verify_positioning(self):
        """Verify and correct positioning if needed"""
        try:
            # Check if the overlay is visible and properly positioned
            if self.winfo_viewable():
                x = self.winfo_x()
                y = self.winfo_y()
                
                # If positioned at (0,0) or negative, recalculate
                if x <= 0 or y <= 0:
                    self.master.update_idletasks()
                    parent_width = self.master.winfo_width()
                    parent_height = self.master.winfo_height()
                    
                    if parent_width > 1 and parent_height > 1:
                        x = (parent_width - 300) // 2
                        y = (parent_height - 120) // 2
                        x = max(x, 10)
                        y = max(y, 10)
                        self.place(x=x, y=y)
        except:
            # Ignore any errors during position verification
            pass
        
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
        # Ensure parent has proper dimensions before showing
        self.master.update_idletasks()
        
        # Small delay to ensure parent is fully rendered
        self.after(10, lambda: self._show_and_animate(title, message))
        
    def _show_and_animate(self, title, message):
        """Show overlay and start animation after delay"""
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
