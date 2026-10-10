import { useEffect, useRef, type KeyboardEvent, type PointerEvent } from "react";

interface WorkbenchResizeHandleProps {
  label: string;
  controls: string;
  orientation: "vertical" | "horizontal";
  value: number;
  minimum: number;
  maximum: number;
  pointerFactor: 1 | -1;
  onChange: (value: number) => void;
  onReset: () => void;
  className?: string;
}

interface DragState {
  pointerId: number;
  startPosition: number;
  startValue: number;
}

export function WorkbenchResizeHandle({
  label,
  controls,
  orientation,
  value,
  minimum,
  maximum,
  pointerFactor,
  onChange,
  onReset,
  className = "",
}: WorkbenchResizeHandleProps) {
  const dragRef = useRef<DragState | null>(null);
  const frameRef = useRef<number | null>(null);
  const pendingValueRef = useRef(value);

  useEffect(() => () => {
    if (frameRef.current !== null) cancelAnimationFrame(frameRef.current);
    clearResizeCursor();
  }, []);

  function onPointerDown(event: PointerEvent<HTMLDivElement>) {
    if (event.button !== 0) return;
    event.preventDefault();
    dragRef.current = {
      pointerId: event.pointerId,
      startPosition: orientation === "vertical" ? event.clientX : event.clientY,
      startValue: value,
    };
    pendingValueRef.current = value;
    event.currentTarget.setPointerCapture(event.pointerId);
    document.body.dataset.workbenchResize = orientation;
  }

  function onPointerMove(event: PointerEvent<HTMLDivElement>) {
    const drag = dragRef.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    const position = orientation === "vertical" ? event.clientX : event.clientY;
    pendingValueRef.current = clamp(
      drag.startValue + ((position - drag.startPosition) * pointerFactor),
      minimum,
      maximum,
    );
    if (frameRef.current !== null) return;
    frameRef.current = requestAnimationFrame(() => {
      frameRef.current = null;
      onChange(pendingValueRef.current);
    });
  }

  function finishPointerResize(event: PointerEvent<HTMLDivElement>) {
    const drag = dragRef.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    if (frameRef.current !== null) {
      cancelAnimationFrame(frameRef.current);
      frameRef.current = null;
      onChange(pendingValueRef.current);
    }
    dragRef.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    clearResizeCursor();
  }

  function onKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    const step = event.shiftKey ? 40 : 10;
    let next: number | null = null;
    if (event.key === "Home") next = minimum;
    if (event.key === "End") next = maximum;
    if (orientation === "vertical" && event.key === "ArrowLeft") next = value - (step * pointerFactor);
    if (orientation === "vertical" && event.key === "ArrowRight") next = value + (step * pointerFactor);
    if (orientation === "horizontal" && event.key === "ArrowUp") next = value - (step * pointerFactor);
    if (orientation === "horizontal" && event.key === "ArrowDown") next = value + (step * pointerFactor);
    if (next === null) return;
    event.preventDefault();
    onChange(clamp(next, minimum, maximum));
  }

  return (
    <div
      className={`workbench-resizer ${orientation} ${className}`.trim()}
      role="separator"
      aria-label={label}
      aria-controls={controls}
      aria-orientation={orientation}
      aria-valuemin={minimum}
      aria-valuemax={maximum}
      aria-valuenow={value}
      tabIndex={0}
      title={`${label}. Drag to resize; double-click to reset.`}
      onDoubleClick={onReset}
      onKeyDown={onKeyDown}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={finishPointerResize}
      onPointerCancel={finishPointerResize}
      onLostPointerCapture={() => {
        dragRef.current = null;
        clearResizeCursor();
      }}
    />
  );
}

function clearResizeCursor() {
  delete document.body.dataset.workbenchResize;
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.round(Math.min(Math.max(value, minimum), maximum));
}
