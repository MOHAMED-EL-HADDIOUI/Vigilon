use maxminddb::geoip2;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use vigilon_core::metrics::GeoLocation;

#[derive(Clone)]
pub struct GeoIpResolver {
    reader: Option<Arc<maxminddb::Reader<Vec<u8>>>>,
}

impl GeoIpResolver {
    pub fn new(data_dir: &Path) -> Self {
        let candidates = [
            data_dir.join("GeoLite2-City.mmdb"),
            data_dir.join("GeoLite2-Country.mmdb"),
            PathBuf::from("GeoLite2-City.mmdb"),
        ];

        let mut reader = None;
        for c in &candidates {
            if c.exists()
                && let Ok(r) = maxminddb::Reader::open_readfile(c)
            {
                tracing::info!(path = %c.display(), "loaded GeoIP database");
                reader = Some(Arc::new(r));
                break;
            }
        }

        Self { reader }
    }

    pub fn resolve(&self, ip_str: &str) -> Option<GeoLocation> {
        let clean = vigilon_core::netparse::strip_port(ip_str);
        let ip: IpAddr = clean.parse().ok()?;

        if is_private_or_local(&ip) {
            return None;
        }

        let reader = self.reader.as_ref()?;
        let city: geoip2::City = reader.lookup(ip).ok()?;

        let country_code = city
            .country
            .as_ref()
            .and_then(|c| c.iso_code)
            .unwrap_or("??")
            .to_string();

        let country_name = city
            .country
            .as_ref()
            .and_then(|c| c.names.as_ref())
            .and_then(|n| n.get("en"))
            .copied()
            .unwrap_or("Unknown")
            .to_string();

        let city_name = city
            .city
            .as_ref()
            .and_then(|c| c.names.as_ref())
            .and_then(|n| n.get("en"))
            .copied()
            .map(|s| s.to_string());

        Some(GeoLocation {
            country_code,
            country_name,
            city: city_name,
        })
    }
}

fn is_private_or_local(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            v4.is_loopback()
                || v4.is_private()
                || v4.is_multicast()
                || v4.is_broadcast()
                || (o[0] == 169 && o[1] == 254)
        }
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_multicast(),
    }
}
