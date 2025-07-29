# PyInstaller Build Guide for OptiScaler-GUI

## 📋 Oversigt
Denne guide beskriver hvordan man bygger OptiScaler-GUI til en standalone executable med PyInstaller, samt løsninger på almindelige problemer der opstår.

## 🛠️ Build Kommandoer

### Standard Build
```bash
python -m PyInstaller build_executable.spec --clean
```

### Test Build (hurtigere til udvikling)
```bash
python -m PyInstaller build_executable.spec
```

### Clean Build (hvis der er problemer)
```bash
python -m PyInstaller build_executable.spec --clean --noconfirm
```

### Build uden bekræftelse (automatisk ja til overskrivning)
```bash
python -m PyInstaller build_executable.spec --noconfirm
```

> **💡 Tip**: Brug altid `--noconfirm` for at undgå manuel bekræftelse når dist directory overskrives!

## 📁 Resultater
Efter successful build findes filerne i:
- **Single file**: `dist\OptiScaler-GUI.exe` (~123 MB)
- **Portable**: `dist\OptiScaler-GUI-Portable\` (folder med alle filer)

## ⚠️ Almindelige Problemer og Løsninger

### Problem 1: "robust_wrapper.py not found" Fejl
**Symptom**: 
```
Failed to scan games: [WinError 3] Den angivne sti blev ikke fundet: 'C:\Users\...\utils\robust_wrapper.py'
```

**Løsning**: 
Implementeret PyInstaller-specifik wrapper i `src/gui/widgets/game_list_frame.py`:

```python
# PyInstaller-aware import system
RUNNING_IN_PYINSTALLER = getattr(sys, 'frozen', False)

if RUNNING_IN_PYINSTALLER:
    # Use SimpleGameWrapper for PyInstaller
    class SimpleGameWrapper:
        def safe_operation(self, operation, path):
            # Simple file-based check for OptiScaler
        def _fallback_is_installed(self, path):
            # Basic DLL detection
    robust_wrapper = SimpleGameWrapper()
else:
    # Normal development import
    from utils.robust_wrapper import robust_wrapper
```

### Problem 2: Translation Keys vises i stedet for tekst
**Symptom**: 
Knapper viser `[ui.install_optiscaler]` og `[ui.open_folder]` i stedet for korrekt tekst

**Løsning**: 
PyInstaller-kompatibel path handling i `src/utils/translation_manager.py`:

```python
# PyInstaller-compatible path handling
if getattr(sys, 'frozen', False):
    # PyInstaller environment
    bundle_dir = Path(sys._MEIPASS)
    self.translations_dir = bundle_dir / 'src' / 'translations'
else:
    # Development environment
    self.translations_dir = Path(__file__).parent.parent / "translations"
```

### Problem 3: Path Import Problemer
**Symptom**: Import fejl for utils moduler

**Løsning**: 
PyInstaller-kompatibel path setup i alle utils filer:

```python
def setup_paths():
    """Setup paths for both development and PyInstaller environments"""
    if getattr(sys, 'frozen', False):
        # PyInstaller environment
        bundle_dir = Path(sys._MEIPASS)
        src_dir = bundle_dir / 'src'
    else:
        # Development environment
        current_dir = Path(__file__).parent
        src_dir = current_dir.parent
    
    if str(src_dir) not in sys.path:
        sys.path.insert(0, str(src_dir))
    
    return src_dir

setup_paths()
```

### Problem 3: Icon Conversion Fejl
**Symptom**: 
```
ValueError: Something went wrong converting icon image 'xbox_logo.png' to '.ico'
```

**Løsning**: 
I `build_executable.spec` sæt:
```python
icon=None
```

## 🔧 Build Konfiguration

### Vigtige Hidden Imports
I `build_executable.spec` skal disse hidden imports være inkluderet:

```python
hiddenimports=[
    # GUI dependencies
    'customtkinter', 'tkinter', '_tkinter', 'PIL', 'PIL._tkinter_finder', 'CTkMessagebox',
    
    # Archive extraction
    'py7zr', 'py7zr.helpers', 'py7zr.exceptions',
    
    # Utils modules
    'utils.robust_wrapper', 'utils.translation_manager', 'utils.debug', 
    'utils.config', 'utils.cache_manager', 'utils.progress', 
    'utils.update_manager', 'utils.compatibility_checker', 'utils.archive_extractor',
    'utils.i18n', 'utils.logging_utils',
    
    # GUI modules
    'gui.widgets.game_list_frame', 'gui.widgets.dynamic_setup_frame',
    
    # Scanner modules
    'scanner.game_scanner',
    
    # OptiScaler modules
    'optiscaler.manager',
    
    # System utilities
    'psutil', 'requests', 'winreg', 'json',
],
```

### Data Files
Vigtige data filer der skal inkluderes:

```python
datas=[
    # Include translations
    (str(src_dir / 'translations'), 'src/translations'),
    # Include assets
    ('assets', 'assets'),
    # Include cache directory structure (empty)
    ('cache', 'cache'),
    # Include requirements file
    ('requirements.txt', '.'),
    # Include README
    ('README.md', '.'),
],
```

## 🎯 PyInstaller vs Development Funktionalitet

### Development Mode (fuld funktionalitet)
- ✅ Game scanning
- ✅ OptiScaler installation/deinstallation
- ✅ Archive extraction (7z/ZIP)
- ✅ Robust error handling
- ✅ Debug logging

### PyInstaller Mode (read-only)
- ✅ Game scanning
- ✅ Installation status tjek (via DLL detection)
- ✅ Archive extraction (7z/ZIP)
- ❌ OptiScaler installation (sikkerhedsbesked)
- ❌ OptiScaler deinstallation (sikkerhedsbesked)

## 🐛 Debugging Tips

### 1. Test PyInstaller Detection
```python
import sys
if getattr(sys, 'frozen', False):
    print("Running in PyInstaller")
    print(f"Bundle dir: {sys._MEIPASS}")
else:
    print("Running in development")
```

### 2. Tjek Hidden Imports
Hvis moduler mangler, tilføj dem til `hiddenimports` i spec filen.

### 3. Tjek Data Files
Hvis assets mangler, tilføj dem til `datas` i spec filen.

### 4. Clean Build
Ved mærkelige fejl, kør altid:
```bash
python -m PyInstaller build_executable.spec --clean --noconfirm
```

### 5. Undgå Bekræftelse Prompts
Brug `--noconfirm` for at undgå "Continue? (y/N)" spørgsmål når dist directory overskrives:
```bash
python -m PyInstaller build_executable.spec --noconfirm
```
Dette er især nyttigt i scripts eller automatiserede builds.

## 📦 Distribution

### Single File (OptiScaler-GUI.exe)
- **Fordele**: En enkelt fil, nem at distribuere
- **Ulemper**: Langsommere opstart (udpakker til temp hver gang)
- **Størrelse**: ~123 MB

### Portable (OptiScaler-GUI-Portable/)
- **Fordele**: Hurtigere opstart, alle filer synlige
- **Ulemper**: Mange filer at distribuere
- **Størrelse**: Samme indhold, bare udpakket

## 🔄 Future Updates

### Når du tilføjer nye moduler:
1. Tilføj til `hiddenimports` i `build_executable.spec`
2. Sikr PyInstaller-kompatibel path setup
3. Test både development og PyInstaller mode

### Når du ændrer GUI funktionalitet:
1. Overvej om det skal virke i PyInstaller mode
2. Implementer fallback hvis nødvendigt
3. Test thoroughly på begge modes

## ✅ Success Checklist

Før release, tjek at:
- [ ] Build kompletter uden fejl
- [ ] Executable starter uden crashes
- [ ] Game scanning virker
- [ ] Installation status vises korrekt
- [ ] Archive extraction virker (hvis relevant)
- [ ] Ingen "file not found" fejl i logs

---

## 🔗 Relaterede Filer

- `build_executable.spec` - PyInstaller konfiguration
- `build.py` - Build script med error handling
- `src/gui/widgets/game_list_frame.py` - PyInstaller compatibility wrapper
- `src/utils/` - Alle moduler med PyInstaller path setup

**Sidste opdatering**: 29. juli 2025
**Testet med**: PyInstaller 6.14.2, Python 3.12.7
