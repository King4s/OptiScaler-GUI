import customtkinter as ctk
from pathlib import Path
from utils.translation_manager import t
from utils.debug import debug_log

class LibraryRootsFrame(ctk.CTkScrollableFrame):
    def __init__(self, master, libraries=None, on_rescan=None, **kwargs):
        super().__init__(master, **kwargs)
        self.grid_columnconfigure(0, weight=1)
        self.libraries = libraries or []
        self.on_rescan = on_rescan
        self._create_ui()

    def _create_ui(self):
        title = ctk.CTkLabel(self, text='Detected Library Roots', font=('Arial', 16, 'bold'))
        title.grid(row=0, column=0, padx=10, pady=(10, 5), sticky='w')
        # Quick actions
        actions_frame = ctk.CTkFrame(self, fg_color='transparent')
        actions_frame.grid(row=0, column=1, padx=10, pady=(10, 5), sticky='e')
        # Method label that shows the predominant discovery method
        method_text = self._detect_method_label()
        self.method_lbl = ctk.CTkLabel(actions_frame, text=f"Method: {method_text}")
        self.method_lbl.grid(row=0, column=0, padx=(0, 10))
        # Rescan button
        self._rescan_btn = ctk.CTkButton(actions_frame, text='Rescan', command=self._on_rescan)
        self._rescan_btn.grid(row=0, column=1)

        # Summary label showing counts by source
        from src.scanner.library_discovery import compute_library_summary
        summary = compute_library_summary(self.libraries)
        summary_items = []
        for k, v in summary.items():
            if k == 'total':
                continue
            summary_items.append(f"{k}: {v}")
        summary_text = f"Total: {summary.get('total', 0)}" + (" — " + ", ".join(summary_items) if summary_items else "")
        self._summary_lbl = ctk.CTkLabel(self, text=summary_text, font=('Arial', 10))
        self._summary_lbl.grid(row=1, column=0, padx=10, pady=(5, 10), sticky='w')

        # Create a list box or a frame with labels showing launcher/drive/path
        self._entries = []
        # Status label (last scanned)
        from datetime import datetime
        self._status_lbl = ctk.CTkLabel(self, text=f"Last scanned: {datetime.now().isoformat()}")
        self._status_lbl.grid(row=1, column=1, padx=10, pady=(10, 5), sticky='e')
        # Shift the entries start row down by 1 to leave status line
        self._entries_start_row = 2
        for i, lib in enumerate(self.libraries):
            text = f"{lib.get('Launcher', '')} - {lib.get('Drive', '')} - {lib.get('Path', '')} ({lib.get('Source', '')})"
            lbl = ctk.CTkLabel(self, text=text, font=('Arial', 10))
            lbl.grid(row=self._entries_start_row + i, column=0, padx=10, pady=4, sticky='w')
            self._entries.append(lbl)

    def _detect_method_label(self):
        # Determine the highest-probability method from the list
        srcs = {lib.get('Source') for lib in self.libraries}
        if not srcs:
            return 'Unknown'
        if 'Powershell' in srcs:
            return 'PowerShell'
        if 'WinRT' in srcs:
            return 'WinRT'
        if 'RegistryFallback' in srcs:
            return 'Registry'
        if 'DriveScan' in srcs:
            return 'DriveScan'
        return list(srcs)[0]

    def _on_rescan(self):
        if not self.on_rescan:
            return
        import threading, datetime
        # Run the rescan in a background thread so we don't block the UI
        def _background_rescan():
            try:
                self.after(0, lambda: self.method_lbl.configure(text='Method: Scanning...'))
                new_libs = self.on_rescan()
                def _apply_results():
                    if not new_libs:
                        self.method_lbl.configure(text='Method: (no results)')
                    else:
                        self.libraries = new_libs
                        self.method_lbl.configure(text=f"Method: {self._detect_method_label()}")
                    # Update the list
                    for e in getattr(self, '_entries', []):
                        e.destroy()
                    self._create_entries()
                    # Update status
                    ts = datetime.datetime.now().isoformat()
                    self._status_lbl.configure(text=f"Last scanned: {ts}")
                    # Update summary
                    from src.scanner.library_discovery import compute_library_summary
                    summary = compute_library_summary(self.libraries)
                    summary_items = []
                    for k, v in summary.items():
                        if k == 'total':
                            continue
                        summary_items.append(f"{k}: {v}")
                    summary_text = f"Total: {summary.get('total', 0)}" + (" — " + ", ".join(summary_items) if summary_items else "")
                    self._summary_lbl.configure(text=summary_text)
                self.after(0, _apply_results)
            except Exception as e:
                debug_log(f"Library root rescan failed: {e}")
                self.after(0, lambda: self.method_lbl.configure(text='Method: (scan failed)'))
            finally:
                self.after(0, lambda: self._rescan_btn.configure(state='normal'))

        # Disable the rescan button while scanning
        self._rescan_btn.configure(state='disabled')
        threading.Thread(target=_background_rescan, daemon=True).start()
        except Exception as e:
            debug_log(f"Library root rescan failed: {e}")

    def _create_entries(self):
        self._entries = []
        for i, lib in enumerate(self.libraries):
            text = f"{lib.get('Launcher', '')} - {lib.get('Drive', '')} - {lib.get('Path', '')} ({lib.get('Source', '')})"
            lbl = ctk.CTkLabel(self, text=text, font=('Arial', 10))
            lbl.grid(row=self._entries_start_row + i, column=0, padx=10, pady=4, sticky='w')
            self._entries.append(lbl)
