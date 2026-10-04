import { State } from "./state";

/** The layout zoom and the physical density of each canvas are separate. */
export function islandScale(): number {
  if (State.renderScale != null) return State.renderScale;
  const value = State.mode === "expanded" ? State.settings.expandedScale : State.settings.compactScale;
  return Math.max(.75, Math.min(1.5, value || 1));
}

export function canvasDensity(): number {
  return Math.max(1, Math.min(4, (window.devicePixelRatio || 1) * islandScale()));
}

export function resizeCanvas(canvas: HTMLCanvasElement, width: number, height: number, density = canvasDensity()): number {
  const pixelsW = Math.max(1, Math.round(width * density));
  const pixelsH = Math.max(1, Math.round(height * density));
  if (canvas.width !== pixelsW || canvas.height !== pixelsH) {
    canvas.width = pixelsW;
    canvas.height = pixelsH;
  }
  canvas.style.width = `${width}px`;
  canvas.style.height = `${height}px`;
  return density;
}
