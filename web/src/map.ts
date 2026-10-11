// The map canvas: one pixel per cell in an offscreen canvas, drawn with zoom and pan.
const css = (name: string) => getComputedStyle(document.documentElement).getPropertyValue(name).trim();

export class MapView {
  img: ImageData | null = null;
  hover = -1;
  pick = -1;
  private ctx: CanvasRenderingContext2D;
  private off = document.createElement('canvas');
  private cam = { s: 1, ox: 0, oy: 0 };
  private fitted = true;
  private w = 0; private h = 0; private cellM = 1;
  private raf = 0;

  constructor(private cv: HTMLCanvasElement, private onHover: (i: number) => void, private onPick: (i: number) => void) {
    this.ctx = cv.getContext('2d')!;
    new ResizeObserver(() => { this.resize(); this.invalidate(); }).observe(cv);
    const ptrs = new Map<number, { x: number; y: number }>();
    let moved = 0;
    cv.addEventListener('pointerdown', e => { cv.setPointerCapture(e.pointerId); ptrs.set(e.pointerId, { x: e.offsetX, y: e.offsetY }); if (ptrs.size === 1) moved = 0; });
    cv.addEventListener('pointermove', e => {
      const p = ptrs.get(e.pointerId);
      if (!p) return this.setHover(this.cellAt(e.offsetX, e.offsetY));
      const before = [...ptrs.values()].map(q => ({ ...q }));
      p.x = e.offsetX; p.y = e.offsetY;
      const after = [...ptrs.values()];
      const mid = (a: { x: number; y: number }[]) => ({ x: a.reduce((t, q) => t + q.x, 0) / a.length, y: a.reduce((t, q) => t + q.y, 0) / a.length });
      const m0 = mid(before), m1 = mid(after);
      if (ptrs.size > 1) { this.zoomAt(m0.x, m0.y, Math.hypot(after[0].x - after[1].x, after[0].y - after[1].y) / Math.hypot(before[0].x - before[1].x, before[0].y - before[1].y)); moved = 99; }
      moved += Math.abs(m1.x - m0.x) + Math.abs(m1.y - m0.y);
      this.cam.ox += m1.x - m0.x; this.cam.oy += m1.y - m0.y; this.fitted = false; this.invalidate();
    });
    const up = (e: PointerEvent) => {
      if (ptrs.size === 1 && moved < 6 && e.type === 'pointerup') { this.pick = this.cellAt(e.offsetX, e.offsetY); this.onPick(this.pick); this.invalidate(); }
      ptrs.delete(e.pointerId);
    };
    cv.addEventListener('pointerup', up); cv.addEventListener('pointercancel', up);
    cv.addEventListener('pointerleave', () => this.setHover(-1));
    cv.addEventListener('wheel', e => { e.preventDefault(); this.zoomAt(e.offsetX, e.offsetY, Math.exp(-e.deltaY * 0.0015)); }, { passive: false });
    cv.addEventListener('dblclick', () => this.fit());
  }

  setWorld(w: number, h: number, cellM: number): void {
    this.w = w; this.h = h; this.cellM = cellM;
    this.off.width = w; this.off.height = h;
    this.img = new ImageData(w, h);
    this.pick = this.hover = -1;
    this.resize(); this.fit();
  }

  commit(): void { this.off.getContext('2d')!.putImageData(this.img!, 0, 0); this.invalidate(); }

  fit(): void {
    const k = this.fitScale();
    this.cam = { s: k, ox: (this.cv.clientWidth - this.w * k) / 2, oy: (this.cv.clientHeight - this.h * k) / 2 };
    this.fitted = true; this.invalidate();
  }

  invalidate(): void { if (!this.raf) this.raf = requestAnimationFrame(() => { this.raf = 0; this.draw(); }); }

  private fitScale = () => Math.min(this.cv.clientWidth / this.w, this.cv.clientHeight / this.h) * 0.94 || 1;
  private dpr = () => window.devicePixelRatio || 1;

  private resize(): void {
    const d = this.dpr();
    this.cv.width = Math.round(this.cv.clientWidth * d); this.cv.height = Math.round(this.cv.clientHeight * d);
    if (this.fitted && this.w) this.fit();
  }

  private cellAt(px: number, py: number): number {
    const x = Math.floor((px - this.cam.ox) / this.cam.s), y = Math.floor((py - this.cam.oy) / this.cam.s);
    return x >= 0 && y >= 0 && x < this.w && y < this.h ? y * this.w + x : -1;
  }

  private setHover(i: number): void { if (i !== this.hover) { this.hover = i; this.onHover(i); this.invalidate(); } }

  private zoomAt(px: number, py: number, k: number): void {
    const c = this.cam, s = Math.min(Math.max(c.s * k, this.fitScale() * 0.5), 90);
    c.ox = px - ((px - c.ox) / c.s) * s; c.oy = py - ((py - c.oy) / c.s) * s; c.s = s;
    this.fitted = false; this.invalidate();
  }

  private draw(): void {
    if (!this.w) return;
    const { ctx, cam: { s, ox, oy }, w, h } = this, d = this.dpr();
    ctx.setTransform(d, 0, 0, d, 0, 0);
    ctx.fillStyle = css('--map-bg'); ctx.fillRect(0, 0, this.cv.clientWidth, this.cv.clientHeight);
    ctx.save();
    ctx.shadowColor = 'rgba(0,0,0,.28)'; ctx.shadowBlur = 22; ctx.shadowOffsetY = 5;
    ctx.fillStyle = '#888'; ctx.fillRect(ox, oy, w * s, h * s);
    ctx.restore();
    ctx.imageSmoothingEnabled = s * d < 6; ctx.imageSmoothingQuality = 'high';
    ctx.drawImage(this.off, ox, oy, w * s, h * s);
    for (const [i, col] of [[this.pick, '#fff'], [this.hover, '#000']] as const) {
      if (i < 0) continue;
      ctx.lineWidth = 1.5; ctx.strokeStyle = col;
      ctx.strokeRect(ox + (i % w) * s - 0.5, oy + Math.floor(i / w) * s - 0.5, s + 1, s + 1);
    }
    this.scaleBar();
  }

  private scaleBar(): void {
    const { ctx, cam: { s }, cellM } = this, target = (96 / s) * cellM; // metres for ~96 px
    const p = 10 ** Math.floor(Math.log10(target)), m = [1, 2, 5, 10].map(k => k * p).reduce((a, b) => (Math.abs(b - target) < Math.abs(a - target) ? b : a));
    const px = (m / cellM) * s, x = 14, y = this.cv.clientHeight - 14;
    ctx.strokeStyle = ctx.fillStyle = css('--ink'); ctx.lineWidth = 1.5; ctx.globalAlpha = 0.75;
    ctx.beginPath(); ctx.moveTo(x, y - 5); ctx.lineTo(x, y); ctx.lineTo(x + px, y); ctx.lineTo(x + px, y - 5); ctx.stroke();
    ctx.font = '11px system-ui, sans-serif'; ctx.textBaseline = 'bottom';
    ctx.fillText(m >= 1000 ? `${m / 1000} km` : `${m} m`, x, y - 7);
    ctx.globalAlpha = 1;
  }
}
