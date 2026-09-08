export function countryFlag(code?: string | null): string {
  if (!code || code.length !== 2 || code === "??") {
    return "🌐";
  }
  const upper = code.toUpperCase();
  const c1 = upper.charCodeAt(0);
  const c2 = upper.charCodeAt(1);
  if (c1 < 65 || c1 > 90 || c2 < 65 || c2 > 90) {
    return "🌐";
  }
  return String.fromCodePoint(0x1f1e6 + c1 - 65, 0x1f1e6 + c2 - 65);
}
