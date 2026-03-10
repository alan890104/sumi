use serde::Serialize;

use crate::settings::models_dir;

// ── SystemInfo ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct SystemInfo {
    pub total_ram_bytes: u64,
    pub available_disk_bytes: u64,
    pub is_apple_silicon: bool,
    pub gpu_vram_bytes: u64,
    pub has_cuda: bool,
    pub os: String,
    pub arch: String,
    pub cpu_model: String,
}

/// Detect system information (RAM, disk space, CPU architecture, GPU VRAM, CPU model).
pub fn detect_system_info() -> SystemInfo {
    let total_ram_bytes = get_total_ram();
    let available_disk_bytes = get_available_disk_space();
    let gpu_vram_bytes = get_gpu_vram();
    let arch = std::env::consts::ARCH.to_string();
    let is_apple_silicon = cfg!(target_os = "macos") && arch == "aarch64";
    let has_cuda = cfg!(feature = "cuda");

    SystemInfo {
        total_ram_bytes,
        available_disk_bytes,
        is_apple_silicon,
        gpu_vram_bytes,
        has_cuda,
        os: std::env::consts::OS.to_string(),
        arch,
        cpu_model: get_cpu_model(),
    }
}

// ── System language detection ─────────────────────────────────────────────────

/// Detect the system language via `tauri-plugin-os` (cross-platform).
/// Returns a lowercased BCP-47 tag, e.g. `"zh-tw"`, `"ja"`, `"en-us"`.
pub fn detect_system_language() -> Option<String> {
    tauri_plugin_os::locale().map(|s| s.to_lowercase())
}

// ── Platform-specific helpers ─────────────────────────────────────────────────

#[cfg(unix)]
fn get_total_ram() -> u64 {
    #[cfg(target_os = "macos")]
    {
        use std::mem;
        let mut size: u64 = 0;
        let mut len = mem::size_of::<u64>();
        let mib = [libc::CTL_HW, libc::HW_MEMSIZE];
        let ret = unsafe {
            libc::sysctl(
                mib.as_ptr() as *mut _,
                2,
                &mut size as *mut u64 as *mut _,
                &mut len,
                std::ptr::null_mut(),
                0,
            )
        };
        if ret == 0 { size } else { 0 }
    }
    #[cfg(not(target_os = "macos"))]
    {
        unsafe {
            let info: libc::sysinfo = std::mem::zeroed();
            if libc::sysinfo(&info as *const _ as *mut _) == 0 {
                info.totalram as u64 * info.mem_unit as u64
            } else {
                0
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn get_total_ram() -> u64 {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    unsafe {
        let mut mem_info = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            ..std::mem::zeroed()
        };
        if GlobalMemoryStatusEx(&mut mem_info).is_ok() {
            mem_info.ullTotalPhys
        } else {
            0
        }
    }
}

#[cfg(not(any(unix, target_os = "windows")))]
fn get_total_ram() -> u64 {
    0
}

fn get_available_disk_space() -> u64 {
    let models = models_dir();
    let _ = std::fs::create_dir_all(&models);

    #[cfg(unix)]
    {
        use std::ffi::CString;
        let path_c = match CString::new(models.to_string_lossy().as_bytes()) {
            Ok(c) => c,
            Err(_) => return 0,
        };
        unsafe {
            let mut stat: libc::statvfs = std::mem::zeroed();
            if libc::statvfs(path_c.as_ptr(), &mut stat) == 0 {
                stat.f_bavail as u64 * stat.f_frsize
            } else {
                0
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
        use windows::core::HSTRING;
        let path_str = models.to_string_lossy().to_string();
        let path_w = HSTRING::from(path_str);
        let mut free_bytes_available: u64 = 0;
        unsafe {
            if GetDiskFreeSpaceExW(
                &path_w,
                Some(&mut free_bytes_available),
                None,
                None,
            )
            .is_ok()
            {
                free_bytes_available
            } else {
                0
            }
        }
    }

    #[cfg(not(any(unix, target_os = "windows")))]
    {
        let _ = models;
        0
    }
}

/// Detect the largest dedicated GPU VRAM via DXGI (Windows only).
/// Returns 0 on non-Windows platforms or if no discrete GPU is found.
fn get_gpu_vram() -> u64 {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1};
        let factory: IDXGIFactory1 = match unsafe { CreateDXGIFactory1() } {
            Ok(f) => f,
            Err(_) => return 0,
        };
        let mut max_vram: u64 = 0;
        let mut i = 0u32;
        loop {
            let adapter = match unsafe { factory.EnumAdapters(i) } {
                Ok(a) => a,
                Err(_) => break,
            };
            if let Ok(desc) = unsafe { adapter.GetDesc() } {
                let dedicated = desc.DedicatedVideoMemory as u64;
                if dedicated > max_vram {
                    max_vram = dedicated;
                }
            }
            i += 1;
        }
        max_vram
    }

    #[cfg(not(target_os = "windows"))]
    {
        0
    }
}

/// Read the CPU model string from the OS.
/// macOS: `machdep.cpu.brand_string` sysctl (works on both Intel and Apple Silicon).
/// Windows: `ProcessorNameString` registry value under CentralProcessor\0.
fn get_cpu_model() -> String {
    #[cfg(target_os = "macos")]
    {
        use std::ffi::CStr;
        let name = b"machdep.cpu.brand_string\0";
        let mut size: libc::size_t = 0;
        unsafe {
            libc::sysctlbyname(
                name.as_ptr() as *const libc::c_char,
                std::ptr::null_mut(),
                &mut size,
                std::ptr::null_mut(),
                0,
            );
        }
        if size == 0 {
            return "unknown".to_string();
        }
        let mut buf = vec![0u8; size];
        let ret = unsafe {
            libc::sysctlbyname(
                name.as_ptr() as *const libc::c_char,
                buf.as_mut_ptr() as *mut libc::c_void,
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        };
        if ret == 0 && size > 0 {
            CStr::from_bytes_until_nul(&buf)
                .map(|s| s.to_string_lossy().trim().to_string())
                .unwrap_or_else(|_| "unknown".to_string())
        } else {
            "unknown".to_string()
        }
    }

    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::Registry::{RegGetValueW, RRF_RT_REG_SZ, HKEY_LOCAL_MACHINE};
        let mut buf = [0u16; 256];
        let mut size = (buf.len() * 2) as u32;
        let result = unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                windows::core::w!("HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0"),
                windows::core::w!("ProcessorNameString"),
                RRF_RT_REG_SZ,
                None,
                Some(buf.as_mut_ptr() as *mut core::ffi::c_void),
                Some(&mut size),
            )
        };
        if result.is_ok() {
            let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
            String::from_utf16_lossy(&buf[..len]).trim().to_string()
        } else {
            "unknown".to_string()
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        "unknown".to_string()
    }
}
