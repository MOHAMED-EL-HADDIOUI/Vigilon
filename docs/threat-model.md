# Threat model

Vigilon is a **local observer**. It reads OS-exposed process, socket, hardware, firewall, and (where permitted) authentication signals, stores them on disk, and scores *changes* with documented heuristics.

## In scope

- Honest reporting of local state the user can already see with OS tools
- Explaining *why* a finding fired (indicators, first-seen, process path)
- Keeping data on the host (SQLite WAL; no default network egress except user-initiated dashboard use on localhost)

## Out of scope

- Stopping malware, ransomware, or exploits
- Signature databases or cloud reputation
- Guaranteeing that absence of alerts means the host is safe
- Multi-tenant or untrusted multi-user isolation of the SQLite file

## Trust boundary

Whoever can read `VIGILON_DATA` can read historical connections and process names. Bind defaults to `127.0.0.1`. Do not expose the API to the network without an access-control layer (not shipped in v1).

## Heuristic honesty

Rules such as `NEW_EXTERNAL_CONNECTION` fire on first-seen destinations. That is unusual relative to *this* host’s baseline, not proof of malice. UI copy must stay in “suspicious activity detected” language.
