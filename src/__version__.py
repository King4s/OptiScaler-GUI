"""
Version information for OptiScaler-GUI
"""

__version__ = "0.4.0"
__version_info__ = (0, 4, 0)

# Version history:
# 0.4.0 - Multi-platform detection overhaul & image fix
#       - Fixed structural UI bug: game info/buttons were unreachable dead code
#       - Fixed frozen exe asset paths (sys._MEIPASS for read-only assets)
#       - Replaced dead Steam GetAppList API with SteamSpy background download
#       - Xbox Game Pass detection via packaging files (.xsp/.smd/.xct/.xvi)
#       - Epic game DisplayName reading from .egstore manifests
#       - CamelCase folder name splitting for Epic/Xbox name lookup
#       - Name suffix stripping + difflib fuzzy matching for Steam AppID lookup
#       - Steam Store API fallback for CDN 404s (demos & content-hashed URLs)
#       - Thumbnail retry pass after SteamSpy background download completes
# 0.3.6 - Library discovery improvements, UI perf & release prep
#       - PowerShell + fallback discovery, caching/TTL
#       - Improved UI performance (background image loading)
#       - Fixed redraw race conditions and improved rescan behavior
# 0.3.0 - PyInstaller support and portable version
#       - Fixed translation system for executable builds
#       - Added PyInstaller build system with build.py
#       - Created portable version with bundled 7z.exe
#       - Enhanced OptiScaler detection (Unreal Engine support)
#       - Improved archive extraction with fallback methods
#       - Added documentation and troubleshooting guides
# 0.2.0 - (Previous development version)
# 0.1.0 - Initial development version
