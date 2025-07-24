#!/usr/bin/env python3
"""
OptiScaler-GUI Systemcheck
Viser dit grafikkort og spilkompatibilitet
"""

import os
import sys
import subprocess
import json
import platform
import psutil
from pathlib import Path

class SimpleChecker:
    def __init__(self):
        self.issues = []
        self.info = []
        
    def add_issue(self, message):
        """Tilføj problem der skal løses"""
        self.issues.append(message)
        
    def add_info(self, message):
        """Tilføj god information"""
        self.info.append(message)
    
    def check_your_gpu(self):
        """Find dit grafikkort og tjek om det virker med OptiScaler"""
        try:
            # Hent GPU info fra Windows
            cmd = ['powershell', '-Command', '''
                Get-WmiObject -Class Win32_VideoController | 
                Where-Object {$_.Name -notlike "*Basic*" -and $_.Name -notlike "*Generic*"} |
                Select-Object Name, AdapterRAM |
                ConvertTo-Json
            ''']
            
            result = subprocess.run(cmd, capture_output=True, text=True, timeout=10)
            
            if result.returncode == 0 and result.stdout.strip():
                gpu_data = json.loads(result.stdout)
                if not isinstance(gpu_data, list):
                    gpu_data = [gpu_data]
                
                # Find det bedste GPU
                best_gpu = None
                for gpu in gpu_data:
                    name = gpu.get('Name', '').strip()
                    if not name:
                        continue
                        
                    name_upper = name.upper()
                    vram_bytes = gpu.get('AdapterRAM', 0)
                    vram_gb = 0
                    if vram_bytes and vram_bytes > 0:
                        vram_gb = vram_bytes / (1024**3)
                    
                    # Tjek om det er et gaming GPU
                    if any(brand in name_upper for brand in ['RTX', 'GTX', 'RX', 'VEGA', 'ARC']):
                        best_gpu = {'name': name, 'vram_gb': vram_gb}
                        break
                    elif any(brand in name_upper for brand in ['NVIDIA', 'AMD', 'RADEON']):
                        if not best_gpu:  # Brug som backup
                            best_gpu = {'name': name, 'vram_gb': vram_gb}
                
                if best_gpu:
                    self.add_info(f"Dit Grafikkort: {best_gpu['name']}")
                    if best_gpu['vram_gb'] > 0:
                        self.add_info(f"Video Hukommelse: {best_gpu['vram_gb']:.1f} GB")
                    
                    # Simpel kompatibilitetscheck
                    name = best_gpu['name'].upper()
                    if any(nvidia in name for nvidia in ['RTX', 'GTX', 'NVIDIA']):
                        self.add_info("✓ NVIDIA GPU - Virker perfekt med OptiScaler!")
                        if 'RTX' in name:
                            self.add_info("  → DLSS virker perfekt")
                        else:
                            self.add_info("  → Brug FSR i stedet for DLSS")
                        self.add_info("  → FSR og XeSS virker også")
                    elif any(amd in name for amd in ['RX', 'VEGA', 'AMD', 'RADEON']):
                        self.add_info("✓ AMD GPU - Virker rigtig godt med OptiScaler!")
                        self.add_info("  → FSR virker perfekt (anbefalet)")
                        self.add_info("  → XeSS virker også")
                        self.add_info("  → DLSS virker ikke (det er normalt)")
                    elif 'ARC' in name:
                        self.add_info("✓ Intel Arc GPU - Virker godt med OptiScaler!")
                        self.add_info("  → XeSS virker perfekt (anbefalet)")
                        self.add_info("  → FSR virker også")
                    else:
                        self.add_info("⚠ Ældre grafikkort fundet")
                        self.add_info("  → OptiScaler virker måske, men ydeevnen kan være begrænset")
                else:
                    self.add_issue("❌ Intet gaming grafikkort fundet")
                    
        except Exception:
            self.add_issue("❌ Kan ikke finde dit grafikkort")
    
    def check_your_games(self):
        """Tjek om dine spil kan findes"""
        # Tjek Steam
        steam_paths = [
            "C:/Program Files (x86)/Steam",
            "C:/Program Files/Steam"
        ]
        
        steam_found = False
        for path in steam_paths:
            if os.path.exists(path):
                self.add_info(f"✓ Steam fundet: {path}")
                
                # Tæl spil hurtigt
                steamapps = os.path.join(path, "steamapps")
                if os.path.exists(steamapps):
                    game_files = [f for f in os.listdir(steamapps) if f.startswith("appmanifest_")]
                    if game_files:
                        self.add_info(f"  → Fandt {len(game_files)} Steam spil")
                steam_found = True
                break
        
        if not steam_found:
            self.add_info("⚠ Steam ikke fundet - kun Steam spil understøttes lige nu")
    
    def check_basic_requirements(self):
        """Tjek om programmet kan køre"""
        # Tjek Python pakker
        required = ['customtkinter', 'requests', 'PIL']
        missing = []
        
        for pkg in required:
            try:
                __import__(pkg)
            except ImportError:
                missing.append(pkg)
        
        if missing:
            self.add_issue(f"❌ Manglende programmer: {', '.join(missing)}")
        else:
            self.add_info("✓ Alle nødvendige programmer er installeret")
        
        # Tjek internet
        try:
            import requests
            requests.get('https://httpbin.org/status/200', timeout=3)
            self.add_info("✓ Internet forbindelse virker")
        except:
            self.add_info("⚠ Internet virker måske ikke (behøves til spil billeder)")
    
    def run_check(self):
        """Kør det komplette tjek"""
        print("=" * 50)
        print("OptiScaler-GUI System Check")
        print("=" * 50)
        
        self.check_basic_requirements()
        self.check_your_gpu()
        self.check_your_games()
        
        # Vis resultater
        if self.issues:
            print(f"\n❌ PROBLEMER FUNDET ({len(self.issues)}):")
            for issue in self.issues:
                print(f"   {issue}")
        
        if self.info:
            print(f"\n✓ SYSTEM INFO:")
            for info in self.info:
                print(f"   {info}")
        
        print("\n" + "=" * 50)
        
        if self.issues:
            print("⚠️  Du skal løse problemerne ovenfor først")
            return 1
        else:
            print("🎮 Alt ser godt ud - OptiScaler-GUI burde virke!")
            return 0

if __name__ == "__main__":
    checker = SimpleChecker()
    exit_code = checker.run_check()
    sys.exit(exit_code)
