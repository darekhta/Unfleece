// Pure coordinate math for the PDF editors — NO pdf.js import, so it's unit-testable
// in Node. Screen/overlay space: origin top-left, +Y down, CSS px. PDF user space:
// origin bottom-left, +Y up, points. `scale` = CSS px per PDF point.

export interface StageDims {
  scale: number;
  cssWidth: number;
  cssHeight: number;
  pageWidthPt: number;
  pageHeightPt: number;
}

/** Overlay rect, top-left origin, CSS px. */
export interface RectPx {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface SnapGuides {
  vertical?: number;
  horizontal?: number;
}

/** Overlay point (CSS px) -> PDF point (bottom-left origin). */
export function viewToPdf(vx: number, vy: number, d: StageDims): { x: number; y: number } {
  return { x: vx / d.scale, y: d.pageHeightPt - vy / d.scale };
}

/** PDF point (bottom-left origin) -> overlay point (CSS px). */
export function pdfToView(px: number, py: number, d: StageDims): { x: number; y: number } {
  return { x: px * d.scale, y: (d.pageHeightPt - py) * d.scale };
}

/** Crop rect (overlay px) -> margins trimmed from each edge, in points (clamped ≥0). */
export function cropMargins(r: RectPx, d: StageDims): { top: number; right: number; bottom: number; left: number } {
  return {
    top: Math.max(0, r.y / d.scale),
    bottom: Math.max(0, (d.cssHeight - (r.y + r.h)) / d.scale),
    left: Math.max(0, r.x / d.scale),
    right: Math.max(0, (d.cssWidth - (r.x + r.w)) / d.scale),
  };
}

/** Stamp rect (overlay px) -> stampImage params {x,y bottom-left pt, width, height pt}. */
export function stampRect(r: RectPx, d: StageDims): { x: number; y: number; width: number; height: number } {
  return {
    x: r.x / d.scale,
    y: d.pageHeightPt - (r.y + r.h) / d.scale,
    width: r.w / d.scale,
    height: r.h / d.scale,
  };
}

/** Clamp a rect to stay within the page bounds, keeping a minimum size (px). */
export function clampRect(r: RectPx, d: StageDims, min = 16): RectPx {
  const w = Math.min(Math.max(r.w, min), d.cssWidth);
  const h = Math.min(Math.max(r.h, min), d.cssHeight);
  const x = Math.min(Math.max(r.x, 0), d.cssWidth - w);
  const y = Math.min(Math.max(r.y, 0), d.cssHeight - h);
  return { x, y, w, h };
}

/** Snap a rectangle's center to page-center guides when it is close enough. */
export function snapRectToGuides(r: RectPx, d: StageDims, threshold = 8): { rect: RectPx; guides: SnapGuides } {
  const rect = clampRect(r, d, 1);
  const guides: SnapGuides = {};
  const pageCenterX = d.cssWidth / 2;
  const pageCenterY = d.cssHeight / 2;
  const rectCenterX = rect.x + rect.w / 2;
  const rectCenterY = rect.y + rect.h / 2;

  if (Math.abs(rectCenterX - pageCenterX) <= threshold) {
    rect.x = pageCenterX - rect.w / 2;
    guides.vertical = pageCenterX;
  }
  if (Math.abs(rectCenterY - pageCenterY) <= threshold) {
    rect.y = pageCenterY - rect.h / 2;
    guides.horizontal = pageCenterY;
  }

  return { rect: clampRect(rect, d, 1), guides };
}
