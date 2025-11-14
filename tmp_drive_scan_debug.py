from pathlib import Path
import sys
sys.path.insert(0, str(Path(__file__).parent / 'src'))
from src.scanner import library_discovery as ld
import psutil

p = Path('C:/tmp_drive_scan')
if p.exists():
    import shutil
    shutil.rmtree(p)
p.mkdir(parents=True, exist_ok=True)

fake_install = p / 'Program Files (x86)' / 'Steam' / 'steamapps' / 'common'
fake_install.mkdir(parents=True, exist_ok=True)

class Part:
    def __init__(self, mountpoint):
        self.mountpoint = mountpoint

ld._get_installed_programs_from_registry = lambda: [{'DisplayName': 'Steam', 'InstallLocation': str(p / 'Program Files' / 'Steam')}]

ld.psutil.disk_partitions = lambda all=False: [Part(str(p))]

res = ld.get_game_libraries_from_fallback()
print('results:')
for r in res:
    print(r)

print('done')
