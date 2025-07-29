# Custom hook to block numpy and OpenBLAS from being included
from PyInstaller.utils.hooks import collect_data_files, collect_dynamic_libs

# Block all numpy dynamic libraries
def hook(hook_api):
    # Remove all numpy binaries including OpenBLAS
    hook_api.add_runtime_package('numpy')
    return []

# Explicitly exclude all OpenBLAS libraries
excludedimports = ['numpy', 'numpy.core', 'numpy.linalg', 'openblas']
datas = []
binaries = []
