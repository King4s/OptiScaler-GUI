#!/usr/bin/env python3
"""
Progress Overlay component for OptiScaler-GUI
Shows centered progress overlay during operations
"""

import customtkinter as ctk
import threading
import time
import tkinter as tk
from tkinter import Canvas

class Custom3DProgressBar(ctk.CTkFrame):
    """Custom 3D progress bar that works everywhere"""
    def __init__(self, master, width=250, height=12, **kwargs):
        super().__init__(master, width=width, height=height, 
                         fg_color="transparent", **kwargs)
        
        self.width = width
        self.height = height
        self.progress_value = 0.0
        
        # Create canvas for custom drawing with proper background
        self.canvas = Canvas(self, width=width, height=height, 
                           highlightthickness=0, bg="#2b2b2b")
        self.canvas.pack(fill="both", expand=True)
        
        self._draw_progress()
        
    def set(self, value):
        """Set progress value (0.0 to 1.0)"""
        self.progress_value = max(0.0, min(1.0, value))
        self._draw_progress()
        
    def _draw_progress(self):
        """Draw the 3D progress bar"""
        self.canvas.delete("all")
        
        # Draw background track with 3D effect
        # Outer shadow
        self.canvas.create_rectangle(1, 1, self.width-1, self.height-1, 
                                   fill="#1a1a1a", outline="", width=0)
        
        # Main background
        self.canvas.create_rectangle(2, 2, self.width-2, self.height-2, 
                                   fill="#404040", outline="", width=0)
        
        # Inner highlight
        self.canvas.create_rectangle(3, 3, self.width-3, 4, 
                                   fill="#555555", outline="", width=0)
        
        # Progress fill with 3D effect
        if self.progress_value > 0:
            progress_width = max(4, (self.width - 4) * self.progress_value)
            
            # Main progress fill
            self.canvas.create_rectangle(2, 2, progress_width + 2, self.height-2,
                                       fill="#0078d4", outline="", width=0)
            
            # 3D highlight on top
            self.canvas.create_rectangle(2, 2, progress_width + 2, 4,
                                       fill="#409cff", outline="", width=0)
            
            # 3D shadow on bottom
            self.canvas.create_rectangle(2, self.height-3, progress_width + 2, self.height-2,
                                       fill="#005a9e", outline="", width=0)
            
            # Bright highlight stripe
            if progress_width > 8:
                highlight_width = min(progress_width * 0.4, 20)
                self.canvas.create_rectangle(2, 3, highlight_width + 2, self.height//2,
                                           fill="#60b6ff", outline="", width=0)
        
    def update_idletasks(self):
        """Compatibility method"""
        self.canvas.update_idletasks()

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
        
        # Progress bar with true 3D effect that works everywhere
        self.progress_bar = Custom3DProgressBar(self, width=250, height=12)
        self.progress_bar.grid(row=1, column=0, padx=20, pady=8)
        self.progress_bar.set(0)
        
        # Status label
        self.status_label = ctk.CTkLabel(self, text="Please wait...", 
                                        font=("Arial", 12))
        self.status_label.grid(row=2, column=0, padx=20, pady=(5, 15))
        
        # Hide by default
        self.place_forget()
        self._animation_running = False
        self._animation_value = 0.0
        self._animation_direction = 1
        self._animation_id = None
        
    def show_overlay(self, title="Processing", message="Please wait..."):
        """Show the centered progress overlay"""
        self.title_label.configure(text=title)
        self.status_label.configure(text=message)
        self.progress_bar.set(0)
        
        # Use relative positioning for better centering
        self.place(relx=0.5, rely=0.5, anchor="center")
        
        # Bring to front to ensure visibility
        self.lift()
        
    def update_status(self, message):
        """Update the status message"""
        self.status_label.configure(text=message)
        
    def start_indeterminate(self, title="Processing", message="Please wait..."):
        """Start indeterminate progress animation with 3D effect"""
        # Stop any existing animation first
        self._animation_running = False
        if self._animation_id is not None:
            try:
                self.after_cancel(self._animation_id)
            except:
                pass
            self._animation_id = None
        
        # Show overlay first
        self.show_overlay(title, message)
        
        # Start smooth 3D animation
        self._animation_running = True
        self._animation_value = 0.0
        self._animation_direction = 1
        
        # Force immediate first animation step
        self._animate_step()
        
    def _animate_step(self):
        """Smooth 3D animation step"""
        if not self._animation_running:
            self._animation_id = None
            return
            
        try:
            # Update animation value with smooth acceleration/deceleration
            self._animation_value += self._animation_direction * 0.03
            
            if self._animation_value >= 1.0:
                self._animation_value = 1.0
                self._animation_direction = -1
            elif self._animation_value <= 0.0:
                self._animation_value = 0.0
                self._animation_direction = 1
            
            # Apply smooth easing for 3D effect
            eased_value = self._ease_in_out_cubic(self._animation_value)
            
            # Update progress bar with smooth 3D animation
            self.progress_bar.set(eased_value)
            
            # Schedule next animation step
            if self._animation_running:
                self._animation_id = self.after(50, self._animate_step)  # 20 FPS
            else:
                self._animation_id = None
                
        except Exception as e:
            self._animation_running = False
            self._animation_id = None
            return
            
    def _ease_in_out_cubic(self, t):
        """Cubic easing function for smooth 3D effect"""
        if t < 0.5:
            return 4 * t * t * t
        else:
            return 1 - pow(-2 * t + 2, 3) / 2
            
    def hide_overlay(self):
        """Hide the progress overlay"""
        self._animation_running = False
        
        # Cancel any pending animation callback
        if self._animation_id is not None:
            try:
                self.after_cancel(self._animation_id)
            except:
                pass
            self._animation_id = None
            
        self.progress_bar.set(0)  # Reset progress bar
        self.place_forget()

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
