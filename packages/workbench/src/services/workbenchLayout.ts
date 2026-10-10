export interface WorkbenchLayout {
  primarySidebarWidth: number;
  assistantWidth: number;
  bottomPanelHeight: number;
}

export interface ViewportSize {
  width: number;
  height: number;
}

export interface WorkbenchLayoutLimits {
  primarySidebarMax: number;
  assistantMax: number;
  bottomPanelMax: number;
}

export const DEFAULT_WORKBENCH_LAYOUT: WorkbenchLayout = {
  primarySidebarWidth: 236,
  assistantWidth: 350,
  bottomPanelHeight: 190,
};

export const WORKBENCH_LAYOUT_MIN = {
  primarySidebarWidth: 160,
  assistantWidth: 260,
  bottomPanelHeight: 120,
} as const;

const STORAGE_KEY = "eightbit-studio.m6809.workbench-layout.v1";
const ACTIVITY_BAR_WIDTH = 49;
const SPLITTER_WIDTH = 5;
const EDITOR_MIN_WIDTH = 320;
const EDITOR_CHROME_MIN_HEIGHT = 260;
const ABSOLUTE_MAX = {
  primarySidebarWidth: 480,
  assistantWidth: 600,
  bottomPanelHeight: 600,
} as const;

export function loadWorkbenchLayout(): WorkbenchLayout {
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "null");
    if (!isLayout(parsed)) return DEFAULT_WORKBENCH_LAYOUT;
    return {
      primarySidebarWidth: clamp(parsed.primarySidebarWidth, WORKBENCH_LAYOUT_MIN.primarySidebarWidth, ABSOLUTE_MAX.primarySidebarWidth),
      assistantWidth: clamp(parsed.assistantWidth, WORKBENCH_LAYOUT_MIN.assistantWidth, ABSOLUTE_MAX.assistantWidth),
      bottomPanelHeight: clamp(parsed.bottomPanelHeight, WORKBENCH_LAYOUT_MIN.bottomPanelHeight, ABSOLUTE_MAX.bottomPanelHeight),
    };
  } catch {
    return DEFAULT_WORKBENCH_LAYOUT;
  }
}

export function saveWorkbenchLayout(layout: WorkbenchLayout): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(layout));
  } catch {
    // Layout persistence is a convenience; an unavailable store must not block the workbench.
  }
}

export function constrainWorkbenchLayout(
  layout: WorkbenchLayout,
  viewport: ViewportSize,
  assistantOpen: boolean,
): WorkbenchLayout {
  const horizontalBudget = Math.max(
    WORKBENCH_LAYOUT_MIN.primarySidebarWidth + (assistantOpen ? WORKBENCH_LAYOUT_MIN.assistantWidth : 0),
    viewport.width
      - ACTIVITY_BAR_WIDTH
      - EDITOR_MIN_WIDTH
      - SPLITTER_WIDTH
      - (assistantOpen ? SPLITTER_WIDTH : 0),
  );
  let primarySidebarWidth = clamp(
    layout.primarySidebarWidth,
    WORKBENCH_LAYOUT_MIN.primarySidebarWidth,
    ABSOLUTE_MAX.primarySidebarWidth,
  );
  let assistantWidth = clamp(
    layout.assistantWidth,
    WORKBENCH_LAYOUT_MIN.assistantWidth,
    ABSOLUTE_MAX.assistantWidth,
  );

  if (assistantOpen) {
    assistantWidth = Math.min(
      assistantWidth,
      Math.max(WORKBENCH_LAYOUT_MIN.assistantWidth, horizontalBudget - primarySidebarWidth),
    );
    primarySidebarWidth = Math.min(
      primarySidebarWidth,
      Math.max(WORKBENCH_LAYOUT_MIN.primarySidebarWidth, horizontalBudget - assistantWidth),
    );
  } else {
    primarySidebarWidth = Math.min(primarySidebarWidth, horizontalBudget);
  }

  return {
    primarySidebarWidth,
    assistantWidth,
    bottomPanelHeight: clamp(
      layout.bottomPanelHeight,
      WORKBENCH_LAYOUT_MIN.bottomPanelHeight,
      Math.min(ABSOLUTE_MAX.bottomPanelHeight, Math.max(WORKBENCH_LAYOUT_MIN.bottomPanelHeight, viewport.height - EDITOR_CHROME_MIN_HEIGHT)),
    ),
  };
}

export function workbenchLayoutLimits(
  layout: WorkbenchLayout,
  viewport: ViewportSize,
  assistantOpen: boolean,
): WorkbenchLayoutLimits {
  const constrained = constrainWorkbenchLayout(layout, viewport, assistantOpen);
  const fixedWidth = ACTIVITY_BAR_WIDTH
    + EDITOR_MIN_WIDTH
    + SPLITTER_WIDTH
    + (assistantOpen ? SPLITTER_WIDTH : 0);
  return {
    primarySidebarMax: Math.min(
      ABSOLUTE_MAX.primarySidebarWidth,
      Math.max(
        WORKBENCH_LAYOUT_MIN.primarySidebarWidth,
        viewport.width - fixedWidth - (assistantOpen ? constrained.assistantWidth : 0),
      ),
    ),
    assistantMax: Math.min(
      ABSOLUTE_MAX.assistantWidth,
      Math.max(
        WORKBENCH_LAYOUT_MIN.assistantWidth,
        viewport.width - fixedWidth - constrained.primarySidebarWidth,
      ),
    ),
    bottomPanelMax: Math.min(
      ABSOLUTE_MAX.bottomPanelHeight,
      Math.max(WORKBENCH_LAYOUT_MIN.bottomPanelHeight, viewport.height - EDITOR_CHROME_MIN_HEIGHT),
    ),
  };
}

function isLayout(value: unknown): value is WorkbenchLayout {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Partial<WorkbenchLayout>;
  return finite(candidate.primarySidebarWidth)
    && finite(candidate.assistantWidth)
    && finite(candidate.bottomPanelHeight);
}

function finite(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.round(Math.min(Math.max(value, minimum), Math.max(minimum, maximum)));
}
