use vigilon_core::metrics::FirewallStatus;

pub fn status() -> Option<FirewallStatus> {
    #[cfg(windows)]
    {
        windows_fw()
    }
    #[cfg(target_os = "linux")]
    {
        linux_fw()
    }
    #[cfg(target_os = "macos")]
    {
        macos_fw()
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        None
    }
}

#[cfg(windows)]
fn windows_fw() -> Option<FirewallStatus> {
    use windows::Win32::NetworkManagement::WindowsFirewall::{
        INetFwPolicy2, NET_FW_PROFILE2_DOMAIN, NET_FW_PROFILE2_PRIVATE, NET_FW_PROFILE2_PUBLIC,
        NetFwPolicy2,
    };
    use windows::Win32::System::Com::{
        CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
    };

    // COM initializes once per process; the firewall check runs every
    // metrics tick, so a bare CoInitializeEx here would churn the
    // apartment refcount forever with no matching Uninitialize.
    static COM_INIT: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    COM_INIT.get_or_init(|| unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    });

    unsafe {
        let policy: INetFwPolicy2 = CoCreateInstance(&NetFwPolicy2, None, CLSCTX_ALL).ok()?;
        let domain = policy
            .get_FirewallEnabled(NET_FW_PROFILE2_DOMAIN)
            .ok()?
            .as_bool();
        let private = policy
            .get_FirewallEnabled(NET_FW_PROFILE2_PRIVATE)
            .ok()?
            .as_bool();
        let public = policy
            .get_FirewallEnabled(NET_FW_PROFILE2_PUBLIC)
            .ok()?
            .as_bool();
        let enabled = domain || private || public;
        let profile = if public {
            "public"
        } else if private {
            "private"
        } else if domain {
            "domain"
        } else {
            "disabled"
        };
        Some(FirewallStatus {
            enabled,
            profile: Some(profile.into()),
            details: format!("domain={domain} private={private} public={public}"),
        })
    }
}

#[cfg(target_os = "linux")]
fn linux_fw() -> Option<FirewallStatus> {
    let nft = std::path::Path::new("/usr/sbin/nft").exists()
        || std::path::Path::new("/sys/module/nf_tables").exists();
    let ipt = std::path::Path::new("/proc/net/ip_tables_names").exists();
    let ufw = std::fs::read_to_string("/etc/ufw/ufw.conf")
        .ok()
        .map(|s| s.contains("ENABLED=yes"))
        .unwrap_or(false);
    Some(FirewallStatus {
        enabled: nft || ipt || ufw,
        profile: Some(if ufw {
            "ufw".into()
        } else if nft {
            "nftables".into()
        } else {
            "iptables".into()
        }),
        details: format!("nft={nft} iptables={ipt} ufw={ufw}"),
    })
}

#[cfg(target_os = "macos")]
fn macos_fw() -> Option<FirewallStatus> {
    Some(FirewallStatus {
        enabled: std::path::Path::new("/etc/pf.conf").exists(),
        profile: Some("pf".into()),
        details: "pf.conf present (state not queried)".into(),
    })
}
