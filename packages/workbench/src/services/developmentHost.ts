import type { MonitorTransport } from "./monitorProtocol";
import { OriginalBasicDebugTransport, originalBasicDebugReply, type OriginalDebugIdentity, type OriginalDebugReply } from "./originalBasicDebug";
export interface DevelopmentHostPorts {
  monitor: MonitorTransport;
  debug: (request: string | null, signal: AbortSignal) => Promise<unknown>;
  release: () => void;
}
interface Task { terminal: boolean; run: (signal: AbortSignal) => Promise<void>; resolve: () => void; reject: (error: unknown) => void }
/** One bounded queue for the existing frame scheduler and explicit debug actions.
 * Reads and their revision-bound commands are indivisible to that scheduler.
 * There is no background polling; only explicit start types the guest RUN command. */
export type HostDebugAction = "continue" | "cancel" | "step" | "pause";
export interface DebugHostProtocol<Reply> {
  read: (json: string, owner: OriginalDebugIdentity) => Reply;
  control: (initial: string, owner: OriginalDebugIdentity, operation: HostDebugAction, call: DevelopmentHostPorts["debug"], release: () => void, signal: AbortSignal) => Promise<Reply>;
}
export const basicHostProtocol: DebugHostProtocol<OriginalDebugReply> = {
  read: originalBasicDebugReply,
  async control(initial, owner, operation, call, release, signal) {
    if (operation !== "continue" && operation !== "cancel") throw new Error("This BASIC debugger does not support instruction controls.");
    const transport = new OriginalBasicDebugTransport(initial, owner, async (request, requestSignal) => {
      if (signal.aborted || requestSignal.aborted) throw new Error("Development session disconnected.");
      const encoded = await call(request, signal);
      if (typeof encoded !== "string") throw new Error("Invalid development debug response.");
      return encoded;
    }, release);
    const abort = () => transport.dispose();
    signal.addEventListener("abort", abort, { once: true });
    try { return await transport.request({ operation }); }
    finally { signal.removeEventListener("abort", abort); }
  },
};
export class OwnedDebugHostConnection<Reply extends { revision: { revision: string } }> {
  private owner: OriginalDebugIdentity;
  private queue: Task[] = [];
  private active?: { task: Task; abort: AbortController; timer: ReturnType<typeof setTimeout> };
  private closed = false;
  private ending = false;
  private revision?: bigint;
  constructor(owner: OriginalDebugIdentity, private readonly ports: DevelopmentHostPorts, private readonly publish: (reply: Reply) => void, private readonly protocol: DebugHostProtocol<Reply>) {
    this.owner = { ...owner };
    if (!Number.isInteger(owner.sessionId) || owner.sessionId <= 0 || owner.sessionId > 0xffffffff || !/^[a-f0-9]{64}$/.test(owner.buildId)) { this.dispose(); throw new Error("Invalid development session identity."); }
  }
  private present(reply: Reply) {
    const revision = BigInt(reply.revision.revision);
    if (this.revision !== undefined && revision < this.revision) throw new Error("Stale development inspection.");
    this.revision = revision;
    this.publish(reply);
  }
  private async read(signal: AbortSignal): Promise<string> {
    try {
      if (signal.aborted) throw new Error("Development session disconnected.");
      const json = await this.ports.debug(null, signal);
      if (signal.aborted) throw new Error("Development session disconnected.");
      if (typeof json !== "string") throw new Error("Missing development debugger.");
      this.present(this.protocol.read(json, this.owner));
      return json;
    } catch (error) { this.dispose(); throw error; }
  }
  state = (): Promise<Reply> => this.enqueue(async signal => this.protocol.read(await this.read(signal), this.owner));
  monitor: MonitorTransport = command => this.enqueue(async signal => {
    const frame = await this.ports.monitor(command);
    if (signal.aborted) throw new Error("Development session disconnected.");
    if (command.operation !== "stop") await this.read(signal);
    return frame;
  }, command.operation === "stop");
  /** Continue/cancel refresh their revision within the same queue turn. */
  debug = (operation: HostDebugAction): Promise<Reply> => this.enqueue(signal => this.control(signal, operation));
  /** Explicit Debug queues real guest RUN and Continue without a frame tick
   * between them. The caller first completes the guest CLOAD journey. */
  start = (): Promise<Reply> => this.enqueue(async signal => {
    if (this.owner.backend === "original") throw new Error("Normal Run and Debug require a fresh project launch.");
    await this.ports.monitor({ operation: "type", text: "RUN" });
    return this.control(signal, "continue");
  });
  private async control(signal: AbortSignal, operation: HostDebugAction) {
    const initial = await this.read(signal);
    const reply = await this.protocol.control(initial, this.owner, operation, this.ports.debug, () => this.dispose(), signal);
    if (!signal.aborted) this.present(reply);
    return reply;
  }

  private enqueue<T>(run: (signal: AbortSignal) => Promise<T>, terminal = false): Promise<T> {
    if (this.closed || this.ending || this.queue.length >= 16) return Promise.reject(new Error("Development session is disconnected or its queue is full."));
    if (terminal) this.ending = true;
    return new Promise((resolve, reject) => {
      let complete = () => {};
      this.queue.push({ run: async signal => { const value = await run(signal); complete = () => resolve(value); }, resolve: () => complete(), reject, terminal });
      this.drain();
    });
  }
  private drain() {
    if (this.closed || this.active) return;
    const task = this.queue.shift(); if (!task) return;
    const abort = new AbortController();
    const active = { task, abort, timer: setTimeout(() => this.dispose("Development session operation timed out."), 5000) };
    this.active = active;
    void Promise.resolve().then(() => {
      if (abort.signal.aborted) throw new Error("Development session disconnected.");
      return task.run(abort.signal);
    }).then(() => this.finish(active, task.resolve), error => this.finish(active, () => task.reject(error)));
  }
  private finish(active: NonNullable<OwnedDebugHostConnection<Reply>["active"]>, complete: () => void) {
    if (this.active !== active || this.closed) return;
    clearTimeout(active.timer); this.active = undefined; complete();
    if (active.task.terminal) this.dispose(); else this.drain();
  }
  dispose(message = "Development session disconnected.") {
    if (this.closed) return;
    this.closed = true;
    const active = this.active; this.active = undefined;
    if (active) { clearTimeout(active.timer); active.abort.abort(); active.task.reject(new Error(message)); }
    for (const task of this.queue.splice(0)) task.reject(new Error(message));
    try { this.ports.release(); } catch { /* Separate host teardown remains owned. */ }
  }
}

/** Existing BASIC hosts retain their exact parser and request semantics. */
export class DevelopmentHostConnection extends OwnedDebugHostConnection<OriginalDebugReply> {
  constructor(owner: OriginalDebugIdentity, ports: DevelopmentHostPorts, publish: (reply: OriginalDebugReply) => void) {
    super(owner, ports, publish, basicHostProtocol);
  }
}
