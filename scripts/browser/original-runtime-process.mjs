import { spawn } from 'node:child_process';

/** The executable itself is owned, never a cargo-run parent. This peer cannot
 * spawn children. Await close before reporting acceptance, even on timeout. */
export function spawnOwnedPeer(executable, args = [], options = {}, graceMs = 1_000) {
  const child = spawn(executable, args, { ...options, stdio: ['pipe', 'pipe', 'inherit'] });
  let ended = false;
  let spawnError;
  child.on('error', error => { spawnError = error; });
  // A broken input pipe is also reported by close; never crash during teardown.
  child.stdin.on('error', () => {});
  const closed = new Promise(resolve => child.once('close', (code, signal) => {
    ended = true;
    resolve({ code, signal, error: spawnError });
  }));
  async function wait() {
    let timer;
    try { return await Promise.race([closed, new Promise(resolve => { timer = setTimeout(() => resolve(null), graceMs); })]); }
    finally { clearTimeout(timer); }
  }
  let stopping;
  function stop() {
    stopping ??= (async () => {
      if (ended) return closed;
      child.stdin.end();
      let result = await wait();
      if (result) return result;
      child.kill('SIGTERM');
      result = await wait();
      if (result) return result;
      child.kill('SIGKILL');
      result = await wait();
      if (!result) throw new Error('Owned native acceptance peer did not terminate');
      return result;
    })();
    return stopping;
  }
  return { child, closed, stop };
}
