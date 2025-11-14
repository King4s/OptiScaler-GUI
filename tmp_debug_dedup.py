from pathlib import Path
import sys
sys.path.insert(0, str(Path(__file__).parent / 'src'))
from src.scanner import library_discovery as ld
import psutil

p=Path('C:/tmp_debug_dedup')
if p.exists():
    import shutil
    shutil.rmtree(p)
p.mkdir(parents=True, exist_ok=True)
lib1 = p/'Steam'/'steamapps'/'common'
lib2 = p/'OtherSteam'/'steamapps'/'common'
lib1.mkdir(parents=True, exist_ok=True)
lib2.mkdir(parents=True, exist_ok=True)
# write vdf under lib1.parent
vdf_file = lib1.parent/'libraryfolders.vdf'
vdf_file.write_text('{"libraryfolders": {"0": {"path": "%s"}}}' % str(lib2.parent).replace('\\','\\\\'))
# monkeypatch functions by setting expected ones
ld._get_installed_programs_from_registry = lambda: [{'DisplayName':'Steam', 'InstallLocation': str(lib1.parent)}]
ld._find_steam_install_from_registry = lambda: str(lib1.parent.parent)
# patch disk partitions to empty
ld.psutil.disk_partitions = lambda all=False: []
res = ld.get_game_libraries_from_fallback()
print('results:')
for r in res:
    print(r)

print('\n_parsed vdf: ', ld._parse_steam_libraryfolders_vdf(str(vdf_file)))
print('\n_included: ', ld._include_steam_vdf_libraries([{'Launcher':'Steam', 'Path': str(lib1.parent.parent)}]))
