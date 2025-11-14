from pathlib import Path
import os
import sys
sys.path.insert(0, str(Path(__file__).parent / 'src'))
from src.scanner import library_discovery as ld

p=Path('C:/temp_debug_vdf')
if p.exists():
    import shutil
    shutil.rmtree(p)
p.mkdir(parents=True)
steam_root = p / 'Steam'
other_lib = p / 'OtherSteam'
(steam_root / 'steamapps').mkdir(parents=True)
(other_lib / 'steamapps' / 'common').mkdir(parents=True)
# Write vdf
content = '"libraryfolders"\n{\n    "0"\n    {\n        "path"  "%s"\n    }\n}\n' % str(other_lib).replace('\\', '\\\\')
(lib:= steam_root / 'steamapps' / 'libraryfolders.vdf').write_text(content)
print('wrote', lib)
print('vdf parse ->', ld._parse_steam_libraryfolders_vdf(str(lib)))

# Now test include logic
a = []
a = ld._include_steam_vdf_libraries(a)
print('include result w/o steam entry (should include known libs?) ->', a)
# Now call with a steam registry path appended
import importlib
importlib.reload(ld)
print('Call include with steam install from registry')
res = ld._include_steam_vdf_libraries([{'Launcher':'Steam', 'Path': str(steam_root), 'Source': 'Registry'}])
print('res ->', res)

# Now test JSON VDF format
json_content = '{"libraryfolders": {"0": {"path": "%s"}}}' % str(other_lib).replace('\\', '\\\\')
json_file = steam_root / 'steamapps' / 'libraryfolders.json'
json_vdf_file = steam_root / 'steamapps' / 'libraryfolders.vdf'
json_vdf_file.write_text(json_content)
print('\nJSON test:')
print('wrote', json_vdf_file)
print('json content:', json_vdf_file.read_text())
import json as _json
try:
    parsed_json = _json.loads(json_vdf_file.read_text())
    print('json loaded ->', parsed_json)
except Exception as e:
    print('json loads error', e)
try:
    import vdf as _vdf
    print('vdf.loads on JSON content ->', _vdf.loads(json_vdf_file.read_text()))
except Exception as e:
    print('vdf.loads error on JSON content', e)
print('json parse ->', ld._parse_steam_libraryfolders_vdf(str(json_vdf_file)))
res2 = ld._include_steam_vdf_libraries([{'Launcher':'Steam', 'Path': str(steam_root), 'Source': 'Registry'}])
print('include result JSON ->', res2)
