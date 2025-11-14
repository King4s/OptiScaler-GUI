# OptiScaler v0.7.9 Update Summary **(moved to docs/V0_7_9_UPDATE_SUMMARY.md)**

**Date Updated**: November 13, 2025  
**OptiScaler Release**: https://github.com/optiscaler/OptiScaler/releases/tag/v0.7.9  
**Project Compatibility**: v0.7.0 - v0.7.9

This summary has been moved to `docs/V0_7_9_UPDATE_SUMMARY.md` — please consult the docs folder for the full update summary.

## What Changed in OptiScaler v0.7.9

### 🔄 Major Changes
1. **DLSS Inputs Behavior** (Most Important)
   - No longer creates `nvngx.dll` when user selects "Yes" to DLSS Inputs (AMD/Intel)
   - Only modifies `Dxgi=false` when user selects "No"
   - This reduces file count and installation complexity

2. **FSR 4.0.2 & FidelityFX SDK 2.0.0**
   - Added support for latest FidelityFX SDK 2.0.0
   - FSR 4.0.2 and FSR 3.1.5 bundled (unsigned DLLs for RDNA4 compatibility)

3. **Bug Fixes & Improvements**
   - Fixed menu rendering under Vulkan (Linux/Proton fix)
   - Fixed NMS game crashes
   - Added OptiPatcher detection in overlay
   - More game quirks added
   - Overlay visual glitching fixes

## Project Updates to Support v0.7.9

### 📝 Files Modified

#### 1. `README.md`
- Updated version compatibility table to include v0.7.9
- Changed from: `v0.7.0 - v0.7.7-pre9`
- Changed to: `v0.7.0 - v0.7.9`

#### 2. `docs/CHANGELOG.md`
- Added new unreleased section documenting v0.7.9 support
- Documented DLSS Inputs behavior change
- Listed FSR 4.0.2 support
- Noted all affected files

#### 3. `.github/copilot-instructions.md`
- Added v0.7.9 update note in Integration section
- Documented DLSS Inputs behavior change for AI agents
- Highlighted that only "No" selection modifies `Dxgi=false`

#### 4. `src/optiscaler/manager.py`
- **OptiScalerManager class docstring**: Added IMPORTANT COMPATIBILITY NOTES section
  - v0.7.9+: DLSS Inputs (AMD/Intel) no longer creates nvngx.dll
  - v0.7.8+: Updated default configuration structure
  - v0.7.9+: Added FSR 4.0.2 and FidelityFX SDK 2.0.0 support

- **OptiScalerConfig.ADDITIONAL_FILES**: Added FidelityFX SDK 2.0.0 files
  - `amd_fidelityfx_dx12_v2.dll` (FidelityFX SDK 2.0.0)
  - `amd_fidelityfx_vk_v2.dll` (FidelityFX SDK 2.0.0)

## Implementation Notes

### What Still Works
- All existing installation methods (dxgi.dll, winmm.dll, etc.)
- Configuration management and INI file handling
- Archive extraction (7z, py7zr, zipfile fallback)
- Game detection and scanning
- Uninstall functionality

### What Changed in User Experience
- **AMD/Intel DLSS Inputs**: Users selecting "Yes" will no longer get an extra `nvngx.dll` file
- **AMD/Intel DLSS Inputs**: Users selecting "No" will see `Dxgi=false` in their OptiScaler.ini
- Cleaner installation with fewer files when not using DLSS spoofing

### Testing Recommendations
1. Test AMD GPU installation with DLSS Inputs = "Yes" (should NOT create nvngx.dll)
2. Test AMD GPU installation with DLSS Inputs = "No" (should set Dxgi=false)
3. Test NVIDIA GPU installation (no change in behavior)
4. Verify FSR 4.0.2 files are properly copied when available
5. Check that new FidelityFX v2 DLLs don't break games

## Next Steps

When releasing next version of OptiScaler-GUI:
1. Test with v0.7.9 OptiScaler release
2. Verify DLSS Inputs behavior matches documentation
3. Update GUI version if needed (currently 0.3.6)
4. Build and test portable executable
5. Consider mentioning FSR 4.0.2 support in release notes

## References
- OptiScaler v0.7.9 Release: https://github.com/optiscaler/OptiScaler/releases/tag/v0.7.9
- OptiScaler GitHub: https://github.com/optiscaler/OptiScaler
- OptiPatcher (mentioned in v0.7.9): https://github.com/optiscaler/OptiPatcher
