import { useEffect, useRef, type ReactNode } from "react";

export function BrowserDialog({ title, onClose, children }: { title: string; onClose: () => void; children: ReactNode }) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const opener = document.activeElement;
    const focusable = () => Array.from(ref.current?.querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), a[href], [tabindex="0"]') ?? []);
    focusable()[0]?.focus();
    const keys = (event: KeyboardEvent) => {
      if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); onClose(); }
      if (event.key === "Tab") {
        const items = focusable(); const first = items[0]; const last = items[items.length - 1];
        if (event.shiftKey && (document.activeElement === first || !ref.current?.contains(document.activeElement))) { event.preventDefault(); last?.focus(); }
        else if (!event.shiftKey && (document.activeElement === last || !ref.current?.contains(document.activeElement))) { event.preventDefault(); first?.focus(); }
      }
    };
    document.addEventListener("keydown", keys);
    return () => { document.removeEventListener("keydown", keys); if (opener instanceof HTMLElement && opener.isConnected) opener.focus({ preventScroll: true }); };
  }, [onClose]);
  return <div className="browser-dialog-scrim"><div ref={ref} className="browser-dialog" role="dialog" aria-modal="true" aria-labelledby="browser-dialog-title"><div className="browser-dialog-heading"><h2 id="browser-dialog-title">{title}</h2><button type="button" onClick={onClose} aria-label={`Close ${title}`}>×</button></div>{children}</div></div>;
}
