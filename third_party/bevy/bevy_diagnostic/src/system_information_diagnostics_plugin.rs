use alloc::string::String;
use bevy_ecs::resource::Resource;

/// A resource that stores diagnostic information about the system.
/// This information can be useful for debugging and profiling purposes.
#[derive(Debug, Resource)]
pub struct SystemInfo {
    /// OS name and version.
    pub os: String,
    /// System kernel version.
    pub kernel: String,
    /// CPU model name.
    pub cpu: String,
    /// Physical core count.
    pub core_count: String,
    /// System RAM.
    pub memory: String,
}

// NOTE: sysinfo fails to compile when using bevy dynamic or on iOS and does nothing on Wasm
#[cfg(all(
    any(target_os = "linux", target_os = "windows", target_os = "macos"),
    feature = "std",
))]
mod internal {
    use alloc::{
        format,
        string::{String, ToString},
    };
    use log::info;
    use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

    use super::SystemInfo;

    const BYTES_TO_GIB: f64 = 1.0 / 1024.0 / 1024.0 / 1024.0;

    impl Default for SystemInfo {
        fn default() -> Self {
            let sys = System::new_with_specifics(
                RefreshKind::nothing()
                    .with_cpu(CpuRefreshKind::nothing())
                    .with_memory(MemoryRefreshKind::nothing().with_ram()),
            );

            let system_info = SystemInfo {
                os: System::long_os_version().unwrap_or_else(|| String::from("not available")),
                kernel: System::kernel_version().unwrap_or_else(|| String::from("not available")),
                cpu: sys
                    .cpus()
                    .first()
                    .map(|cpu| cpu.brand().trim().to_string())
                    .unwrap_or_else(|| String::from("not available")),
                core_count: System::physical_core_count()
                    .map(|x| x.to_string())
                    .unwrap_or_else(|| String::from("not available")),
                // Convert from Bytes to GibiBytes since it's probably what people expect most of the time
                memory: format!("{:.1} GiB", sys.total_memory() as f64 * BYTES_TO_GIB),
            };

            info!("{system_info:?}");
            system_info
        }
    }
}

#[cfg(not(all(
    any(target_os = "linux", target_os = "windows", target_os = "macos"),
    feature = "std",
)))]
mod internal {
    use alloc::string::ToString;

    impl Default for super::SystemInfo {
        fn default() -> Self {
            let unknown = "Unknown".to_string();
            Self {
                os: unknown.clone(),
                kernel: unknown.clone(),
                cpu: unknown.clone(),
                core_count: unknown.clone(),
                memory: unknown.clone(),
            }
        }
    }
}
