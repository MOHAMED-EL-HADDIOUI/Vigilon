use vigilon_core::metrics::DnsCacheEntry;

pub fn cache_snapshot() -> Vec<DnsCacheEntry> {
    #[cfg(windows)]
    {
        windows_dns_cache()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

#[cfg(windows)]
fn windows_dns_cache() -> Vec<DnsCacheEntry> {
    #[repr(C)]
    struct RawEntry {
        next: *mut RawEntry,
        name: *const u16,
        ty: u16,
        data_length: u16,
        flags: u32,
    }

    #[link(name = "dnsapi")]
    unsafe extern "system" {
        fn DnsGetCacheDataTable(table: *mut *mut RawEntry) -> i32;
        fn DnsFree(p: *mut std::ffi::c_void, free_type: u32);
    }

    unsafe {
        let mut head: *mut RawEntry = std::ptr::null_mut();
        if DnsGetCacheDataTable(&mut head) == 0 || head.is_null() {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut cur = head;
        // Bound both the list walk and each name scan: a non-terminated
        // name pointer must not drive an unbounded over-read.
        const MAX_NAME_UTF16: usize = 256;
        while !cur.is_null() && out.len() < 80 {
            let e = &*cur;
            if !e.name.is_null() {
                let mut len = 0;
                while len < MAX_NAME_UTF16 && *e.name.add(len) != 0 {
                    len += 1;
                }
                let slice = std::slice::from_raw_parts(e.name, len);
                let name = String::from_utf16_lossy(slice);
                out.push(DnsCacheEntry {
                    name,
                    record_type: format!("{}", e.ty),
                });
            }
            cur = e.next;
        }
        DnsFree(head as *mut _, 1);
        out
    }
}

pub fn query_snapshot() -> Vec<vigilon_core::metrics::DnsQuery> {
    let cache = cache_snapshot();
    let now = chrono::Utc::now();
    cache
        .into_iter()
        .map(|c| {
            let ty_num = c.record_type.parse::<u16>().unwrap_or(0);
            let ty_label = match ty_num {
                1 => "A",
                28 => "AAAA",
                5 => "CNAME",
                12 => "PTR",
                15 => "MX",
                16 => "TXT",
                33 => "SRV",
                _ => "ANY",
            };
            vigilon_core::metrics::DnsQuery {
                timestamp: now,
                query_name: c.name,
                record_type: ty_label.into(),
                response_code: 0,
                process_name: None,
            }
        })
        .collect()
}
