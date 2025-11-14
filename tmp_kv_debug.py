from pathlib import Path
import sys
sys.path.insert(0, r'D:\VSC Projekt mappe\OptiScaler-GUI\src')
from src.scanner import library_discovery as ld
p=Path('C:/tmp_kv_debug')
import shutil
if p.exists(): shutil.rmtree(p)
p.mkdir(parents=True, exist_ok=True)
steam_root=p/'SteamKV'
other_lib=p/'OtherSteamKV'
(steam_root/'steamapps').mkdir(parents=True, exist_ok=True)
(other_lib/'steamapps'/'common').mkdir(parents=True, exist_ok=True)
content='"libraryfolders"\n{\n\t"0"\n\t{\n\t\t"path"  "%s"\n\t}\n}\n' % str(other_lib).replace('\\','\\\\')
libs_file=steam_root/'steamapps'/'libraryfolders.vdf'
libs_file.write_text(content)
print('content:', content)
ld.vdf = None
print('_parse result ->', ld._parse_steam_libraryfolders_vdf(str(libs_file)))
print('_kv parse result ->', ld._parse_steam_libraryfolders_vdf_kv(content))
print('include result ->', ld._include_steam_vdf_libraries([{'Launcher':'Steam','Path':str(steam_root)}]))
print('get_game_libraries ->', ld.get_game_libraries_from_fallback())
