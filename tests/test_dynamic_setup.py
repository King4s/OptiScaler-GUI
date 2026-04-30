import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'src'))

from optiscaler.manager import OptiScalerManager


def test_dynamic_setup_nvngx_handling(test_env_path):
    test_env = test_env_path
    extracted_path = test_env / 'fixtures' / 'extracted_test'
    extracted_path.mkdir(parents=True, exist_ok=True)

    # Create OptiScaler.dll and nvngx_dlss.dll
    (extracted_path / 'OptiScaler.dll').write_bytes(b'OPTISCALER')
    (extracted_path / 'nvngx_dlss.dll').write_bytes(b'NVNGX_DUMMY')

    game_dir = test_env / 'mock_games' / 'DynamicGame'
    game_dir.mkdir(parents=True, exist_ok=True)

    man = OptiScalerManager(download_dir=str(test_env / 'cache'))

    # v0.7.9+ does not create nvngx.dll for DLSS Inputs. OptiScaler handles this
    # internally; disabling DLSS Inputs writes Dxgi=false instead.
    success, message = man.run_dynamic_setup(str(extracted_path), str(game_dir), 'dxgi.dll', 'amd', use_dlss=True, dlss_source_path=str(extracted_path / 'nvngx_dlss.dll'), overwrite=True)
    assert success, message
    assert not (game_dir / 'nvngx.dll').exists(), 'nvngx.dll should not be created when use_dlss=True'

    # Clean
    if (game_dir / 'nvngx.dll').exists():
        (game_dir / 'nvngx.dll').unlink()
    if (game_dir / 'dxgi.dll').exists():
        (game_dir / 'dxgi.dll').unlink()

    # Run dynamic setup with DLSS = False
    success2, message2 = man.run_dynamic_setup(str(extracted_path), str(game_dir), 'dxgi.dll', 'amd', use_dlss=False, overwrite=True)
    assert success2, message2
    assert not (game_dir / 'nvngx.dll').exists(), 'nvngx.dll should not be present when use_dlss=False'
    ini_text = (game_dir / 'OptiScaler.ini').read_text(encoding='utf-8')
    assert 'dxgi = false' in ini_text.lower()


def test_dynamic_setup_copies_v09_payload(test_env_path):
    test_env = test_env_path
    extracted_path = test_env / 'fixtures' / 'extracted_v09'
    extracted_path.mkdir(parents=True, exist_ok=True)

    files = [
        'OptiScaler.dll',
        'OptiScaler.ini',
        'fakenvapi.dll',
        'fakenvapi.ini',
        'dlssg_to_fsr3_amd_is_better.dll',
        'libxell.dll',
        'libxess_fg.dll',
        'amd_fidelityfx_framegeneration_dx12.dll',
        'amd_fidelityfx_upscaler_dx12.dll',
    ]
    for filename in files:
        (extracted_path / filename).write_bytes(b'PAYLOAD')

    d3d12_dir = extracted_path / 'D3D12_Optiscaler'
    d3d12_dir.mkdir(exist_ok=True)
    (d3d12_dir / 'D3D12Core.dll').write_bytes(b'D3D12')

    licenses_dir = extracted_path / 'Licenses'
    licenses_dir.mkdir(exist_ok=True)
    (licenses_dir / 'XeSS_LICENSE.txt').write_text('license', encoding='utf-8')

    game_dir = test_env / 'mock_games' / 'V09Game'
    game_dir.mkdir(parents=True, exist_ok=True)
    (game_dir / 'nvapi64.dll').write_bytes(b'OLD')

    man = OptiScalerManager(download_dir=str(test_env / 'cache'))
    success, message = man.run_dynamic_setup(str(extracted_path), str(game_dir), 'dxgi.dll', 'amd', use_dlss=True, overwrite=True)

    assert success, message
    assert (game_dir / 'dxgi.dll').exists()
    assert (game_dir / 'fakenvapi.dll').exists()
    assert (game_dir / 'dlssg_to_fsr3_amd_is_better.dll').exists()
    assert (game_dir / 'libxell.dll').exists()
    assert (game_dir / 'libxess_fg.dll').exists()
    assert (game_dir / 'amd_fidelityfx_framegeneration_dx12.dll').exists()
    assert (game_dir / 'amd_fidelityfx_upscaler_dx12.dll').exists()
    assert (game_dir / 'D3D12_Optiscaler' / 'D3D12Core.dll').exists()
    assert (game_dir / 'Licenses' / 'XeSS_LICENSE.txt').exists()
    assert not (game_dir / 'nvapi64.dll').exists()
