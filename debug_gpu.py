#!/usr/bin/env python3
"""
Enhanced Debug and GPU Detection Tool for OptiScaler-GUI
"""

import os
import sys
import subprocess
import platform
import json
import time
from pathlib import Path

# Add src to path
sys.path.insert(0, str(Path(__file__).parent / "src"))

def detect_gpu():
    """Comprehensive GPU detection for any GPU on Windows"""
    gpu_info = {
        'primary_gpu': {
            'type': 'unknown',
            'name': 'Unknown',
            'vendor': 'Unknown',
            'driver_version': 'Unknown',
            'vram_gb': 0,
            'category': 'unknown'
        },
        'all_gpus': [],
        'discrete_gpu': None,
        'integrated_gpu': None
    }
    
    try:
        # Method 1: Enhanced PowerShell WMI query
        cmd = ['powershell', '-Command', '''
            Get-WmiObject -Class Win32_VideoController | 
            Where-Object {$_.Name -notlike "*Basic*" -and $_.Name -notlike "*Generic*" -and $_.Name -ne $null} |
            Select-Object Name, AdapterRAM, DriverVersion, VideoProcessor, DeviceID |
            ConvertTo-Json
        ''']
        
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=15)
        
        if result.returncode == 0 and result.stdout.strip():
            try:
                gpu_data = json.loads(result.stdout)
                if not isinstance(gpu_data, list):
                    gpu_data = [gpu_data]
                
                for gpu in gpu_data:
                    name = gpu.get('Name', '').strip()
                    if not name:
                        continue
                        
                    name_upper = name.upper()
                    vram_bytes = gpu.get('AdapterRAM', 0)
                    vram_gb = 0
                    if vram_bytes and vram_bytes > 0:
                        vram_gb = vram_bytes / (1024**3)
                    
                    # Comprehensive vendor detection
                    gpu_vendor = 'unknown'
                    gpu_category = 'unknown'
                    gpu_series = ''
                    
                    # NVIDIA detection (all product lines)
                    if any(brand in name_upper for brand in ['NVIDIA', 'GEFORCE', 'RTX', 'GTX', 'QUADRO', 'TESLA', 'TITAN', 'NVS']):
                        gpu_vendor = 'nvidia'
                        if any(discrete in name_upper for discrete in ['RTX', 'GTX', 'TITAN', 'QUADRO', 'TESLA']):
                            gpu_category = 'discrete'
                            if 'RTX' in name_upper:
                                if any(x in name_upper for x in ['4090', '4080', '4070', '4060']):
                                    gpu_series = 'RTX 40 Series'
                                elif any(x in name_upper for x in ['3090', '3080', '3070', '3060']):
                                    gpu_series = 'RTX 30 Series'
                                elif any(x in name_upper for x in ['2080', '2070', '2060']):
                                    gpu_series = 'RTX 20 Series'
                            elif 'GTX' in name_upper:
                                gpu_series = 'GTX Series'
                        else:
                            gpu_category = 'integrated'
                    
                    # AMD detection (comprehensive, including all product lines)
                    elif any(brand in name_upper for brand in ['AMD', 'RADEON', 'RX', 'VEGA', 'NAVI', 'RDNA', 'FURY', 'FIREPRO', 'HD', 'R9', 'R7', 'R5']):
                        gpu_vendor = 'amd'
                        if any(discrete in name_upper for discrete in ['RX', 'VEGA', 'FURY', 'FIREPRO', 'NAVI', 'RDNA', 'R9', 'R7']):
                            gpu_category = 'discrete'
                            if 'RX' in name_upper:
                                if any(x in name_upper for x in ['7900', '7800', '7700', '7600']):
                                    gpu_series = 'RX 7000 Series (RDNA 3)'
                                elif any(x in name_upper for x in ['6950', '6900', '6800', '6700', '6600', '6500', '6400']):
                                    gpu_series = 'RX 6000 Series (RDNA 2)'
                                elif any(x in name_upper for x in ['5700', '5600', '5500']):
                                    gpu_series = 'RX 5000 Series (RDNA)'
                                elif any(x in name_upper for x in ['590', '580', '570', '560']):
                                    gpu_series = 'RX 500 Series'
                            elif 'VEGA' in name_upper:
                                gpu_series = 'Vega Series'
                        else:
                            gpu_category = 'integrated'
                            if any(apu in name_upper for apu in ['RYZEN', 'APU']):
                                gpu_series = 'Integrated Radeon'
                    
                    # Intel detection (all product lines including Arc)
                    elif any(brand in name_upper for brand in ['INTEL', 'UHD', 'IRIS', 'ARC', 'HD GRAPHICS']):
                        gpu_vendor = 'intel'
                        if 'ARC' in name_upper:
                            gpu_category = 'discrete'
                            if any(x in name_upper for x in ['A770', 'A750', 'A580', 'A380']):
                                gpu_series = 'Arc Alchemist Series'
                        else:
                            gpu_category = 'integrated'
                            if 'IRIS' in name_upper:
                                gpu_series = 'Iris Graphics'
                            elif 'UHD' in name_upper:
                                gpu_series = 'UHD Graphics'
                            else:
                                gpu_series = 'Intel HD Graphics'
                    
                    # Other vendors
                    elif any(brand in name_upper for brand in ['MATROX', 'VIA', 'SIS', 'S3']):
                        vendor_map = {'MATROX': 'matrox', 'VIA': 'via', 'SIS': 'sis', 'S3': 's3'}
                        for brand, vendor in vendor_map.items():
                            if brand in name_upper:
                                gpu_vendor = vendor
                                break
                        gpu_category = 'integrated'
                    
                    gpu_entry = {
                        'name': name,
                        'vendor': gpu_vendor,
                        'category': gpu_category,
                        'series': gpu_series,
                        'vram_gb': vram_gb,
                        'driver_version': gpu.get('DriverVersion', 'Unknown'),
                        'device_id': gpu.get('DeviceID', 'Unknown')
                    }
                    
                    gpu_info['all_gpus'].append(gpu_entry)
                    
                    # Set primary GPU (prioritize discrete over integrated)
                    if gpu_category == 'discrete':
                        if not gpu_info['discrete_gpu']:
                            gpu_info['discrete_gpu'] = gpu_entry
                        if not gpu_info['primary_gpu'] or gpu_info['primary_gpu']['category'] != 'discrete':
                            gpu_info['primary_gpu'] = gpu_entry
                    elif gpu_category == 'integrated':
                        if not gpu_info['integrated_gpu']:
                            gpu_info['integrated_gpu'] = gpu_entry
                        if not gpu_info['primary_gpu']:
                            gpu_info['primary_gpu'] = gpu_entry
                    else:  # unknown category
                        if not gpu_info['primary_gpu']:
                            gpu_info['primary_gpu'] = gpu_entry
                            
            except json.JSONDecodeError as e:
                print(f"JSON parsing failed: {e}")
    
    except Exception as e:
        print(f"PowerShell GPU detection failed: {e}")
    
    # Fallback method using WMIC if PowerShell fails
    if not gpu_info['all_gpus']:
        try:
            result = subprocess.run(['wmic', 'path', 'win32_VideoController', 'get', 'name,AdapterRAM,DriverVersion'], 
                                  capture_output=True, text=True, timeout=10)
            if result.returncode == 0:
                lines = [line.strip() for line in result.stdout.split('\n') if line.strip()]
                current_gpu = {}
                
                for line in lines[1:]:  # Skip header
                    if line and 'Name' not in line and 'AdapterRAM' not in line:
                        # Parse WMIC output (can be tricky due to formatting)
                        parts = line.split()
                        if len(parts) >= 1:
                            name = ' '.join(parts[:-2]) if len(parts) > 2 else ' '.join(parts)
                            name_upper = name.upper()
                            
                            # Basic vendor detection for fallback
                            gpu_vendor = 'unknown'
                            if any(brand in name_upper for brand in ['NVIDIA', 'GEFORCE', 'RTX', 'GTX']):
                                gpu_vendor = 'nvidia'
                            elif any(brand in name_upper for brand in ['AMD', 'RADEON', 'RX']):
                                gpu_vendor = 'amd'
                            elif any(brand in name_upper for brand in ['INTEL', 'UHD', 'IRIS', 'ARC']):
                                gpu_vendor = 'intel'
                            
                            gpu_entry = {
                                'name': name,
                                'vendor': gpu_vendor,
                                'category': 'unknown',
                                'series': '',
                                'vram_gb': 0,
                                'driver_version': 'Unknown',
                                'device_id': 'Unknown'
                            }
                            
                            gpu_info['all_gpus'].append(gpu_entry)
                            if not gpu_info['primary_gpu']:
                                gpu_info['primary_gpu'] = gpu_entry
                            
        except Exception as e:
            print(f"WMIC GPU detection also failed: {e}")
    
    # Final fallback - at least report that no GPU was detected
    if not gpu_info['primary_gpu']:
        gpu_info['primary_gpu'] = {
            'type': 'none',
            'name': 'No GPU Detected',
            'vendor': 'None',
            'driver_version': 'N/A',
            'vram_gb': 0,
            'category': 'none'
        }
    else:
        # Copy primary GPU info to maintain backward compatibility
        primary = gpu_info['primary_gpu']
        gpu_info['primary_gpu'].update({
            'type': primary['vendor']
        })
    
    return gpu_info

def check_optiscaler_compatibility(gpu_info):
    """Check OptiScaler compatibility based on detected GPU(s)"""
    recommendations = []
    primary_gpu = gpu_info['primary_gpu']
    
    if primary_gpu['vendor'] == 'nvidia':
        recommendations.append("✓ NVIDIA GPU detected - Excellent OptiScaler compatibility")
        recommendations.append(f"  GPU: {primary_gpu['name']}")
        if primary_gpu['series']:
            recommendations.append(f"  Series: {primary_gpu['series']}")
        if primary_gpu['vram_gb'] > 0:
            recommendations.append(f"  VRAM: {primary_gpu['vram_gb']:.1f} GB")
        recommendations.append("  - DLSS: Native support (recommended for NVIDIA)")
        recommendations.append("  - FSR: Full support")
        recommendations.append("  - XeSS: Full support")
        
        # DLSS specific recommendations
        if any(rtx in primary_gpu['name'].upper() for rtx in ['RTX', 'TITAN']):
            recommendations.append("  ✓ RTX GPU detected - DLSS 2.0/3.0 fully supported")
        elif 'GTX' in primary_gpu['name'].upper():
            recommendations.append("  ⚠ GTX GPU - DLSS not supported, use FSR instead")
            
    elif primary_gpu['vendor'] == 'amd':
        recommendations.append("✓ AMD GPU detected - Very good OptiScaler compatibility")
        recommendations.append(f"  GPU: {primary_gpu['name']}")
        if primary_gpu['series']:
            recommendations.append(f"  Series: {primary_gpu['series']}")
        if primary_gpu['vram_gb'] > 0:
            recommendations.append(f"  VRAM: {primary_gpu['vram_gb']:.1f} GB")
        recommendations.append("  - FSR: Native support (recommended for AMD)")
        recommendations.append("  - XeSS: Full support")
        recommendations.append("  ! DLSS: Not natively supported")
        
        # FSR specific recommendations
        if 'RDNA' in primary_gpu['series'].upper():
            recommendations.append("  ✓ RDNA architecture - FSR 2.0+ fully optimized")
        elif 'VEGA' in primary_gpu['series'].upper():
            recommendations.append("  ✓ Vega architecture - FSR 1.0/2.0 supported")
            
    elif primary_gpu['vendor'] == 'intel':
        if primary_gpu['category'] == 'discrete':
            recommendations.append("✓ Intel Arc GPU detected - Good OptiScaler compatibility")
            recommendations.append(f"  GPU: {primary_gpu['name']}")
            if primary_gpu['series']:
                recommendations.append(f"  Series: {primary_gpu['series']}")
            if primary_gpu['vram_gb'] > 0:
                recommendations.append(f"  VRAM: {primary_gpu['vram_gb']:.1f} GB")
            recommendations.append("  - XeSS: Native support (recommended for Intel Arc)")
            recommendations.append("  - FSR: Full support")
            recommendations.append("  ! DLSS: Not supported")
        else:
            recommendations.append("⚠ Intel integrated GPU detected - Limited OptiScaler compatibility")
            recommendations.append(f"  GPU: {primary_gpu['name']}")
            recommendations.append("  - XeSS: May work with reduced performance")
            recommendations.append("  - FSR: May work with reduced performance")
            recommendations.append("  ! DLSS: Not supported")
            recommendations.append("  ! Performance may be limited for demanding games")
            
    elif primary_gpu['vendor'] in ['matrox', 'via', 'sis', 's3']:
        recommendations.append(f"❌ {primary_gpu['vendor'].upper()} GPU detected - OptiScaler not supported")
        recommendations.append(f"  GPU: {primary_gpu['name']}")
        recommendations.append("  ! OptiScaler requires modern NVIDIA, AMD, or Intel GPUs")
        recommendations.append("  ! Consider upgrading to a supported GPU")
        
    else:
        recommendations.append("❌ Unknown/Unsupported GPU - OptiScaler compatibility uncertain")
        recommendations.append(f"  GPU: {primary_gpu['name']}")
        recommendations.append("  ! Manual configuration may be required")
        recommendations.append("  ! Performance may be limited or not work at all")
    
    # Multi-GPU recommendations
    if len(gpu_info['all_gpus']) > 1:
        recommendations.append(f"\nMulti-GPU Setup Detected ({len(gpu_info['all_gpus'])} GPUs):")
        for i, gpu in enumerate(gpu_info['all_gpus']):
            status = "(Primary)" if gpu == primary_gpu else "(Secondary)"
            recommendations.append(f"  GPU {i+1}: {gpu['name']} - {gpu['vendor'].upper()} {status}")
        
        if gpu_info['discrete_gpu'] and gpu_info['integrated_gpu']:
            recommendations.append("  ✓ Hybrid setup detected - OptiScaler will use discrete GPU")
            recommendations.append("  ℹ Ensure games are running on discrete GPU for best performance")
    
    return recommendations

def test_dependencies():
    """Test all dependencies"""
    try:
        from utils.config import config
        from utils.cache_manager import cache_manager
        from utils.performance import performance_monitor
        from scanner.game_scanner import GameScanner
        print("✓ All utility modules imported successfully")
        return True
    except Exception as e:
        print(f"✗ Error importing modules: {e}")
        return False

def run_performance_test():
    """Run a quick performance test"""
    print("\n=== Performance Test ===")
    
    try:
        from utils.performance import performance_monitor
        performance_monitor.start_monitoring()
        
        # Test file operations
        start_time = time.time()
        test_data = {"test": list(range(1000))}
        with open("test_perf.json", "w") as f:
            json.dump(test_data, f)
        
        with open("test_perf.json", "r") as f:
            loaded_data = json.load(f)
        
        os.remove("test_perf.json")
        file_ops_time = time.time() - start_time
        
        print(f"File I/O test: {file_ops_time:.3f} seconds")
        
        # Get current metrics
        time.sleep(1)  # Let monitor collect data
        metrics = performance_monitor.get_current_metrics()
        if metrics:
            print(f"Memory usage: {metrics.get('memory_mb', 0):.1f} MB")
            print(f"Thread count: {metrics.get('thread_count', 0)}")
        
        performance_monitor.stop_monitoring()
        
    except Exception as e:
        print(f"Performance test failed: {e}")

def main():
    print("=" * 60)
    print("OptiScaler-GUI Enhanced Debug Tool")
    print("=" * 60)
    
    # System info
    print(f"OS: {platform.system()} {platform.release()}")
    print(f"Python: {platform.python_version()}")
    print(f"Architecture: {platform.machine()}")
    print(f"Processor: {platform.processor()}")
    
    # GPU detection
    print("\n=== GPU Detection ===")
    gpu_info = detect_gpu()
    primary_gpu = gpu_info['primary_gpu']
    print(f"Primary GPU: {primary_gpu['name']}")
    print(f"GPU Vendor: {primary_gpu['vendor'].upper()}")
    print(f"GPU Category: {primary_gpu['category'].title()}")
    if primary_gpu['series']:
        print(f"GPU Series: {primary_gpu['series']}")
    if primary_gpu['vram_gb'] > 0:
        print(f"VRAM: {primary_gpu['vram_gb']:.1f} GB")
    print(f"Driver Version: {primary_gpu['driver_version']}")
    
    # Show all GPUs if multiple detected
    if len(gpu_info['all_gpus']) > 1:
        print(f"\nAll Detected GPUs ({len(gpu_info['all_gpus'])}):")
        for i, gpu in enumerate(gpu_info['all_gpus'], 1):
            primary_indicator = " (Primary)" if gpu == primary_gpu else ""
            print(f"  {i}. {gpu['name']} - {gpu['vendor'].upper()}{primary_indicator}")
    
    # OptiScaler compatibility
    print("\n=== OptiScaler Compatibility ===")
    recommendations = check_optiscaler_compatibility(gpu_info)
    for rec in recommendations:
        print(rec)
    
    # Test dependencies
    print("\n=== Dependency Test ===")
    deps_ok = test_dependencies()
    
    if deps_ok:
        # Run performance test
        run_performance_test()
        
        # Test cache manager
        print("\n=== Cache Status ===")
        try:
            from utils.cache_manager import cache_manager
            stats = cache_manager.get_cache_stats()
            print(f"Total cache size: {stats['total_size_mb']:.1f} MB")
            print(f"Cached images: {stats['image_count']}")
            print(f"Image cache size: {stats['image_size_mb']:.1f} MB")
        except Exception as e:
            print(f"Cache test failed: {e}")
    
    print("\n" + "=" * 60)
    print("Debug complete!")

if __name__ == "__main__":
    main()
