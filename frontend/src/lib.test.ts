import { describe, expect, it } from "vitest";
import { formatBps, formatBytes, indicatorText, riskColor, riskLabel, timeAgo } from "./lib";

describe("formatters", () => {
  it("formats throughput", () => {
    expect(formatBps(500)).toContain("B/s");
    expect(formatBps(2048)).toContain("KB/s");
    expect(formatBps(1024 * 1024 * 5)).toContain("MB/s");
  });
  it("formats bytes", () => {
    expect(formatBytes(2048)).toContain("KB");
    expect(formatBytes(1024 * 1024 * 50)).toContain("MB");
    expect(formatBytes(1024 ** 3 * 2)).toContain("GB");
  });
  it("never labels risk as attack", () => {
    expect(riskLabel("HIGH")).toBe("Investigate");
    expect(riskLabel("CRITICAL")).toBe("Investigate");
    expect(riskLabel("MEDIUM")).toBe("Suspicious");
    expect(riskLabel("LOW")).toBe("Unusual");
    expect(riskLabel("INFO")).toBe("Observed");
    for (const r of ["HIGH", "CRITICAL", "MEDIUM", "LOW", "INFO"] as const) {
      expect(riskLabel(r).toLowerCase()).not.toContain("attack");
    }
  });
  it("guards relative time against invalid input", () => {
    expect(timeAgo("not-a-date")).toBe("—");
    expect(timeAgo("")).toBe("—");
  });
  it("assigns appropriate color classes for risk levels", () => {
    expect(riskColor("INFO")).toBe("text-mute");
    expect(riskColor("LOW")).toBe("text-info");
    expect(riskColor("MEDIUM")).toBe("text-warn");
    expect(riskColor("HIGH")).toBe("text-hot");
    expect(riskColor("CRITICAL")).toBe("text-hot");
  });
  it("formats relative time strings", () => {
    const now = new Date().toISOString();
    expect(timeAgo(now)).toBe("0s ago");
    const tenMinAgo = new Date(Date.now() - 10 * 60 * 1000).toISOString();
    expect(timeAgo(tenMinAgo)).toBe("10m ago");
    const twoHoursAgo = new Date(Date.now() - 2 * 60 * 60 * 1000).toISOString();
    expect(timeAgo(twoHoursAgo)).toBe("2h ago");
  });
  it("expands raw indicator strings into explainable text", () => {
    expect(indicatorText("new_process")).toContain("New process");
    expect(indicatorText("first_external_destination")).toContain("First time");
    expect(indicatorText("custom_indicator")).toBe("custom indicator");
  });
});
