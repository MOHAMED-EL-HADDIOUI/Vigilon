use vigilon_core::metrics::ServiceInfo;

pub fn list() -> Vec<ServiceInfo> {
    #[cfg(windows)]
    {
        windows_services()
    }
    #[cfg(target_os = "linux")]
    {
        linux_services()
    }
    #[cfg(target_os = "macos")]
    {
        macos_services()
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        Vec::new()
    }
}

pub fn failed_logins_hint() -> u32 {
    #[cfg(target_os = "linux")]
    {
        linux_failed_logins()
    }
    #[cfg(windows)]
    {
        windows_failed_logins()
    }
    #[cfg(target_os = "macos")]
    {
        macos_failed_logins()
    }
    #[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
    {
        0
    }
}

#[cfg(windows)]
fn windows_services() -> Vec<ServiceInfo> {
    use windows::Win32::System::Services::{
        CloseServiceHandle, ENUM_SERVICE_STATUS_PROCESSW, EnumServicesStatusExW, OpenSCManagerW,
        SC_ENUM_PROCESS_INFO, SC_MANAGER_ENUMERATE_SERVICE, SERVICE_STATE_ALL, SERVICE_WIN32,
    };
    use windows::core::PCWSTR;

    unsafe {
        let Ok(scm) = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_ENUMERATE_SERVICE)
        else {
            return Vec::new();
        };
        let mut bytes_needed: u32 = 0;
        let mut resume: u32 = 0;
        let mut count: u32 = 0;
        let _ = EnumServicesStatusExW(
            scm,
            SC_ENUM_PROCESS_INFO,
            SERVICE_WIN32,
            SERVICE_STATE_ALL,
            None,
            &mut bytes_needed,
            &mut count,
            Some(&mut resume),
            None,
        );
        if bytes_needed == 0 {
            let _ = CloseServiceHandle(scm);
            return Vec::new();
        }
        let mut buf = vec![0u8; bytes_needed as usize];
        resume = 0;
        let ok = EnumServicesStatusExW(
            scm,
            SC_ENUM_PROCESS_INFO,
            SERVICE_WIN32,
            SERVICE_STATE_ALL,
            Some(&mut buf),
            &mut bytes_needed,
            &mut count,
            Some(&mut resume),
            None,
        );
        let mut out = Vec::new();
        if ok.is_ok() {
            // Bound the trusted count by the actual buffer: the service set
            // may have grown between the sizing call and this call.
            let max_entries = buf.len() / std::mem::size_of::<ENUM_SERVICE_STATUS_PROCESSW>();
            let n = (count as usize).min(max_entries).min(400);
            let slice =
                std::slice::from_raw_parts(buf.as_ptr() as *const ENUM_SERVICE_STATUS_PROCESSW, n);
            for s in slice.iter().take(400) {
                let name = s.lpServiceName.to_string().unwrap_or_default();
                let display = s.lpDisplayName.to_string().ok();
                let status_code = s.ServiceStatusProcess.dwCurrentState.0;
                let status = match status_code {
                    1 => "stopped",
                    4 => "running",
                    2 => "start_pending",
                    3 => "stop_pending",
                    _ => "other",
                };
                let pid = s.ServiceStatusProcess.dwProcessId;
                out.push(ServiceInfo {
                    name,
                    display_name: display,
                    status: status.into(),
                    pid: if pid == 0 { None } else { Some(pid) },
                });
            }
        }
        let _ = CloseServiceHandle(scm);
        out
    }
}

#[cfg(target_os = "linux")]
fn linux_services() -> Vec<ServiceInfo> {
    let from_cgroup = linux_cgroup_services();
    if !from_cgroup.is_empty() {
        return from_cgroup;
    }
    let Ok(entries) = std::fs::read_dir("/etc/systemd/system") else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.ends_with(".service") {
                Some(ServiceInfo {
                    name,
                    display_name: None,
                    status: "unit".into(),
                    pid: None,
                })
            } else {
                None
            }
        })
        .take(200)
        .collect()
}

#[cfg(target_os = "linux")]
fn linux_cgroup_services() -> Vec<ServiceInfo> {
    let Ok(entries) = std::fs::read_dir("/sys/fs/cgroup/system.slice") else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.ends_with(".service") {
                Some(ServiceInfo {
                    name,
                    display_name: None,
                    status: "running".into(),
                    pid: None,
                })
            } else {
                None
            }
        })
        .take(200)
        .collect()
}

#[cfg(target_os = "macos")]
fn macos_services() -> Vec<ServiceInfo> {
    let Ok(entries) = std::fs::read_dir("/Library/LaunchDaemons") else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.ends_with(".plist") {
                Some(ServiceInfo {
                    name,
                    display_name: None,
                    status: "launchd".into(),
                    pid: None,
                })
            } else {
                None
            }
        })
        .take(200)
        .collect()
}

#[cfg(windows)]
fn windows_failed_logins() -> u32 {
    use windows::Win32::System::EventLog::{
        EVT_HANDLE, EvtClose, EvtNext, EvtQuery, EvtQueryChannelPath, EvtQueryReverseDirection,
    };
    unsafe {
        let query = windows::core::w!(
            "*[System[(EventID=4625) and TimeCreated[timediff(@SystemTime) <= 600000]]]"
        );
        let Ok(handle) = EvtQuery(
            EVT_HANDLE::default(),
            windows::core::w!("Security"),
            query,
            EvtQueryChannelPath.0 | EvtQueryReverseDirection.0,
        ) else {
            return 0;
        };
        // The handle array must fit exactly the requested event count:
        // EvtNext writes up to `requested` handles into the buffer.
        const MAX_EVENTS: u32 = 400;
        let mut handles = [0isize; MAX_EVENTS as usize];
        let mut returned: u32 = 0;
        let _ = EvtNext(handle, &mut handles, MAX_EVENTS, 0, &mut returned);
        let n = returned.min(MAX_EVENTS);
        for h in handles.iter().take(n as usize) {
            let _ = EvtClose(EVT_HANDLE(*h));
        }
        let _ = EvtClose(handle);
        n
    }
}

#[cfg(target_os = "linux")]
fn linux_failed_logins() -> u32 {
    let text = std::fs::read_to_string("/var/log/auth.log")
        .or_else(|_| std::fs::read_to_string("/var/log/secure"))
        .unwrap_or_default();
    text.lines()
        .rev()
        .take(400)
        .filter(|l| l.contains("Failed password") || l.contains("authentication failure"))
        .count() as u32
}

#[cfg(target_os = "macos")]
fn macos_failed_logins() -> u32 {
    let text = std::fs::read_to_string("/var/log/system.log").unwrap_or_default();
    text.lines()
        .rev()
        .take(400)
        .filter(|l| l.contains("Failed to authenticate") || l.contains("authentication failure"))
        .count() as u32
}
