@echo off
powershell -Command "Get-Content src\optiscaler\manager.py | %%{$_ -replace \"result = subprocess.run.*wmic.*name.*capture_output.*text.*\", \"result = subprocess.run([^\"powershell^\",-Command^\"Get-CimInstance -ClassName Win32_VideoController | Select-Object Name^\"],[capture_output=True,text=True,timeout=5])\"} | Set-Content src\optiscaler\manager_fixed.py"
copy src\optiscaler\manager_fixed.py src\optiscaler\manager.py
