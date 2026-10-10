import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
beforeEach(() => {
  let queue: Promise<unknown> = Promise.resolve();
  Object.defineProperty(navigator, "locks", { configurable: true, value: {
    request: (_name: string, _options: LockOptions, callback: () => unknown) => {
      const task = queue.then(callback); queue = task.catch(() => undefined); return task;
    },
  } });
});
afterEach(() => cleanup());
