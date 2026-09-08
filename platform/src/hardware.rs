use std::collections::HashMap;
use std::time::Instant;

use sysinfo::{Components, CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};
use vigilon_core::metrics::{
    CpuMetrics, DeviceInfo, DiskMetrics, GpuMetrics, GpuProcess, MemoryMetrics,
};

pub struct HardwareCollector {
    sys: System,
    disks: Disks,
    last_disk: HashMap<String, (u64, u64, Instant)>,
}

impl HardwareCollector {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        sys.refresh_cpu_all();
        Self {
            sys,
            disks: Disks::new_with_refreshed_list(),
            last_disk: HashMap::new(),
        }
    }

    pub fn device(&mut self) -> DeviceInfo {
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();
        let cpu_model = self
            .sys
            .cpus()
            .first()
            .map(|c| c.brand().to_string())
            .unwrap_or_else(|| "unknown".into());
        let freq = self.sys.cpus().first().map(|c| c.frequency() as f32);
        let hostname = hostname::get()
            .ok()
            .map(|h| h.to_string_lossy().into_owned())
            .or_else(System::host_name)
            .unwrap_or_else(|| "localhost".into());
        DeviceInfo {
            id: format!("dev-{hostname}"),
            hostname,
            os_name: System::name().unwrap_or_else(|| std::env::consts::OS.into()),
            os_version: System::os_version().unwrap_or_default(),
            kernel: System::kernel_version(),
            cpu_model,
            physical_cores: System::physical_core_count().unwrap_or(0) as u32,
            logical_cores: self.sys.cpus().len() as u32,
            base_frequency_mhz: freq,
            motherboard: motherboard_hint(),
            bios_version: bios_hint(),
            total_memory_bytes: self.sys.total_memory(),
        }
    }

    pub fn cpu_memory(&mut self) -> (CpuMetrics, MemoryMetrics) {
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();
        let per_core: Vec<f32> = self.sys.cpus().iter().map(|c| c.cpu_usage()).collect();
        let freq = self.sys.cpus().first().map(|c| c.frequency() as f32);
        let mut components = Components::new_with_refreshed_list();
        components.refresh(true);
        let temperature_c = components.iter().find_map(|c| {
            let label = c.label().to_lowercase();
            if label.contains("cpu") || label.contains("package") || label.contains("tdie") {
                c.temperature()
            } else {
                None
            }
        });
        let cpu = CpuMetrics {
            usage_percent: self.sys.global_cpu_usage(),
            per_core,
            frequency_mhz: freq,
            temperature_c,
        };
        let mem = MemoryMetrics {
            total_bytes: self.sys.total_memory(),
            used_bytes: self.sys.used_memory(),
            available_bytes: self.sys.available_memory(),
            swap_total_bytes: self.sys.total_swap(),
            swap_used_bytes: self.sys.used_swap(),
        };
        (cpu, mem)
    }

    pub fn disks(&mut self) -> Vec<DiskMetrics> {
        self.disks.refresh(true);
        let now = Instant::now();
        let mut out = Vec::new();
        for disk in self.disks.iter() {
            let name = disk.name().to_string_lossy().into_owned();
            let mount = disk.mount_point().to_string_lossy().into_owned();
            let key = format!("{name}|{mount}");
            let usage = disk.usage();
            let (read_bps, write_bps) = if let Some((pr, pw, t)) = self.last_disk.get(&key) {
                let dt = now.duration_since(*t).as_secs_f64().max(0.001);
                (
                    (usage.read_bytes.saturating_sub(*pr)) as f64 / dt,
                    (usage.written_bytes.saturating_sub(*pw)) as f64 / dt,
                )
            } else {
                (0.0, 0.0)
            };
            self.last_disk
                .insert(key, (usage.read_bytes, usage.written_bytes, now));
            let total = disk.total_space();
            let used = total.saturating_sub(disk.available_space());
            let smart_status = smart_hint(&name, &mount);
            out.push(DiskMetrics {
                name,
                mount_point: mount,
                file_system: disk.file_system().to_string_lossy().into_owned(),
                total_bytes: total,
                used_bytes: used,
                read_bps,
                write_bps,
                smart_status,
            });
        }
        out
    }
}

pub fn gpus() -> Vec<GpuMetrics> {
    match nvml() {
        Some(list) if !list.is_empty() => list,
        _ => amd_or_sysfs_gpus(),
    }
}

fn nvml() -> Option<Vec<GpuMetrics>> {
    let nvml = nvml_wrapper::Nvml::init().ok()?;
    let count = nvml.device_count().ok()?;
    let mut out = Vec::new();
    for i in 0..count {
        let dev = nvml.device_by_index(i).ok()?;
        let name = dev.name().unwrap_or_else(|_| format!("GPU {i}"));
        let util = dev.utilization_rates().ok().map(|u| u.gpu as f32);
        let mem = dev.memory_info().ok();
        let temp = dev
            .temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
            .ok()
            .map(|t| t as f32);
        let mut processes = Vec::new();
        if let Ok(running) = dev.running_compute_processes() {
            for p in running {
                processes.push(GpuProcess {
                    pid: p.pid,
                    name: None,
                    memory_bytes: match p.used_gpu_memory {
                        nvml_wrapper::enums::device::UsedGpuMemory::Used(b) => Some(b),
                        nvml_wrapper::enums::device::UsedGpuMemory::Unavailable => None,
                    },
                });
            }
        }
        out.push(GpuMetrics {
            name,
            usage_percent: util,
            memory_total_bytes: mem.as_ref().map(|m| m.total),
            memory_used_bytes: mem.as_ref().map(|m| m.used),
            temperature_c: temp,
            processes,
        });
    }
    Some(out)
}

/// Linux AMD/Intel best-effort via sysfs (no nvidia-smi).
fn amd_or_sysfs_gpus() -> Vec<GpuMetrics> {
    #[cfg(target_os = "linux")]
    {
        let mut out = Vec::new();
        let Ok(entries) = std::fs::read_dir("/sys/class/drm") else {
            return out;
        };
        for e in entries.flatten() {
            let name = e.file_name();
            let n = name.to_string_lossy();
            if !n.starts_with("card") || n.contains('-') {
                continue;
            }
            let dev = e.path().join("device");
            let busy = std::fs::read_to_string(dev.join("gpu_busy_percent"))
                .ok()
                .and_then(|s| s.trim().parse::<f32>().ok());
            let mem_used = std::fs::read_to_string(dev.join("mem_info_vram_used"))
                .ok()
                .and_then(|s| s.trim().parse::<u64>().ok());
            let mem_total = std::fs::read_to_string(dev.join("mem_info_vram_total"))
                .ok()
                .and_then(|s| s.trim().parse::<u64>().ok());
            let vendor = std::fs::read_to_string(dev.join("vendor"))
                .unwrap_or_default()
                .trim()
                .to_string();
            if busy.is_none() && mem_total.is_none() {
                continue;
            }
            out.push(GpuMetrics {
                name: format!("DRM {n} ({vendor})"),
                usage_percent: busy,
                memory_total_bytes: mem_total,
                memory_used_bytes: mem_used,
                temperature_c: None,
                processes: Vec::new(),
            });
        }
        out
    }
    #[cfg(not(target_os = "linux"))]
    {
        Vec::new()
    }
}

fn smart_hint(name: &str, mount: &str) -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let base = name.trim_start_matches("/dev/");
        let base = base.split('/').next_back().unwrap_or(base);
        let path = format!("/sys/block/{base}/device/state");
        if let Ok(s) = std::fs::read_to_string(&path) {
            return Some(s.trim().to_string());
        }
        let _ = mount;
        None
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (name, mount);
        None
    }
}

fn motherboard_hint() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/sys/class/dmi/id/board_name")
            .ok()
            .map(|s| s.trim().to_string())
    }
    #[cfg(windows)]
    {
        windows_bios("BaseBoardProduct")
    }
    #[cfg(target_os = "macos")]
    {
        macos_sysctl("hw.model")
    }
    #[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
    {
        None
    }
}

fn bios_hint() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/sys/class/dmi/id/bios_version")
            .ok()
            .map(|s| s.trim().to_string())
    }
    #[cfg(windows)]
    {
        windows_bios("BIOSVersion")
    }
    #[cfg(target_os = "macos")]
    {
        macos_sysctl("kern.osversion")
    }
    #[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
    {
        None
    }
}

#[cfg(windows)]
fn windows_bios(value: &str) -> Option<String> {
    let key = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE)
        .open_subkey(r"HARDWARE\DESCRIPTION\System\BIOS")
        .ok()?;
    key.get_value::<String, _>(value)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(target_os = "macos")]
fn macos_sysctl(key: &str) -> Option<String> {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, String>>> =
        std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
    if let Ok(g) = cache.lock() {
        if let Some(v) = g.get(key) {
            return Some(v.clone());
        }
    }
    let out = std::process::Command::new("/usr/sbin/sysctl")
        .args(["-n", key])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        return None;
    }
    if let Ok(mut g) = cache.lock() {
        g.insert(key.to_string(), s.clone());
    }
    Some(s)
}
