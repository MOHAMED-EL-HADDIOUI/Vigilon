use std::collections::HashMap;
use std::time::Instant;

use netstat2::{AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, get_sockets_info};
use sysinfo::Networks;
use vigilon_core::metrics::{Connection, ListeningPort, NetworkInterface};

use crate::processes::ProcessCollector;

pub struct NetCollector {
    networks: Networks,
    last: HashMap<String, (u64, u64, Instant)>,
}

impl NetCollector {
    pub fn new() -> Self {
        Self {
            networks: Networks::new_with_refreshed_list(),
            last: HashMap::new(),
        }
    }

    pub fn interfaces(&mut self) -> Vec<NetworkInterface> {
        self.networks.refresh(true);
        let now = Instant::now();
        let (dns, gateway) = dns_and_gateway();
        let mut out = Vec::new();
        for (name, data) in self.networks.iter() {
            let rx = data.total_received();
            let tx = data.total_transmitted();
            let (rx_bps, tx_bps) = if let Some((pr, pt, t)) = self.last.get(name) {
                let dt = now.duration_since(*t).as_secs_f64().max(0.001);
                (
                    (rx.saturating_sub(*pr)) as f64 / dt,
                    (tx.saturating_sub(*pt)) as f64 / dt,
                )
            } else {
                (0.0, 0.0)
            };
            self.last.insert(name.clone(), (rx, tx, now));
            let kind = classify_iface(name);
            let ips: Vec<String> = data.ip_networks().iter().map(|n| n.to_string()).collect();
            let is_up = rx > 0 || tx > 0 || !ips.is_empty();
            out.push(NetworkInterface {
                name: name.clone(),
                kind,
                mac: Some(data.mac_address().to_string()),
                ips,
                is_up,
                rx_bps,
                tx_bps,
                rx_total_bytes: rx,
                tx_total_bytes: tx,
                gateway: gateway.clone(),
                dns: dns.clone(),
            });
        }
        out
    }

    pub fn sockets(&self, procs: &mut ProcessCollector) -> (Vec<ListeningPort>, Vec<Connection>) {
        procs.refresh();
        let af = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
        let proto = ProtocolFlags::TCP | ProtocolFlags::UDP;
        let Ok(sockets) = get_sockets_info(af, proto) else {
            return (Vec::new(), Vec::new());
        };
        let mut ports = Vec::new();
        let mut conns = Vec::new();
        for s in sockets {
            let pid = s.associated_pids.first().copied();
            let (name, path) = procs.lookup(pid);
            match s.protocol_socket_info {
                ProtocolSocketInfo::Tcp(t) => {
                    let state = format!("{:?}", t.state);
                    let listening = state.to_uppercase().contains("LISTEN");
                    if listening {
                        ports.push(ListeningPort {
                            protocol: "TCP".into(),
                            address: t.local_addr.to_string(),
                            port: t.local_port,
                            pid,
                            process_name: name.clone(),
                            process_path: path.clone(),
                        });
                    } else {
                        conns.push(Connection {
                            protocol: "TCP".into(),
                            local_addr: t.local_addr.to_string(),
                            local_port: t.local_port,
                            remote_addr: t.remote_addr.to_string(),
                            remote_port: t.remote_port,
                            state,
                            pid,
                            process_name: name,
                            process_path: path,
                            first_seen: None,
                            last_seen: None,
                            geo: None,
                        });
                    }
                }
                ProtocolSocketInfo::Udp(u) => {
                    ports.push(ListeningPort {
                        protocol: "UDP".into(),
                        address: u.local_addr.to_string(),
                        port: u.local_port,
                        pid,
                        process_name: name,
                        process_path: path,
                    });
                }
            }
        }
        (ports, conns)
    }
}

fn classify_iface(name: &str) -> String {
    let n = name.to_lowercase();
    if n.contains("loopback") || n == "lo" || n.starts_with("lo") {
        "loopback".into()
    } else if n.contains("wi-fi")
        || n.contains("wifi")
        || n.contains("wlan")
        || n.contains("airport")
    {
        "wifi".into()
    } else if n.contains("vpn") || n.contains("tun") || n.contains("tap") || n.contains("wg") {
        "vpn".into()
    } else if n.contains("vethernet")
        || n.contains("docker")
        || n.contains("veth")
        || n.contains("br-")
    {
        "virtual".into()
    } else if n.contains("ethernet") || n.contains("eth") || n.contains("en") {
        "ethernet".into()
    } else {
        "other".into()
    }
}

pub fn dns_and_gateway() -> (Vec<String>, Option<String>) {
    #[cfg(windows)]
    {
        windows_dns_gateway()
    }
    #[cfg(target_os = "linux")]
    {
        linux_dns_gateway()
    }
    #[cfg(target_os = "macos")]
    {
        macos_dns_gateway()
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        (Vec::new(), None)
    }
}

#[cfg(windows)]
fn windows_dns_gateway() -> (Vec<String>, Option<String>) {
    use windows::Win32::NetworkManagement::IpHelper::{
        GAA_FLAG_INCLUDE_GATEWAYS, GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH,
    };
    use windows::Win32::Networking::WinSock::{AF_UNSPEC, SOCKADDR_IN};

    let mut size: u32 = 0;
    unsafe {
        let _ = GetAdaptersAddresses(
            AF_UNSPEC.0 as u32,
            GAA_FLAG_INCLUDE_GATEWAYS,
            None,
            None,
            &mut size,
        );
    }
    if size == 0 {
        return (Vec::new(), None);
    }
    let mut buf = vec![0u8; size as usize];
    let head = buf.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH;
    let ok = unsafe {
        GetAdaptersAddresses(
            AF_UNSPEC.0 as u32,
            GAA_FLAG_INCLUDE_GATEWAYS,
            None,
            Some(head),
            &mut size,
        )
    };
    if ok != 0 {
        return (Vec::new(), None);
    }
    let mut dns = Vec::new();
    let mut gateway = None;
    let mut cur = head;
    unsafe {
        while !cur.is_null() {
            let adapter = &*cur;
            let mut d = adapter.FirstDnsServerAddress;
            while !d.is_null() {
                if let Some(ip) = sockaddr_ip((*d).Address.lpSockaddr as *const SOCKADDR_IN)
                    && !dns.contains(&ip)
                {
                    dns.push(ip);
                }
                d = (*d).Next;
            }
            if gateway.is_none() {
                let mut g = adapter.FirstGatewayAddress;
                while !g.is_null() {
                    if let Some(ip) = sockaddr_ip((*g).Address.lpSockaddr as *const SOCKADDR_IN) {
                        gateway = Some(ip);
                        break;
                    }
                    g = (*g).Next;
                }
            }
            cur = adapter.Next;
        }
    }
    (dns, gateway)
}

#[cfg(windows)]
unsafe fn sockaddr_ip(
    sa: *const windows::Win32::Networking::WinSock::SOCKADDR_IN,
) -> Option<String> {
    use windows::Win32::Networking::WinSock::{AF_INET, AF_INET6, SOCKADDR_IN6};
    if sa.is_null() {
        return None;
    }
    let family = unsafe { (*sa).sin_family };
    if family == AF_INET {
        let oct = unsafe { (*sa).sin_addr.S_un.S_un_b };
        return Some(format!(
            "{}.{}.{}.{}",
            oct.s_b1, oct.s_b2, oct.s_b3, oct.s_b4
        ));
    }
    if family == AF_INET6 {
        let sa6 = sa as *const SOCKADDR_IN6;
        let bytes = unsafe { (*sa6).sin6_addr.u.Byte };
        let addr = std::net::Ipv6Addr::from(bytes);
        return Some(addr.to_string());
    }
    None
}

#[cfg(target_os = "linux")]
fn linux_dns_gateway() -> (Vec<String>, Option<String>) {
    let dns = std::fs::read_to_string("/etc/resolv.conf")
        .ok()
        .map(|s| {
            s.lines()
                .filter_map(|l| l.strip_prefix("nameserver "))
                .map(|v| v.trim().to_string())
                .collect()
        })
        .unwrap_or_default();
    let gateway = std::fs::read_to_string("/proc/net/route")
        .ok()
        .and_then(|s| {
            for line in s.lines().skip(1) {
                let cols: Vec<&str> = line.split_whitespace().collect();
                if cols.len() >= 3 && cols[1] == "00000000" {
                    let gw = u32::from_str_radix(cols[2], 16).ok()?;
                    return Some(format!(
                        "{}.{}.{}.{}",
                        gw & 0xff,
                        (gw >> 8) & 0xff,
                        (gw >> 16) & 0xff,
                        (gw >> 24) & 0xff
                    ));
                }
            }
            None
        });
    (dns, gateway)
}

#[cfg(target_os = "macos")]
fn macos_dns_gateway() -> (Vec<String>, Option<String>) {
    let dns = std::fs::read_to_string("/etc/resolv.conf")
        .ok()
        .map(|s| {
            s.lines()
                .filter_map(|l| l.strip_prefix("nameserver "))
                .map(|v| v.trim().to_string())
                .collect()
        })
        .unwrap_or_default();
    (dns, None)
}
