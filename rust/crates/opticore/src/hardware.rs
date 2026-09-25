use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub gpus: Vec<GpuProfile>,
    pub windows_build: Option<String>,
    pub ram_bytes: Option<u64>,
    pub collected_at: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GpuProfile {
    /// Snapshot-only enumeration key. Clear selections when collecting a new profile.
    pub id: String,
    pub name: Option<String>,
    pub vendor: Option<String>,
    pub dedicated_bytes: Option<u64>,
    pub shared_bytes: Option<u64>,
    pub driver: Option<String>,
}

/// Collects local hardware facts. This is blocking; callers should run it off the UI thread.
pub fn collect() -> HardwareProfile {
    let mut profile = HardwareProfile {
        collected_at: time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .ok(),
        ..HardwareProfile::default()
    };

    #[cfg(windows)]
    collect_windows(&mut profile);

    #[cfg(not(windows))]
    profile
        .warnings
        .push("Native hardware collection is only supported on Windows".into());

    profile
}

#[cfg(windows)]
fn collect_windows(profile: &mut HardwareProfile) {
    use windows::core::Interface;
    use windows::Win32::Graphics::Dxgi::{
        CreateDXGIFactory1, IDXGIAdapter1, IDXGIDevice, IDXGIFactory1, DXGI_ERROR_NOT_FOUND,
    };
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    // SAFETY: The structure length is initialized as required by GlobalMemoryStatusEx.
    let mut memory = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    match unsafe { GlobalMemoryStatusEx(&mut memory) } {
        Ok(()) => profile.ram_bytes = Some(memory.ullTotalPhys),
        Err(error) => profile.warnings.push(format!("RAM unavailable: {error}")),
    }

    match windows_registry::LOCAL_MACHINE.open("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion") {
        Ok(key) => match (key.get_string("CurrentBuildNumber"), key.get_u32("UBR")) {
            (Ok(build), Ok(revision)) if !build.is_empty() => {
                profile.windows_build = Some(format!("{build}.{revision}"));
            }
            _ => profile
                .warnings
                .push("Windows build or update revision unavailable".into()),
        },
        Err(error) => profile
            .warnings
            .push(format!("Windows build registry unavailable: {error}")),
    }

    // SAFETY: DXGI owns the returned COM interfaces; windows-rs releases them on drop.
    let factory: IDXGIFactory1 = match unsafe { CreateDXGIFactory1() } {
        Ok(factory) => factory,
        Err(error) => {
            profile.warnings.push(format!("DXGI unavailable: {error}"));
            return;
        }
    };

    for index in 0.. {
        // SAFETY: The factory is live and the index is valid until DXGI reports NOT_FOUND.
        let adapter: IDXGIAdapter1 = match unsafe { factory.EnumAdapters1(index) } {
            Ok(adapter) => adapter,
            Err(error) if error.code() == DXGI_ERROR_NOT_FOUND => break,
            Err(error) => {
                profile
                    .warnings
                    .push(format!("GPU enumeration stopped: {error}"));
                break;
            }
        };

        // SAFETY: The adapter interface is live for this call.
        let desc = match unsafe { adapter.GetDesc1() } {
            Ok(desc) => desc,
            Err(error) => {
                profile
                    .warnings
                    .push(format!("GPU {index} description unavailable: {error}"));
                continue;
            }
        };

        // DXGI_ADAPTER_FLAG_SOFTWARE is 0x2. Never report a software adapter as hardware.
        if desc.Flags & 0x2 != 0 {
            continue;
        }

        let name_end = desc
            .Description
            .iter()
            .position(|&unit| unit == 0)
            .unwrap_or(desc.Description.len());
        let name = String::from_utf16_lossy(&desc.Description[..name_end]);
        let driver = match unsafe { adapter.CheckInterfaceSupport(&IDXGIDevice::IID) } {
            Ok(version) => {
                let bits = version as u64;
                Some(format!(
                    "{}.{}.{}.{}",
                    (bits >> 48) & 0xffff,
                    (bits >> 32) & 0xffff,
                    (bits >> 16) & 0xffff,
                    bits & 0xffff
                ))
            }
            Err(error) => {
                profile
                    .warnings
                    .push(format!("GPU {index} driver unavailable: {error}"));
                None
            }
        };

        profile.gpus.push(GpuProfile {
            id: format!("gpu-{index}"),
            name: (!name.is_empty()).then_some(name),
            vendor: match desc.VendorId {
                0x10de => Some("NVIDIA".into()),
                0x1002 | 0x1022 => Some("AMD".into()),
                0x8086 => Some("Intel".into()),
                0 => None,
                id => Some(format!("0x{id:04X}")),
            },
            dedicated_bytes: Some(desc.DedicatedVideoMemory as u64),
            shared_bytes: Some(desc.SharedSystemMemory as u64),
            driver,
        });
    }
}
