import sys, pathlib
sys.path.insert(0, str(pathlib.Path().resolve() / 'src'))
from scanner.game_scanner import GameScanner

gs = GameScanner()
reg_games = gs._scan_installed_programs()
appx_games = gs._scan_appx_packages()
print('Registry apps > games found:', len(reg_games))
print([ (g.name, g.path) for g in reg_games[:10] ])
print('Appx packages > games found:', len(appx_games))
print([ (g.name, g.path) for g in appx_games[:10] ])
