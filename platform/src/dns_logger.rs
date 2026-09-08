//! DNS query logging surface.
//!
//! The live capture primitives live in [`crate::dns`]; this module is the
//! v2 task-tracker home for DNS logging — it re-exports the snapshot helpers
//! and adds the batching helper used by the runtime before persisting.

pub use crate::dns::{cache_snapshot, query_snapshot};

/// Cap a DNS batch before inserting into storage / publishing on the bus.
pub fn cap_batch(
    mut queries: Vec<vigilon_core::metrics::DnsQuery>,
    limit: usize,
) -> Vec<vigilon_core::metrics::DnsQuery> {
    queries.truncate(limit.max(1));
    queries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cap_batch_truncates() {
        let now = chrono::Utc::now();
        let mk = |name: &str| vigilon_core::metrics::DnsQuery {
            timestamp: now,
            query_name: name.into(),
            record_type: "A".into(),
            response_code: 0,
            process_name: None,
        };
        let out = cap_batch(vec![mk("a"), mk("b"), mk("c")], 2);
        assert_eq!(out.len(), 2);
    }
}
