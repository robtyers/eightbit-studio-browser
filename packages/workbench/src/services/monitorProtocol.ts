/** Transport bounds mirror the authoritative original-machine session contract. */
export const MONITOR_CASSETTE_LIMIT = 1024 * 1024;
export const MONITOR_REQUEST_LIMIT = 21024 * 1024;
export const MONITOR_RESPONSE_LIMIT = MONITOR_CASSETTE_LIMIT * 4 + 1024 * 1024;
export type MonitorCommand = { operation: "advance"; cycles: number } | { operation: "state" | "reset" | "stop" | "release" | "rewind-cassette" | "eject-cassette" | "start-recording" | "finish-recording" | "discard-recording" | "export-recording" } | { operation: "type"; text: string } | { operation: "key"; code: string; pressed: boolean } | { operation: "mount-cassette" | "mount-wav"; bytes: number[] };
export type MonitorTransport = (command: MonitorCommand) => Promise<unknown>;
