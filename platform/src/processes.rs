use chrono::{TimeZone, Utc};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};
use vigilon_core::metrics::{Connection, ProcessInfo};

/// Single cap for the process pipeline: the collector truncates here and
/// storage inserts up to the same bound (see `insert_processes`).
pub const MAX_PROCESSES: usize = 250;

pub struct ProcessCollector {
    sys: System,
}

impl ProcessCollector {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_specifics(
            RefreshKind::nothing().with_processes(ProcessRefreshKind::everything()),
        );
        Self { sys }
    }

    pub fn refresh(&mut self) {
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::everything(),
        );
    }

    pub fn lookup(&self, pid: Option<u32>) -> (Option<String>, Option<String>) {
        let Some(pid) = pid else {
            return (None, None);
        };
        let Some(p) = self.sys.process(Pid::from_u32(pid)) else {
            return (None, None);
        };
        (
            Some(p.name().to_string_lossy().into_owned()),
            p.exe().map(|e| e.to_string_lossy().into_owned()),
        )
    }

    pub fn list(&mut self) -> Vec<ProcessInfo> {
        self.refresh();
        let mut out: Vec<ProcessInfo> = self
            .sys
            .processes()
            .iter()
            .map(|(pid, p)| ProcessInfo {
                pid: pid.as_u32(),
                name: p.name().to_string_lossy().into_owned(),
                path: p.exe().map(|e| e.to_string_lossy().into_owned()),
                cpu_percent: p.cpu_usage(),
                memory_bytes: p.memory(),
                status: format!("{:?}", p.status()),
                start_time: Utc.timestamp_opt(p.start_time() as i64, 0).single(),
                connection_count: 0,
                parent_pid: p.parent().map(|x| x.as_u32()),
            })
            .collect();
        out.sort_by(|a, b| {
            b.cpu_percent
                .partial_cmp(&a.cpu_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        out.truncate(MAX_PROCESSES);
        out
    }
}

pub fn attach_counts(processes: &mut [ProcessInfo], conns: &[Connection]) {
    for p in processes.iter_mut() {
        p.connection_count = conns.iter().filter(|c| c.pid == Some(p.pid)).count() as u32;
    }
}
