import { useEffect, useReducer } from "react";
import type {
  Connection,
  ListeningPort,
  NetworkInterface,
  ProcessInfo,
  SecurityEvent,
  ServiceInfo,
  Snapshot,
  TimelineEntry,
  WsEvent,
} from "./types";
import { getJson, wsUrl } from "./lib";

export interface Live {
  snapshot: Snapshot | null;
  ports: ListeningPort[];
  connections: Connection[];
  processes: ProcessInfo[];
  interfaces: NetworkInterface[];
  services: ServiceInfo[];
  events: SecurityEvent[];
  timeline: TimelineEntry[];
  connected: boolean;
  /** True once the initial REST hydration completed (success or fail). */
  hydrated: boolean;
  /** True when every hydration request failed — likely daemon down. */
  backendDown: boolean;
}

const empty: Live = {
  snapshot: null,
  ports: [],
  connections: [],
  processes: [],
  interfaces: [],
  services: [],
  events: [],
  timeline: [],
  connected: false,
  hydrated: false,
  backendDown: false,
};

type Action =
  | { type: "ws"; connected: boolean }
  | { type: "msg"; event: WsEvent }
  | { type: "hydrate"; live: Partial<Live> };

function reducer(state: Live, action: Action): Live {
  if (action.type === "ws") return { ...state, connected: action.connected };
  if (action.type === "hydrate") return { ...state, ...action.live };
  const ev = action.event;
  switch (ev.type) {
    case "METRICS":
      return { ...state, snapshot: ev };
    case "PORTS":
      return { ...state, ports: ev.items ?? state.ports };
    case "CONNECTIONS":
      return { ...state, connections: ev.items ?? state.connections };
    case "PROCESSES":
      return { ...state, processes: ev.items ?? state.processes };
    case "INTERFACES":
      return { ...state, interfaces: ev.items ?? state.interfaces };
    case "SERVICES":
      return { ...state, services: ev.items ?? state.services };
    case "SECURITY_EVENT":
      return { ...state, events: [ev, ...state.events].slice(0, 200) };
    case "TIMELINE":
      return { ...state, timeline: [ev, ...state.timeline].slice(0, 500) };
    case "DNS_CACHE":
      return state;
    case "PROCESS_BANDWIDTH":
      return state;
    case "DNS_QUERIES":
      return state;
    default:
      return state;
  }
}

export function useLive(): Live {
  const [state, dispatch] = useReducer(reducer, empty);

  useEffect(() => {
    void Promise.allSettled([
      getJson<Snapshot | null>("/metrics/latest"),
      getJson<ListeningPort[]>("/ports"),
      getJson<Connection[]>("/connections"),
      getJson<ProcessInfo[]>("/processes"),
      getJson<NetworkInterface[]>("/interfaces"),
      getJson<SecurityEvent[]>("/events"),
      getJson<TimelineEntry[]>("/timeline"),
      getJson<ServiceInfo[]>("/services"),
    ]).then((results) => {
      const val = <T,>(i: number): T | undefined => {
        const r = results[i];
        return r.status === "fulfilled" ? (r.value as T) : undefined;
      };
      const failed = results.filter((r) => r.status === "rejected").length;
      dispatch({
        type: "hydrate",
        live: {
          snapshot: val<Snapshot | null>(0) ?? null,
          ports: val(1) ?? [],
          connections: val(2) ?? [],
          processes: val(3) ?? [],
          interfaces: val(4) ?? [],
          events: val(5) ?? [],
          timeline: val(6) ?? [],
          services: val(7) ?? [],
          hydrated: true,
          backendDown: failed === results.length,
        },
      });
    });
  }, []);

  useEffect(() => {
    let ws: WebSocket | undefined;
    let closed = false;
    let retry: number | undefined;

    const connect = () => {
      ws = new WebSocket(wsUrl());
      ws.onopen = () => dispatch({ type: "ws", connected: true });
      ws.onclose = () => {
        dispatch({ type: "ws", connected: false });
        if (!closed) retry = window.setTimeout(connect, 1500);
      };
      ws.onmessage = (ev) => {
        try {
          dispatch({ type: "msg", event: JSON.parse(ev.data) as WsEvent });
        } catch {
          /* ignore malformed frames */
        }
      };
    };
    connect();
    return () => {
      closed = true;
      if (retry) window.clearTimeout(retry);
      ws?.close();
    };
  }, []);

  return state;
}
