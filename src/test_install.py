from optiscaler.manager import OptiScalerManager
import traceback

print("Testing OptiScaler installation...")

mgr = OptiScalerManager()

try:
    print("Step 1: Testing download...")
    result = mgr._download_latest_release()
    print(f"Download result: {result}")
    
    if result:
        print("Step 2: Testing extraction...")
        extracted = mgr._extract_release(result)
        print(f"Extraction result: {extracted}")
        
        if extracted:
            print("Step 3: Testing installation...")
            install_result = mgr.install_optiscaler(r'C:\Program Files (x86)\Steam\steamapps\common\Soulmask')
            print(f"Installation result: {install_result}")
        else:
            print("Extraction failed")
    else:
        print("Download failed")

except Exception as e:
    print(f"Error: {e}")
    traceback.print_exc()
