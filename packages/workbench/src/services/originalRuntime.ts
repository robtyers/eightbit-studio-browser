export type OriginalRuntimeProfileId = "eightbit-dragon32-basic-1" | "eightbit-dragon32-basic-extended-1" | "eightbit-dragon64-basic-32k-dev-1" | "eightbit-dragon64-basic-64k-dev-1";
export function originalRuntimeProfileId(value: unknown): value is OriginalRuntimeProfileId {
  return value === "eightbit-dragon32-basic-1" || value === "eightbit-dragon32-basic-extended-1" || value === "eightbit-dragon64-basic-32k-dev-1" || value === "eightbit-dragon64-basic-64k-dev-1";
}
/** A profile label describes the admitted map; it never grants launch authority. */
export function originalRuntimeLabel(profile: OriginalRuntimeProfile): string {
  if (profile.target === "dragon64") return "Built-in Dragon 64 · 32K (development)";
  if (profile.target === "dragon64-native") return "Built-in Dragon 64 · native (development)";
  return profile.profileId === "eightbit-dragon32-basic-extended-1" ? "Built-in Dragon 32" : "Original BASIC";
}
/** Presentation of Rust's compiled admission; launch always rechecks in Rust. */
export interface OriginalRuntimeProfile {
  schemaVersion: 1;
  backend: "original";
  target: "dragon32" | "dragon64" | "dragon64-native";
  profileId: OriginalRuntimeProfileId;
  firmwareSha256: string;
  coreVersion: string;
}
function record(value: unknown): value is Record<string, unknown> { return value !== null && typeof value === "object" && !Array.isArray(value); }
export function originalRuntimeCapability(value: unknown): OriginalRuntimeProfile | null {
  if (!record(value) || Object.keys(value).some(key => key !== "schemaVersion" && key !== "profile") || value.schemaVersion !== 1) throw new Error("Invalid original runtime capability.");
  const profile = value.profile;
  if (profile === null) return null;
  if (!record(profile) || Object.keys(profile).some(key => !["schemaVersion", "backend", "target", "profileId", "firmwareSha256", "coreVersion"].includes(key)) || profile.schemaVersion !== 1 || profile.backend !== "original" || !validTarget(profile.target, profile.profileId) || !originalRuntimeProfileId(profile.profileId) || typeof profile.firmwareSha256 !== "string" || !/^[a-f0-9]{64}$/.test(profile.firmwareSha256) || typeof profile.coreVersion !== "string" || !/^[A-Za-z0-9.+-]{1,64}$/.test(profile.coreVersion)) throw new Error("Invalid original runtime capability.");
  return { schemaVersion: 1, backend: "original", target: profile.target, profileId: profile.profileId, firmwareSha256: profile.firmwareSha256, coreVersion: profile.coreVersion };
}

function validTarget(target: unknown, profile: unknown): target is OriginalRuntimeProfile["target"] {
  return target === "dragon32" && (profile === "eightbit-dragon32-basic-1" || profile === "eightbit-dragon32-basic-extended-1")
    || target === "dragon64" && profile === "eightbit-dragon64-basic-32k-dev-1"
    || target === "dragon64-native" && profile === "eightbit-dragon64-basic-64k-dev-1";
}
