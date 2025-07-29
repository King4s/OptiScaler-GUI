#!/usr/bin/env python3
"""
Check System Requirements - Standalone Script
Run this to check if your system is ready for OptiScaler GUI
"""

import sys
import os
from pathlib import Path

# Add src to path for imports
current_dir = Path(__file__).parent
src_dir = current_dir / "src"
if str(src_dir) not in sys.path:
    sys.path.insert(0, str(src_dir))

try:
    from utils.system_requirements import print_requirements_report, show_requirements_dialog
    
    def main():
        """Main function to check requirements"""
        print("🔧 OptiScaler GUI System Requirements Checker")
        print("=" * 60)
        
        # Try GUI version first, fallback to console
        try:
            import customtkinter
            print("Using GUI mode for requirements check...")
            show_requirements_dialog()
        except ImportError:
            print("GUI not available, using console mode...")
            print_requirements_report()

    if __name__ == "__main__":
        main()

except ImportError as e:
    print(f"❌ Error importing requirements checker: {e}")
    print("Please ensure you're running from the OptiScaler-GUI directory")
    sys.exit(1)
