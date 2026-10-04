> Historical legacy snapshot; not current build/run guidance. See ../../README.md and ../../CLAUDE.md.

# 🚨 KRITISKE PROBLEMER - HUSK ALTID AT TJEKKE DETTE!

## ⚠️ KRITISK PROBLEM: Python DLL Load Error

### Problem
PyInstaller executable fejler med:
```
Failed to load Python DLL 'C:\OptiScaler-GUI\build\OptiScaler-GUI\_internal\python312.dll'
LoadLibrary: Det angivne modul blev ikke fundet.
```

### Root Cause
PyInstaller har ikke korrekt bundlet Python DLL eller der er sti problemer i den byggede executable.

### LØSNING 
1. **Brug dist mappen i stedet for build mappen:**
```bash
# I stedet for:
c:\OptiScaler-GUI\build\OptiScaler-GUI\OptiScaler-GUI.exe

# Brug:
c:\OptiScaler-GUI\dist\OptiScaler-GUI\OptiScaler-GUI.exe
```

2. **Eller byg igen med clean flag:**
```bash
python -m PyInstaller OptiScaler-GUI.spec --clean --noconfirm
```

### Test Procedure
- Brug ALTID dist mappen til test af executable
- Build mappen er intermediate files - ikke den endelige executable

## ⚠️ KRITISK PROBLEM: Icon/Ikon Problemer i PyInstaller

### Problem
PyInstaller build fejler med fejl om ikon filer der ikke kan konverteres til .ico format:
```
ValueError: Something went wrong converting icon image 'C:\OptiScaler-GUI\assets/icons/xbox_logo.png' to '.ico' with Pillow
```

### Root Cause
PNG filer kan ikke direkte bruges som Windows executable ikoner - de skal være .ico format.

### LØSNING (IMPLEMENTERET!)
Fjern ikon reference fra spec fil:
```python
# I stedet for:
icon='assets/icons/xbox_logo.png' if os.path.exists('assets/icons/xbox_logo.png') else None,

# Brug:
icon=None,
```

### Resultat
✅ PyInstaller build succeeds without icon errors
✅ Executable builds correctly (bare uden brugerdefineret ikon)

## ⚠️ KRITISK PROBLEM: Translation ID Keys Vises i UI

### Problem
Når PyInstaller bygger executable, vises translation keys som `[ui.install_optiscaler]` i stedet for rigtig tekst.

### Root Cause 
`src/translations` mappen bliver ikke bundlet med executable fordi den ikke er inkluderet i PyInstaller spec fil.

### LØSNING (HUSK DETTE!)
1. **Tjek altid at `src/translations` er inkluderet i spec fil:**
```python
datas = [
    ('assets', 'assets'),
    ('cache', 'cache'),
    ('src/translations', 'src/translations'),  # ← DENNE LINJE!
]
```

## ⚠️ KRITISK PROBLEM: SimpleGameWrapper mangler optiscaler_manager

### Problem
PyInstaller executable fejler med: "'SimpleGameWrapper' object has no attribute 'optiscaler_manager'"

### Root Cause
`SimpleGameWrapper` klassen i `game_list_frame.py` havde ikke `optiscaler_manager` attributten som koden forsøger at tilgå.

### LØSNING (IMPLEMENTERET!)
Tilføjede `__init__` metode til `SimpleGameWrapper` med:
```python
def __init__(self):
    try:
        from optiscaler.manager import OptiScalerManager
        self.optiscaler_manager = OptiScalerManager()
    except Exception as e:
        self.optiscaler_manager = None
```

## ⚠️ KRITISK PROBLEM: UI Refresh Efter Installation

### Problem
Efter OptiScaler installation vises stadig "Install" knappen i stedet for "Uninstall" knappen, selvom installationen var succesfuld.

### Root Cause
`_refresh_display()` metoden opdaterede ikke `optiscaler_installed` status på game objekterne før UI'en blev genskabt.

### LØSNING (IMPLEMENTERET!)
Enhanced `_refresh_display()` method i `game_list_frame.py`:
```python
def _refresh_display(self):
    """Refresh the game list display to update button states"""
    # Clear update cache to force fresh check
    self._update_check_cache = None
    self._cache_timestamp = None
    
    # Update OptiScaler status for all games to get current state
    for game in self.games:
        try:
            game.optiscaler_installed = self.game_scanner._detect_optiscaler(game.path)
            debug_log(f"Updated OptiScaler status for {game.name}: {game.optiscaler_installed}")
        except Exception as e:
            debug_log(f"Failed to update OptiScaler status for {game.name}: {e}")
            game.optiscaler_installed = False
    
    # Clear current display and recreate
    for widget in self.winfo_children():
        widget.destroy()
    self._display_games()
```

### Resultat
✅ UI opdateres korrekt efter installation
✅ Install/Uninstall knapper viser den rigtige status
✅ Fungerer for både normale spil og Unreal Engine spil

## ⚠️ KRITISK PROBLEM: Unreal Engine OptiScaler Detection

### Problem
OptiScaler detection fejler for Unreal Engine spil fordi OptiScaler installeres i `Engine/Binaries/Win64` i stedet for hovedmappen.

### Root Cause
`_detect_optiscaler` metoden i game_scanner.py tjekkede kun hovedmappen og subdirectorier, men ikke Unreal Engine strukturen.

### LØSNING (IMPLEMENTERET!)
Updated `_detect_optiscaler` to check both:
- Main game directory (for normal games)
- `Engine/Binaries/Win64` directory (for Unreal Engine games)

### Unreal Engine Games Examples
- Soulmask, Black Myth Wukong, mange moderne spil
- OptiScaler installeres altid i `<game>/Engine/Binaries/Win64/`
- Detection skal tjekke begge steder

2. **Brug altid `--noconfirm` flag:**
```bash
python -m PyInstaller OptiScaler-GUI.spec --noconfirm
```

3. **Test efter hver build:**
- Åbn executable
- Tjek at knapper viser rigtig tekst
- Hvis translation keys vises: spec fil mangler translations!

### FEJL JEG HAR LAVET FLERE GANGE
❌ Glemmer at tilføje `src/translations` til spec fil (GJORT IGEN!)
❌ Ikke tjekker terminal output efter build
❌ Ikke tester executable efter ændringer  
❌ Ikke verificer at spec fil faktisk indeholder translations

**VIGTIGT: ALTDIG tjek at spec fil indeholder denne linje:**
```python
('src/translations', 'src/translations'),  # Translation files for i18n
```

### PROCEDURE FOR AT UNDGÅ DETTE
1. Læs ALTID denne fil før PyInstaller builds
2. Verificer spec fil indeholder translations
3. Brug --noconfirm flag 
4. Test executable umiddelbart efter build fra DIST mappen
5. Tjek terminal output med get_terminal_output

**VIGTIG: Brug ALTID dist mappen til test - IKKE build mappen!**
```bash
# Korrekt test path:
c:\OptiScaler-GUI\dist\OptiScaler-GUI\OptiScaler-GUI.exe

# Forkert test path (forårsager DLL fejl):
c:\OptiScaler-GUI\build\OptiScaler-GUI\OptiScaler-GUI.exe
```

## 📝 Andre Kritiske Punkter

### PyInstaller Build Process
- Brug ALTID `--noconfirm` flag for at undgå manuel bekræftelse
- Tjek ALTID terminal output efter hver operation
- Test ALTID executable efter build

### Translation System Issues
- Spec fil skal inkludere `('src/translations', 'src/translations')`
- `translation_manager.py` skal have korrekt `sys._MEIPASS` håndtering
- Test ved at åbne executable og se om knapper har rigtig tekst

### OptiScaler Detection
- Game scanner har nu integreret detection
- Brug `game.optiscaler_installed` i stedet for wrapper calls
- `_detect_optiscaler` metode tjekker alle relevante filer

---
**HUSK**: Læs denne fil HVER gang før du arbejder med PyInstaller eller translation issues!
