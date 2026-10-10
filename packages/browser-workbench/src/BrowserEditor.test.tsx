import { act, fireEvent, render, screen } from "@testing-library/react";
import { createRef } from "react";
import { BrowserEditor } from "./BrowserEditor";
import type { BrowserEngine } from "./contracts";
const SOURCE = "10 END\n";
const HIGHLIGHTS = { schemaVersion: 1, ok: true, result: { highlights: [{ start: 0, end: 2, kind: "number" }, { start: 3, end: 6, kind: "keyword" }] } };
function fixture(request: BrowserEngine["request"]) { return { source: SOURCE, basic: true, busy: false, engine: { request }, inputRef: createRef<HTMLTextAreaElement>(), onChange: vi.fn() }; }
beforeEach(() => vi.useFakeTimers());
afterEach(() => { vi.useRealTimers(); vi.restoreAllMocks(); });
it("renders bounded Rust ranges while preserving native, safely escaped text input", async () => {
  const request = vi.fn(async () => HIGHLIGHTS); const props = fixture(request); const { container } = render(<BrowserEditor {...props} />);
  await act(async () => { await vi.advanceTimersByTimeAsync(251); });
  expect(container.querySelector('.browser-syntax-keyword')).toHaveTextContent("END");
  expect(screen.getByRole("textbox")).toHaveValue(SOURCE);
  fireEvent.change(screen.getByRole("textbox"), { target: { value: '<img src=x onerror="alert(1)">' } });
  expect(props.onChange).toHaveBeenCalledWith('<img src=x onerror="alert(1)">');
  expect(container.querySelector("img")).toBeNull();
});
it("aborts stale presentation after edits, during builds and on unmount", async () => {
  let resolve: ((value: unknown) => void) | undefined; let signal: AbortSignal | undefined;
  const props = fixture((_request, _bytes, abort) => { signal = abort; return new Promise((done) => { resolve = done; }); });
  const { rerender, container, unmount } = render(<BrowserEditor {...props} />);
  await act(async () => { await vi.advanceTimersByTimeAsync(251); });
  rerender(<BrowserEditor {...props} source="20 END\n" />); expect(signal?.aborted).toBe(true);
  await act(async () => { resolve?.(HIGHLIGHTS); }); expect(container.querySelector('.browser-syntax-keyword')).toBeNull();
  await act(async () => { await vi.advanceTimersByTimeAsync(251); });
  rerender(<BrowserEditor {...props} source="20 END\n" busy />); expect(signal?.aborted).toBe(true);
  rerender(<BrowserEditor {...props} />); await act(async () => { await vi.advanceTimersByTimeAsync(251); });
  unmount(); expect(signal?.aborted).toBe(true);
});
it("uses plain text for malformed presentation, assembly and large documents, and preserves composition", async () => {
  const request = vi.fn(async () => ({ schemaVersion: 1, ok: true, result: { highlights: [{ start: 0, end: 500, kind: "html" }] } }));
  const props = fixture(request); const { container, rerender } = render(<BrowserEditor {...props} />);
  await act(async () => { await vi.advanceTimersByTimeAsync(251); }); expect(container.querySelector('.browser-editor-highlighted')).toBeNull();
  rerender(<BrowserEditor {...props} basic={false} />); await act(async () => { await vi.advanceTimersByTimeAsync(251); });
  rerender(<BrowserEditor {...props} source={' '.repeat(65_537)} />); await act(async () => { await vi.advanceTimersByTimeAsync(251); });
  expect(request).toHaveBeenCalledTimes(1);
  rerender(<BrowserEditor {...props} />); fireEvent.compositionStart(screen.getByRole("textbox"));
  await act(async () => { await vi.advanceTimersByTimeAsync(251); }); expect(request).toHaveBeenCalledTimes(1);
  fireEvent.compositionEnd(screen.getByRole("textbox")); await act(async () => { await vi.advanceTimersByTimeAsync(251); }); expect(request).toHaveBeenCalledTimes(2);
});
