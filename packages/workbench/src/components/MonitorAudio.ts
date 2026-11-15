/** Owns only presentation buffers; the Rust board generates every sample. */
export class MonitorAudio {
  constructor(private readonly sampleRate: 44100 | 48000 = 48000) {}
  private context: AudioContext | undefined;
  private sources = new Set<AudioBufferSourceNode>();
  private nextTime = 0;
  private disposed = false;

  async resume(): Promise<boolean> {
    if (this.disposed || typeof AudioContext === "undefined") return false;
    try {
      this.context ??= new AudioContext({ latencyHint: "interactive" });
      await this.context.resume();
      return !this.disposed && this.context.state === "running";
    } catch { return false; }
  }

  push(samples: number[]): void {
    const context = this.context;
    if (this.disposed || !context || context.state !== "running" || samples.length === 0) return;
    if (this.sources.size >= 8 || this.nextTime > context.currentTime + 0.15) this.clear();
    const buffer = context.createBuffer(1, samples.length, this.sampleRate);
    const channel = buffer.getChannelData(0);
    for (let index = 0; index < samples.length; index += 1) channel[index] = samples[index]! / 32768;
    const source = context.createBufferSource();
    source.buffer = buffer;
    source.connect(context.destination);
    source.onended = () => { source.disconnect(); this.sources.delete(source); };
    this.sources.add(source);
    const start = Math.max(context.currentTime + 0.02, this.nextTime);
    source.start(start);
    this.nextTime = start + buffer.duration;
  }

  clear(): void {
    for (const source of this.sources) { source.onended = null; source.stop(); source.disconnect(); }
    this.sources.clear();
    this.nextTime = 0;
  }

  dispose(): void {
    this.disposed = true;
    this.clear();
    if (this.context) void this.context.close().catch(() => {});
    this.context = undefined;
  }
}
