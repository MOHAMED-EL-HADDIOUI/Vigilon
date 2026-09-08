# Vigilon — Local-First System & Security Observability Platform

> **Your machine. Your network. Your data. One dashboard.**

This document is the complete build specification for the project. It is written to be handed directly to a coding agent (human or AI, e.g. Claude Code) as the source of truth for architecture, scope, data model, detection logic, and build order.

---

## 0. Project Name

**Chosen name: `Vigilon`** (from *vigilant* + a product-style suffix).

Why it works:
- Short, pronounceable, unused-sounding as a single word (verify availability on crates.io, npm, and GitHub before publishing — name collisions are common in this space).
- Doesn't overpromise ("Sentinel", "Guardian", "Defender" imply active protection; this tool *observes and explains*, it doesn't block or defend).
- Works as a CLI binary name (`vigilon`), a service name (`vigilond`), and a brand.

**Alternates considered** (kept here in case `Vigilon` collides with an existing project/trademark):
- `Wardstack` — emphasizes the layered stack (agent → API → dashboard).
- `Hearthwatch` — "watching over your home machine," softer/more personal framing.

Avoid names that already belong to established tools/companies in this space: *Sentry*, *Lookout*, *Argus*, *Overwatch*, *Datadog*-style suffixes.

---

## 1. Positioning

**One-liner:** An open-source, local-first observability and security dashboard that shows you everything happening on your machine and network — hardware, processes, ports, connections — and explains *why* something looks unusual, without ever claiming certainty it can't back up.

**Not:** an antivirus, an EDR/XDR replacement, or an intrusion prevention system. It does not block traffic or kill processes automatically. It observes, correlates, and explains.

**Core differentiator vs. plain system monitors (htop, Glances, Netdata):** the combination of (1) historical, queryable state ("what changed since yesterday?"), (2) process↔port↔connection correlation, and (3) an explainable risk-scoring layer that never overstates confidence.

---

## 2. Design Principles (non-negotiable)

1. **Local-first.** All telemetry is stored on the user's machine by default. No cloud dependency, no phone-home, no external service required to use the product. Remote/cloud sync is an explicit, opt-in feature added later — never the default.
2. **Explain, don't accuse.** Every security-relevant finding is framed as "suspicious activity detected, here's why" with a confidence/risk level — never "you are under attack." Heuristics are heuristics, not verdicts.
3. **Low overhead by construction.** The agent must not become a significant load on the system it's monitoring. This is a design constraint enforced through architecture (event-driven detection over polling, native APIs over shelling out, tiered sampling), not an afterthought to optimize later.
4. **Event-driven over polling wherever the OS allows it.** Store state *changes*, not repeated snapshots of unchanging state.
5. **Privacy by default.** No telemetry leaves the machine unless the user explicitly configures an export/remote feature.
6. **Cross-platform, but not falsely uniform.** Shared trait/interface at the core; platform-specific implementations underneath. Don't force Linux/Windows/macOS into one code path when their OS primitives differ.

---

## 3. Feature Scope

### 3.1 Hardware Telemetry
- CPU: model, physical/logical cores, base/current frequency, per-core usage, temperature (where exposed)
- RAM: total, used, available, swap usage
- GPU: model, VRAM total/used, utilization, temperature, per-process GPU usage (NVIDIA first-class; AMD/Intel best-effort)
- Disk: capacity, usage per volume, read/write I/O throughput, SMART health where available
- Motherboard / BIOS / OS version and build info
- Battery status and health for laptops

### 3.2 Network Telemetry
- Interfaces: Ethernet, Wi-Fi, VPN, virtual/loopback adapters, per-interface up/down state
- Local IP, MAC, gateway, DNS servers
- Upload/download throughput (per interface, aggregate)
- Active TCP/UDP connections with local/remote address and port
- Listening ports mapped to owning process (PID, executable path)
- Remote IPs currently connected, with first-seen/last-seen timestamps
- DNS query activity where the OS/permissions allow capture
- Firewall status (enabled/disabled, active profile)

### 3.3 Security Signals
- New listening port opened
- Process communicating externally for the first time
- Unexpected/unusual outbound destination
- Repeated connection attempts to the same host/port
- Port-scan-like access pattern *against* this machine
- Suspicious changes to network configuration (DNS, gateway, interface changes)
- Firewall state or rule changes
- Authentication/login events, where the OS exposes them (e.g. Windows Security event log, Linux `auth.log`/`journald`, macOS unified log)
- Services/processes starting outside expected patterns (e.g. at unusual times, from unusual paths)

### 3.4 Explainability ("Why?")
Every notable event ships with:
- What was observed (process, ports, addresses, timestamps)
- Whether this is the first time this pattern was seen
- The specific indicators that contributed to the risk level (bulleted, not a black-box score)
- An explicit assessment level (see §7.3) — never a bare "attack" claim

---

## 4. Dashboard Layout (reference mockup)

```
┌──────────────────────────────────────────────────────────────┐
│  VIGILON                                ● SYSTEM PROTECTED    │
├──────────────────────────────────────────────────────────────┤
│ CPU        RAM         GPU        DISK        NETWORK         │
│ 34%        58%         41%        67%        ↓ 12.4 MB/s      │
├──────────────────────────────────────────────────────────────┤
│ NETWORK ACTIVITY                                               │
│ Local PC ─────── Router ─────── Internet                       │
│    ├─ :3000     ├─ :443                                        │
│    ├─ :5432     └─ :22                                         │
│    └─ :8080                                                    │
├────────────────────────────┬───────────────────────────────────┤
│ OPEN PORTS                 │ ACTIVE CONNECTIONS                │
│ 22   sshd      TCP         │ 192.168.1.20 → 8.8.8.8:443         │
│ 3000 node      TCP         │ 192.168.1.20 → github.com          │
│ 5432 postgres  TCP         │ 192.168.1.20 → api.example.com     │
│ 8080 python    TCP         │                                    │
├────────────────────────────┼───────────────────────────────────┤
│ SECURITY EVENTS            │ PROCESS MONITOR                    │
│ ⚠ New port 4567            │ firefox   12 connections           │
│ ⚠ New outbound connection  │ node       8 connections           │
│ ✓ Firewall active          │ python     4 connections           │
└────────────────────────────┴───────────────────────────────────┘
```

### "Why?" panel (opened per-event)
```
NEW CONNECTION
Process:            python3
Local:               192.168.1.15:51832
Remote:              185.xxx.xxx.xxx:443
First seen:          Today 17:42
Previous activity:   None

Assessment: ⚠ Suspicious
Why?
 • New process (not previously observed)
 • First external destination for this process
 • Connection established shortly after process start
```

### Timeline view
```
17:31:12  ✓ System started
17:31:15  ✓ Wi-Fi connected
17:32:08  ✓ Docker started
17:33:42  ℹ Port 3000 detected
17:34:01  ℹ Node process started
17:35:22  ⚠ New outbound connection
17:36:02  ⚠ Port 4567 opened
17:36:04  🔴 Unknown process started
17:36:10  ⚠ Multiple connections detected
```
Clicking any timeline entry opens all correlated data (process, ports, connections, related events) for that moment.

### Network topology graph
Rendered with React Flow; animates connections in near-real-time as sockets open/close.
```
                    INTERNET
                       │
               ┌───────┴───────┐
               │    Router     │
               └───────┬───────┘
                       │
                ┌──────▼──────┐
                │   MY PC     │
                │ 192.168.1.8 │
                └──────┬──────┘
          ┌────────────┼────────────┐
          ▼            ▼            ▼
       :22 ssh      :3000        :5432
          │            │             │
        sshd         node        postgres
                       │
                       ▼
                    Internet
```

---

## 5. System Architecture

### 5.1 Final recommended architecture (production target)

Build the core **entirely in Rust** — agent, collectors, detection engine, storage layer, and the API/WebSocket server in one process (or a small set of cooperating Rust processes). Do **not** put a Python service in the critical monitoring path for the shipped product.

```
┌──────────────────────────────────────┐
│         React + TypeScript           │
│       Dashboard / Visualization      │
└──────────────────┬───────────────────┘
                    │ REST + WebSocket
┌──────────────────▼───────────────────┐
│              Rust Core (Tokio)        │
│  ┌─────────────────────────────────┐  │
│  │ Collectors                      │  │
│  │  ├─ CPU / RAM / GPU / Disk      │  │
│  │  ├─ Processes                   │  │
│  │  ├─ Network interfaces          │  │
│  │  ├─ Connections / Ports         │  │
│  │  └─ Services / OS events        │  │
│  ├─────────────────────────────────┤  │
│  │ Detection Engine                │  │
│  │  ├─ Normalizer                  │  │
│  │  ├─ Rules Engine                │  │
│  │  ├─ Behavior/baseline analysis  │  │
│  │  └─ Risk scoring                │  │
│  ├─────────────────────────────────┤  │
│  │ API layer (Axum)                │  │
│  │  ├─ REST endpoints              │  │
│  │  └─ WebSocket event stream      │  │
│  ├─────────────────────────────────┤  │
│  │ Storage (SQLx + SQLite/WAL)     │  │
│  └─────────────────────────────────┘  │
└──────────────────┬───────────────────┘
                    │
              Operating System
      (native APIs — see §6.4 per platform)
```

### 5.2 Why one Rust process, not Rust-agent + Python-API

A Python/FastAPI layer between the collector and the dashboard is a fine **prototype** shortcut (fast to iterate, easy for a first working demo) but should not be the shipped architecture, because:
- It adds a process hop, serialization overhead, and a second runtime to package and keep alive.
- It reintroduces exactly the overhead the project is positioned against ("lightweight, low-footprint agent").
- Rust's Axum/Tokio stack can serve REST + WebSocket directly from the same process that owns the collectors, with zero IPC needed.

**Prototyping exception:** if the team wants a throwaway v0 to validate UI/UX before committing engineering time to native collectors, a Python/FastAPI + `psutil` prototype is acceptable *as a prototype only*, explicitly not carried into v0.1+.

### 5.3 Hybrid polling + event-driven pipeline

```
              SYSTEM
                 │
          ┌──────┴──────┐
          │             │
      Metrics        OS Events
      (sampled)      (change-detected)
          │             │
          ▼             ▼
     Periodic       Event-driven
     sampling       detection
          │             │
          └──────┬──────┘
                 ▼
             Event Bus
                 │
      ┌──────────┼───────────┐
      ▼          ▼           ▼
   SQLite    WebSocket     Rules Engine
                                │
                                ▼
                             Alerts
```

**Sampling tiers (defaults, all configurable):**
| Data | Strategy | Default interval |
|---|---|---|
| CPU / RAM / GPU / network throughput | Periodic sampling | 1–2 s |
| Processes | Periodic sampling | 5–10 s |
| Listening ports | Event-driven (diff socket table vs. previous state) | on change |
| Services | Periodic reconciliation + event-driven | ~30–60 s reconciliation |
| Security events | Event-driven only | immediate |

**Explicit anti-patterns to avoid:**
- Do not scan all 65,535 ports on an interval. Read the OS socket table (native syscalls) and diff against previous state; emit `PORT_OPENED` / `PORT_CLOSED` events only on change.
- Do not shell out to tools like `nvidia-smi` on a tight loop and parse text output. Use a native binding (see §6.4) and only fall back to shelling out as a last resort, at reduced frequency.
- Do not store a full metrics row every second for data that rarely changes (ports, services, interfaces) — store state transitions as events instead.

---

## 6. Technology Stack

### 6.1 Final stack

| Layer | Technology | Notes |
|---|---|---|
| Core agent + API | **Rust** (2021/2024 edition) + **Tokio** (async runtime) | single long-running process |
| HTTP/WebSocket server | **Axum** | REST + WS from the same Rust process |
| Serialization | **Serde** | shared types between collectors, storage, and API |
| Database access | **SQLx** | compile-time-checked queries, async |
| Storage | **SQLite in WAL mode** (v1) → optional PostgreSQL/TimescaleDB or ClickHouse later for large/multi-host deployments | local-first default |
| Structured logging | `tracing` + `tracing-subscriber` | needed for debugging a long-running background service |
| Frontend framework | **React + TypeScript**, built with **Vite** | |
| Styling | **Tailwind CSS** | |
| Charts | **Apache ECharts** (or Recharts for simpler cases) | CPU/RAM/GPU/disk/network timelines |
| Network topology graph | **React Flow** | animated live connection graph |
| Realtime transport | **WebSocket** | push-based event stream to the dashboard |
| Packaging | Cargo cross-compilation + platform-native installers; Docker image for containerized/server use | |
| CI | **GitHub Actions** | build matrix across Linux/Windows/macOS |
| Testing | Rust unit + integration tests; **Vitest** for frontend | |

### 6.2 Language comparison (why Rust for the core)

| Technology | Speed | Memory footprint | System access | Best use here |
|---|---|---|---|---|
| Rust | Excellent | Excellent | Excellent (native OS bindings, safe FFI) | Core agent + API |
| C++ | Excellent | Very good | Excellent | Viable alternative, but weaker memory-safety guarantees for a security-adjacent tool |
| Go | Very good | Very good | Good | Reasonable fallback if the team knows Go better than Rust |
| Python | Poor–OK | Poor | OK via `psutil`/shelling out | Prototyping only |
| Node.js/TypeScript | OK | OK | Weak for low-level system access | Frontend, not core |

**Ranking for the core agent: Rust > C++ > Go > Python.** Rust is preferred over C++ specifically because this is a security-adjacent tool parsing OS/network state continuously — memory-safety bugs here are worse than in an average application, and Rust's ownership model substantially reduces that class of bug without sacrificing performance.

### 6.3 Suggested crates (starting point, verify current versions before pinning)
- `sysinfo` — cross-platform CPU/RAM/disk/process baseline info
- `nvml-wrapper` — NVIDIA GPU telemetry (utilization, VRAM, temperature, per-process GPU usage) via NVML, avoiding `nvidia-smi` shelling
- `windows-rs` — direct Windows API access (services, WMI, event log) where `sysinfo` isn't enough
- `netlink`-family crates (Linux) — reading the kernel's socket/route tables directly instead of parsing `netstat`/`ss` output
- `pnet` / `pcap` (used sparingly, permission-gated) — for deeper packet-level visibility where OS-level tables aren't sufficient
- `axum`, `tokio`, `tokio-tungstenite` (or Axum's built-in WS support) — API/WebSocket layer
- `sqlx` — async, compile-time-checked SQL
- `serde`, `serde_json` — shared wire/storage types
- `tracing` — structured logs for a background service

### 6.4 Platform-specific data sources
| Platform | Primary sources |
|---|---|
| Linux | `/proc`, `/sys`, netlink sockets, `journald`/`auth.log` for auth events, `nftables`/`iptables` state for firewall |
| Windows | WMI, Performance Counters, Windows Event Log (Security/System channels), `netsh`/firewall API, Windows Service Control Manager |
| macOS | IOKit, `libproc`, unified logging (`log` subsystem), `pfctl` state for firewall |

Structure the codebase so each platform implements a shared Rust trait; do not force identical internals across platforms:
```
core/
  metrics.rs
  events.rs
  storage.rs
  detection.rs
platform/
  linux/
  windows/
  macos/
```

---

## 7. Threat Detection Engine

### 7.1 Pipeline
```
Collectors → Normalizer → Rules Engine → Behavior Analysis → Risk Score → Alert
```
This is explicitly **not** an antivirus and does not claim signature-based malware detection. It is a behavioral/heuristic layer over locally observed state.

### 7.2 Example rules
- `NEW_LISTENING_PORT`
- `NEW_EXTERNAL_CONNECTION`
- `PORT_STATE_CHANGED`
- `UNKNOWN_PROCESS_NETWORK_ACCESS`
- `MULTIPLE_FAILED_LOGINS`
- `UNUSUAL_CONNECTION_RATE`
- `NEW_SERVICE_STARTED`
- `FIREWALL_CONFIGURATION_CHANGED`

### 7.3 Risk levels
`INFO → LOW → MEDIUM → HIGH → CRITICAL`

Each level maps to language, never to a certainty claim:
- `INFO`: normal, logged for the timeline.
- `LOW`/`MEDIUM`: "new/unusual, worth a look."
- `HIGH`/`CRITICAL`: "multiple independent indicators align — investigate promptly." Still phrased as "suspicious activity detected," never "you are under attack."

### 7.4 Example rendered alert
```
HIGH
Process:        unknown_process
Destination:    91.xxx.xxx.xxx:443
First observed: 17:48

Indicators:
+ New process
+ New destination
+ Started outside normal pattern
+ Persistent connection

Action: Investigate process
```

### 7.5 Sample event JSON (WebSocket payload)
```json
{
  "type": "SECURITY_EVENT",
  "rule": "NEW_EXTERNAL_CONNECTION",
  "risk": "MEDIUM",
  "timestamp": "2026-09-06T17:48:03Z",
  "process": { "name": "python3", "pid": 18420, "path": "/usr/bin/python3" },
  "local": "192.168.1.15:51832",
  "remote": "185.xxx.xxx.xxx:443",
  "indicators": [
    "new_process",
    "first_external_destination",
    "connection_shortly_after_process_start"
  ]
}
```

---

## 8. Data Model (SQLite, WAL mode)

Core tables:
```
devices
system_snapshots
cpu_metrics
memory_metrics
gpu_metrics
disk_metrics
network_interfaces
connections
ports
processes
services
security_events
alerts
```

Example — `system_metrics`-style periodic table:
```
timestamp
cpu_usage
ram_usage
gpu_usage
gpu_memory
disk_read
disk_write
network_rx
network_tx
```

Example — event-driven `ports` history (store transitions, not per-second snapshots):
```
14:30:12  PORT_OPENED   3000
15:02:45  PORT_CLOSED   3000
15:14:02  PORT_OPENED   8080
```

**Queries this data model should make trivial:**
- What was using my GPU yesterday?
- Which process opened port 8080, and when?
- Which external IPs connected to this machine in the last 24 hours?
- When did this service first appear on the system?
- What changed between yesterday and today?

**Scaling path:** SQLite (v1, single machine) → PostgreSQL → TimescaleDB/ClickHouse (only if/when multi-host or long-retention deployments are needed). Don't build this until there's a real need for it.

---

## 9. Repository Structure

```
vigilon/
│
├── core/                 # shared Rust types, traits, event bus
│   ├── metrics.rs
│   ├── events.rs
│   ├── storage.rs
│   └── detection.rs
│
├── platform/
│   ├── linux/
│   ├── windows/
│   └── macos/
│
├── agent/
│   ├── hardware/
│   ├── processes/
│   ├── network/
│   ├── ports/
│   ├── services/
│   ├── security/
│   └── collectors/
│
├── api/                  # Axum REST + WebSocket server
│
├── frontend/
│   ├── dashboard/
│   ├── network-map/
│   ├── processes/
│   ├── ports/
│   ├── security/
│   ├── timeline/
│   └── settings/
│
├── shared/               # wire types shared between Rust and TS (via codegen or hand-kept parity)
├── docs/
├── tests/
├── docker/
└── README.md
```

---

## 10. Non-Goals (state these explicitly in the README to set expectations)

- Not an antivirus or malware scanner.
- Not an EDR/XDR replacement and not a compliance tool.
- Does not automatically block, kill, or quarantine anything — it observes and explains.
- Does not claim certainty about intent; all security output is heuristic and explicitly labeled as such.
- Cloud/remote monitoring is optional and off by default, not a core requirement of v1.

---

## 11. MVP & Roadmap

**MVP (first legitimate release) — do not build everything at once:**
1. CPU, RAM, GPU, Disk metrics
2. Processes
3. Network interfaces
4. Open ports
5. Process → port mapping
6. Active connections
7. Historical metrics (queryable)
8. Basic security events (no full rules engine yet)
9. Web dashboard (metrics + ports + connections views)

**Release sequence:**
| Version | Focus |
|---|---|
| v0.1 | System monitor (CPU/RAM/GPU/Disk, processes) |
| v0.2 | Network monitor (interfaces, connections, throughput) |
| v0.3 | Port & service intelligence (process↔port mapping, service reconciliation) |
| v0.4 | Security detection engine (rules, risk scoring, "Why?" panel) |
| v0.5 | Network visualization (React Flow live topology graph) |
| v1.0 | Full dashboard: timeline, all of the above integrated, packaged installers |

**Later (explicitly post-v1, do not scope into v1):**
- Optional AI security analyst summarizing historical data — kept out of the real-time detection path so an LLM is never a dependency for core monitoring.
- Optional remote/cloud sync for multi-machine fleets.
- PostgreSQL/TimescaleDB/ClickHouse backend for larger deployments.

---

## 12. Recommended Agent Roles / Skills for Building This Project

If building this with an agentic coding workflow (e.g. multiple specialized sub-agents or skill files), split responsibilities like this:

1. **Systems/Rust engineer** — collectors, native OS API integration (`sysinfo`, `nvml-wrapper`, `windows-rs`, netlink), async runtime design with Tokio.
2. **Network engineer** — socket table diffing, process↔port↔connection correlation, firewall state reads, DNS activity capture where permitted.
3. **Security detection engineer** — rules engine design, baseline/behavior analysis, risk-scoring taxonomy, and — critically — the *language* of alerts (explain, don't accuse).
4. **Backend/API engineer** — Axum REST + WebSocket server, SQLx schema/migrations, event bus design.
5. **Frontend engineer** — React/TypeScript dashboard, ECharts time-series views, React Flow live topology graph, Tailwind styling.
6. **DevOps/packaging engineer** — Cargo cross-compilation for Linux/Windows/macOS, Docker image, GitHub Actions CI matrix, installers.
7. **Technical writer** — README, architecture decision records, threat-model documentation, and the non-goals/disclaimer language (important for credibility, see §10).

Skills/expertise areas that matter most, in priority order: **Rust async systems programming → native OS/security API knowledge per platform → embedded database design (SQLite/WAL, event-sourced schema) → real-time web (WebSocket) → data visualization (time-series + graph) → security heuristics/UX framing for risk communication.**

---

## 13. Build Order (for the implementing agent)

Build one full **vertical slice** before expanding breadth — don't build all collectors before anything is wired end-to-end.

1. Scaffold a Cargo workspace: `core`, `platform/linux` (start with one OS), `agent`, `api`.
2. Define shared `serde`-friendly types in `core` for metrics and events.
3. Implement one collector end-to-end: CPU + RAM via `sysinfo`.
4. Set up SQLite + `sqlx` migrations; write CPU/RAM samples to `cpu_metrics`/`memory_metrics`.
5. Stand up the Axum server: one REST endpoint (`GET /metrics/latest`) and one WebSocket stream pushing live samples.
6. Scaffold the React/Vite/Tailwind frontend; render the CPU/RAM tiles from the live WebSocket feed. **This is the first working demo.**
7. Expand collectors: GPU, Disk, Processes.
8. Add network collector: interfaces, connections, listening ports, process↔port mapping. Switch ports/services to event-driven diffing at this point, not before.
9. Add the security normalizer + rules engine (§7); start with 2–3 rules (`NEW_LISTENING_PORT`, `NEW_EXTERNAL_CONNECTION`), then expand.
10. Build the "Why?" explainability panel and the timeline view.
11. Add the React Flow live network topology graph.
12. Port collectors to Windows, then macOS, behind the shared platform trait.
13. Packaging: Cargo cross-compiles, Docker image, installers; set up GitHub Actions CI.

---

## 14. License & Governance (suggested)

- Dual-license **MIT / Apache-2.0** — the de facto standard for Rust open-source projects, and permissive enough to encourage adoption and contribution.
- Publish a clear **threat model** and **non-goals** doc alongside the README (see §10) — this is what makes the security framing credible rather than snake-oil.
