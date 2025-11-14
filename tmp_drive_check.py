from pathlib import Path
import sys
sys.path.insert(0, str(Path('.') / 'src'))
from src.scanner import library_discovery as ld

# Replace with path used by the failing test
tmp_path = Path(r'C:/Users/marci/AppData/Local/Temp/pytest-of-marci/pytest-52/test_fallback_discovery_regist0')
(p:=tmp_path / 'Program Files (x86)' / 'Steam' / 'steamapps' / 'common').mkdir(parents=True, exist_ok=True)
print('candidate', p)

class Part:
    def __init__(self, mountpoint):
        self.mountpoint = mountpoint

part=Part(str(tmp_path))
known_folders = [r"\\Program Files\\Steam\\steamapps\\common", r"\\Program Files (x86)\\Steam\\steamapps\\common", r"\\Epic Games", r"\\GOG Games", r"\\Program Files\\GOG Galaxy\\Games"]
for f in known_folders:
    candidate = Path(part.mountpoint) / f.strip('\\')
    print('checking', candidate, 'exists?', candidate.exists())
