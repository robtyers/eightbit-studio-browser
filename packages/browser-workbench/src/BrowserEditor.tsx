import { useEffect, useMemo, useRef, useState, type RefObject } from "react";
import { highlights, type BrowserEngine, type Highlight } from "./contracts";
const NO_HIGHLIGHTS: Highlight[] = [];

/** Native input remains authoritative for selection, touch keyboards and composition. */
export function BrowserEditor({ source, basic, busy, engine, inputRef, onChange, target = "dragon32", label = "Dragon 32" }: { target?: string; label?: string; source: string; basic: boolean; busy: boolean; engine: BrowserEngine; inputRef: RefObject<HTMLTextAreaElement | null>; onChange: (source: string) => void }) {
  const [presentation, setPresentation] = useState<{ source: string; spans: Highlight[] }>();
  const [composing, setComposing] = useState(false);
  const mirror = useRef<HTMLPreElement>(null);
  useEffect(() => {
    if (!basic || busy || composing || source.length > 65_536 || document.visibilityState === "hidden") return;
    const controller = new AbortController();
    const timer = window.setTimeout(() => {
      void engine.request({ schemaVersion: 1, operation: "presentation", source, target }, new Uint8Array(), controller.signal).then((value) => {
        if (!controller.signal.aborted) setPresentation({ source, spans: highlights(value, source) });
      }).catch(() => { if (!controller.signal.aborted) setPresentation(undefined); });
    }, 250);
    const suspend = () => { if (document.visibilityState === "hidden") { window.clearTimeout(timer); controller.abort(); } };
    document.addEventListener("visibilitychange", suspend);
    return () => { window.clearTimeout(timer); controller.abort(); document.removeEventListener("visibilitychange", suspend); };
  }, [source, basic, busy, composing, engine, target]);
  const spans = !composing && basic && presentation?.source === source ? presentation.spans : NO_HIGHLIGHTS;
  const content = useMemo(() => {
    const output = []; let end = 0;
    for (const item of spans) { output.push(source.slice(end, item.start), <span key={item.start} className={`browser-syntax-${item.kind}`}>{source.slice(item.start, item.end)}</span>); end = item.end; }
    output.push(source.slice(end), "\n"); return output;
  }, [source, spans]);
  useEffect(() => {
    const input = inputRef.current;
    if (input && mirror.current) mirror.current.style.transform = `translate(${-input.scrollLeft}px, ${-input.scrollTop}px)`;
  }, [source, inputRef]);
  return <div className={`editor-host browser-editor${spans.length ? " browser-editor-highlighted" : ""}`}>
    <div className="browser-editor-mirror" aria-hidden="true"><pre ref={mirror}>{content}</pre></div>
    <textarea ref={inputRef} aria-label={basic ? `${label} BASIC source` : "Recovered 6809 assembly source"} spellCheck={false} autoCapitalize="off" autoCorrect="off" value={source} onChange={(event) => onChange(event.currentTarget.value)} onCompositionStart={() => setComposing(true)} onCompositionEnd={() => setComposing(false)} onScroll={(event) => { if (mirror.current) mirror.current.style.transform = `translate(${-event.currentTarget.scrollLeft}px, ${-event.currentTarget.scrollTop}px)`; }} />
  </div>;
}
