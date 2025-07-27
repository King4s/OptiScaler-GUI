#!/usr/bin/env python3
"""
Repository Cleanup Script for OptiScaler-GUI
Removes temporary, debug, and test files before GitHub upload
"""

import os
import shutil
from pathlib import Path

def cleanup_repository():
    """Clean up repository by removing unnecessary files"""
    
    # Files to remove from root directory
    files_to_remove = [
        # Test files
        "test_github.py",
        "test_gpu_detection.py", 
        "test_new_gpu_detection.py",
        
        # Debug files
        "debug_analyzer.py",
        "debug_gpu.py",
        "DEBUG_OPTIMIZATION_COMPLETE.md",
        "debug_optimization_report.md",
        "DEBUG_SYSTEM.md",
        
        # Fix files
        "fix_gpu.bat",
        "fix_gpu_detection.py",
        "fix_gpu_python.py",
        
        # Backup files
        "manager_backup.py",
        "manager_temp.py",
        
        # Development files
        "DEVELOPMENT_SUMMARY.md",
        "DEPLOYMENT.md",
        "FUNCTION_REFERENCE.md",
        "PROJECT_STATUS.md",
        "SOLUTIONS_SUMMARY.md",
        
        # Utility files
        "cleanup_project.py",
        "create_no_image.py",
        "commit_message.txt",
        
        # Build files (keep build.py, remove build.bat)
        "build.bat",
    ]
    
    # Directories to remove
    dirs_to_remove = [
        "cache",  # Contains temporary downloads and logs
        ".vscode",  # IDE settings
        ".venv",  # Virtual environment
    ]
    
    root_path = Path(".")
    removed_files = []
    removed_dirs = []
    
    print("🧹 Starting repository cleanup...")
    print("=" * 50)
    
    # Remove files
    for file_name in files_to_remove:
        file_path = root_path / file_name
        if file_path.exists():
            try:
                file_path.unlink()
                removed_files.append(file_name)
                print(f"✅ Removed file: {file_name}")
            except Exception as e:
                print(f"❌ Failed to remove {file_name}: {e}")
        else:
            print(f"⚠️  File not found: {file_name}")
    
    # Remove directories
    for dir_name in dirs_to_remove:
        dir_path = root_path / dir_name
        if dir_path.exists() and dir_path.is_dir():
            try:
                shutil.rmtree(dir_path)
                removed_dirs.append(dir_name)
                print(f"✅ Removed directory: {dir_name}")
            except Exception as e:
                print(f"❌ Failed to remove {dir_name}: {e}")
        else:
            print(f"⚠️  Directory not found: {dir_name}")
    
    # Clean up __pycache__ directories
    pycache_removed = 0
    for pycache in root_path.rglob("__pycache__"):
        try:
            shutil.rmtree(pycache)
            pycache_removed += 1
            print(f"✅ Removed __pycache__: {pycache}")
        except Exception as e:
            print(f"❌ Failed to remove __pycache__ {pycache}: {e}")
    
    # Clean up .pyc files
    pyc_removed = 0
    for pyc_file in root_path.rglob("*.pyc"):
        try:
            pyc_file.unlink()
            pyc_removed += 1
        except Exception as e:
            print(f"❌ Failed to remove .pyc file {pyc_file}: {e}")
    
    print("\n" + "=" * 50)
    print("🎉 Cleanup completed!")
    print(f"📁 Files removed: {len(removed_files)}")
    print(f"📂 Directories removed: {len(removed_dirs)}")
    print(f"🧹 __pycache__ directories removed: {pycache_removed}")
    print(f"🗑️ .pyc files removed: {pyc_removed}")
    
    if removed_files:
        print("\n📋 Files removed:")
        for file in removed_files:
            print(f"   - {file}")
    
    if removed_dirs:
        print("\n📋 Directories removed:")
        for dir in removed_dirs:
            print(f"   - {dir}")
    
    print("\n✨ Repository is now clean and ready for GitHub!")

def check_remaining_files():
    """Show remaining files structure"""
    print("\n📁 Remaining repository structure:")
    print("=" * 50)
    
    root_path = Path(".")
    
    # Show root files
    for item in sorted(root_path.iterdir()):
        if item.name.startswith('.git'):
            continue
        if item.is_file():
            print(f"📄 {item.name}")
        elif item.is_dir():
            print(f"📂 {item.name}/")
            # Show first level of subdirectories
            try:
                for subitem in sorted(item.iterdir()):
                    if subitem.is_file():
                        print(f"   📄 {subitem.name}")
                    elif subitem.is_dir():
                        print(f"   📂 {subitem.name}/")
            except PermissionError:
                print(f"   ⚠️ Permission denied")

if __name__ == "__main__":
    print("🚀 OptiScaler-GUI Repository Cleanup")
    print("This script will remove temporary, debug, and test files")
    print("=" * 60)
    
    # Auto-run cleanup without user input
    print("✅ Auto-running cleanup...")
    cleanup_repository()
    check_remaining_files()
