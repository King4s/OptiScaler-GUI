from optiscaler.manager import OptiScalerManager
import traceback
import os

print("Testing OptiScaler installation with detailed logging...")

mgr = OptiScalerManager()

game_path = r'C:\Program Files (x86)\Steam\steamapps\common\Soulmask'
extracted_path = r'C:\OptiScaler-GUI\cache\optiscaler_downloads\extracted_optiscaler'

print(f"Game path: {game_path}")
print(f"Extracted path: {extracted_path}")

# Check if game path exists
if not os.path.exists(game_path):
    print(f"ERROR: Game path does not exist: {game_path}")
    exit(1)

# Check for OptiScaler.dll
optiscaler_dll_path = os.path.join(extracted_path, 'OptiScaler.dll')
print(f"Looking for OptiScaler.dll at: {optiscaler_dll_path}")
print(f"OptiScaler.dll exists: {os.path.exists(optiscaler_dll_path)}")

# Test destination directory
dest_dir = game_path
target_filename = 'dxgi.dll'
target_path = os.path.join(dest_dir, target_filename)
print(f"Target path: {target_path}")
print(f"Target already exists: {os.path.exists(target_path)}")

# Test write permissions
try:
    test_file = os.path.join(dest_dir, 'test_write.txt')
    with open(test_file, 'w') as f:
        f.write('test')
    os.remove(test_file)
    print("Write permissions: OK")
except Exception as e:
    print(f"Write permissions: FAILED - {e}")

# Now try the actual installation
try:
    print("\nStarting installation...")
    result = mgr.install_optiscaler(game_path, target_filename, overwrite=True)
    print(f"Installation result: {result}")
except Exception as e:
    print(f"Installation error: {e}")
    traceback.print_exc()
