import sys
from pathlib import Path
import json
import zipfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'src'))

from optiscaler.manager import OptiScalerManager
from utils.update_manager import OptiScalerUpdateManager


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

    manifest_path = game_dir / '.optiscaler-gui-install.json'
    assert manifest_path.exists()
    manifest = json.loads(manifest_path.read_text(encoding='utf-8'))
    assert manifest['target_filename'] == 'dxgi.dll'
    assert 'fakenvapi.dll' in manifest['files']
    assert 'D3D12_Optiscaler' in manifest['directories']

    success, message = man.uninstall_optiscaler(str(game_dir))
    assert success, message
    assert not (game_dir / 'dxgi.dll').exists()
    assert not (game_dir / 'fakenvapi.dll').exists()
    assert not (game_dir / 'D3D12_Optiscaler').exists()
    assert not manifest_path.exists()


def test_install_update_uninstall_end_to_end_preserves_target_and_backs_up_config(test_env_path):
    test_env = test_env_path
    archive_path = test_env / 'fixtures' / 'archives' / 'OptiScaler_e2e.zip'
    archive_path.parent.mkdir(parents=True, exist_ok=True)

    with zipfile.ZipFile(archive_path, 'w') as zf:
        zf.writestr('OptiScaler.dll', b'OPTISCALER')
        zf.writestr('OptiScaler.ini', '[OptiScaler]\nFromArchive=true\n')
        zf.writestr('fakenvapi.dll', b'FAKE')
        zf.writestr('D3D12_Optiscaler/D3D12Core.dll', b'D3D12')

    game_dir = test_env / 'mock_games' / 'E2EGame'
    if game_dir.exists():
        import shutil
        shutil.rmtree(game_dir)
    game_dir.mkdir(parents=True)

    man = OptiScalerManager(download_dir=str(test_env / 'cache'))
    man._last_release_info = {'tag_name': 'v-test', 'html_url': 'https://example.invalid/release'}
    man._download_latest_release = lambda progress_callback=None: str(archive_path)

    success, message = man.install_optiscaler(str(game_dir), target_filename='dxgi.dll', overwrite=True)
    assert success, message
    assert (game_dir / 'dxgi.dll').exists()
    assert (game_dir / 'fakenvapi.dll').exists()

    manifest = json.loads((game_dir / '.optiscaler-gui-install.json').read_text(encoding='utf-8'))
    assert manifest['target_filename'] == 'dxgi.dll'
    assert manifest['optiscaler_version'] == 'v-test'

    update_manager = OptiScalerUpdateManager()
    update_manager.cache_dir = test_env / 'cache'
    update_manager.version_cache_file = update_manager.cache_dir / 'optiscaler_version_cache.json'
    update_manager.save_version_cache({'latest_known_version': 'v-test-2'})

    original_update_manager = update_manager
    # Patch the manager used inside this update manager instance.
    import utils.update_manager as update_module
    original_manager_cls = update_module.OptiScalerManager
    update_module.OptiScalerManager = lambda: man
    try:
        (game_dir / 'OptiScaler.ini').write_text('[OptiScaler]\nUserValue=true\n', encoding='utf-8')
        success, message = original_update_manager.update_optiscaler_for_game(str(game_dir))
    finally:
        update_module.OptiScalerManager = original_manager_cls

    assert success, message
    assert (game_dir / 'dxgi.dll').exists()
    assert not (game_dir / 'nvngx.dll').exists()
    assert list(game_dir.glob('OptiScaler.ini.*.backup'))

    version_info = original_update_manager.get_installed_version_info(str(game_dir))
    assert version_info['installed']
    assert version_info['target_filename'] == 'dxgi.dll'

    success, message = man.uninstall_optiscaler(str(game_dir))
    assert success, message
    assert not (game_dir / 'dxgi.dll').exists()
    assert not (game_dir / 'fakenvapi.dll').exists()


def test_install_rollback_removes_partial_payload_on_failure(test_env_path, monkeypatch):
    test_env = test_env_path
    archive_path = test_env / 'fixtures' / 'archives' / 'OptiScaler_rollback.zip'
    archive_path.parent.mkdir(parents=True, exist_ok=True)

    with zipfile.ZipFile(archive_path, 'w') as zf:
        zf.writestr('OptiScaler.dll', b'OPTISCALER')
        zf.writestr('fakenvapi.dll', b'FAKE')

    game_dir = test_env / 'mock_games' / 'RollbackGame'
    if game_dir.exists():
        import shutil
        shutil.rmtree(game_dir)
    game_dir.mkdir(parents=True)

    man = OptiScalerManager(download_dir=str(test_env / 'cache'))
    man._download_latest_release = lambda progress_callback=None: str(archive_path)

    def fail_manifest(*args, **kwargs):
        raise RuntimeError('manifest failure')

    monkeypatch.setattr(man, '_write_install_manifest', fail_manifest)

    success, message = man.install_optiscaler(str(game_dir), target_filename='dxgi.dll', overwrite=True)
    assert not success
    assert 'manifest failure' in message
    assert not (game_dir / 'dxgi.dll').exists()
    assert not (game_dir / 'fakenvapi.dll').exists()
