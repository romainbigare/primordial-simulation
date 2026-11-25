// What god mode draws over the map (god.ts `draw`): the round brush at its true size, the heat it holds as a ring that
// fills and as flames that grow in it, the brush's size beside it, and the reason a tool cannot act there.

/** The width of the ring that shows the heat held under the brush, css px. */
const PROGRESS_PX = 5;
/** The widest line of the box that says why a tool cannot act, css px, and its gap to the brush. */
const LABEL_PX = 220, LABEL_GAP_PX = 10;
/** Flames drawn in the brush while it heats: this many at the least heat and this many more when the ground lights,
 * each a glow of this share of the brush's radius at most (and no less than `FLAME_LEAST_PX`), flickering this many
 * times a second. */
const FLAMES_LEAST = 4, FLAMES_MORE = 26, FLAME_SIZE = 0.16, FLAME_LEAST_PX = 6, FLICKER_HZ = 7;
/** The flames' colours: the hot core, the flame, and the edge where it fades. */
const FLAME_CORE = 'rgba(255,236,170,.95)', FLAME_BODY = 'rgba(255,140,30,.75)', FLAME_EDGE = 'rgba(200,40,10,0)';
/** A flame is this many times taller than wide. */
const FLAME_TALL = 1.8;
/** The angle between one flame and the next about the brush's centre, radians: the golden angle, so they spread evenly
 * over the disc however many there are. */
const GOLDEN_ANGLE = Math.PI * (3 - Math.sqrt(5));

/** The brush at (cx, cy), `r` css px: a disc of `ink` (`held`: while a stroke is under way, stronger), its rim, its
 * centre; the heat it holds (`progress`, 0 to 1; less than 0: none asked) as a ring that fills. */
export function drawBrush(ctx: CanvasRenderingContext2D, cx: number, cy: number, r: number, ink: string, held: boolean, progress: number): void {
  ctx.save();
  ctx.beginPath(); ctx.arc(cx, cy, r, 0, 2 * Math.PI);
  ctx.globalAlpha = held ? 0.3 : 0.15; ctx.fillStyle = ink; ctx.fill();
  ctx.globalAlpha = 1; ctx.lineWidth = 3; ctx.strokeStyle = 'rgba(0,0,0,.45)'; ctx.stroke();
  ctx.lineWidth = 1.5; ctx.strokeStyle = ink; ctx.stroke();
  if (progress > 0) {
    ctx.lineWidth = PROGRESS_PX; ctx.strokeStyle = ink;
    ctx.beginPath(); ctx.arc(cx, cy, r, -Math.PI / 2, -Math.PI / 2 + 2 * Math.PI * Math.min(progress, 1)); ctx.stroke();
  }
  ctx.fillStyle = ink; ctx.strokeStyle = 'rgba(0,0,0,.5)'; ctx.lineWidth = 1;
  ctx.beginPath(); ctx.arc(cx, cy, 3, 0, 2 * Math.PI); ctx.fill(); ctx.stroke();
  ctx.restore();
}

/** Flames in the brush at (cx, cy), `r` css px, as much of the heat that lights the ground under it held as `progress`
 * (0 to 1), at time `now` (ms): more and bigger as it nears lighting. A picture of the heat held, not the core's
 * flames: those play when the stroke settles (fire.ts `play`). */
export function drawFlames(ctx: CanvasRenderingContext2D, cx: number, cy: number, r: number, progress: number, now: number): void {
  const p = Math.min(Math.max(progress, 0), 1), n = Math.round(FLAMES_LEAST + FLAMES_MORE * p), t = (now / 1000) * FLICKER_HZ;
  ctx.save();
  ctx.globalCompositeOperation = 'lighter';
  for (let k = 0; k < n; k++) {
    // Each flame keeps its place (a fixed scatter over the disc) and flickers on its own phase.
    const a = k * GOLDEN_ANGLE, d = r * 0.85 * Math.sqrt((k + 0.5) / (FLAMES_LEAST + FLAMES_MORE));
    const flicker = 0.65 + 0.35 * Math.sin(t + k * 1.7) * Math.sin(0.6 * t + k);
    const size = Math.max(FLAME_LEAST_PX, r * FLAME_SIZE * (0.4 + 0.6 * p)) * flicker;
    const x = cx + d * Math.cos(a), y = cy + d * Math.sin(a);
    // A tongue of flame: a glow drawn taller than wide, its foot at the point.
    ctx.save();
    ctx.translate(x, y); ctx.scale(1, FLAME_TALL);
    const g = ctx.createRadialGradient(0, -0.5 * size, 0, 0, -0.5 * size, size);
    g.addColorStop(0, FLAME_CORE); g.addColorStop(0.35, FLAME_BODY); g.addColorStop(1, FLAME_EDGE);
    ctx.globalAlpha = 0.35 + 0.65 * p;
    ctx.fillStyle = g;
    ctx.beginPath(); ctx.arc(0, -0.5 * size, size, 0, 2 * Math.PI); ctx.fill();
    ctx.restore();
  }
  ctx.restore();
}

/** The brush's size `text` under the brush at (cx, cy), `r` css px, on a dark pill. */
export function drawSizeTag(ctx: CanvasRenderingContext2D, cx: number, cy: number, r: number, text: string): void {
  ctx.save();
  ctx.font = '600 12px system-ui, sans-serif';
  const w = ctx.measureText(text).width + 16, h = 22, x = cx - w / 2, y = cy + r + 6;
  ctx.fillStyle = 'rgba(0,0,0,.6)';
  ctx.beginPath(); ctx.roundRect(x, y, w, h, h / 2); ctx.fill();
  ctx.fillStyle = '#fff'; ctx.textAlign = 'center';
  ctx.fillText(text, cx, y + 15);
  ctx.restore();
}

/** Why a tool cannot act, `text`, in a box over the brush at (cx, cy), `r` css px (under it where there is no room
 * above), its words wrapped at `LABEL_PX`: a finger on the brush does not hide it. */
export function drawReason(ctx: CanvasRenderingContext2D, cx: number, cy: number, r: number, text: string): void {
  ctx.save();
  ctx.font = '12px system-ui, sans-serif';
  const lines: string[] = [];
  for (const word of text.split(' ')) {
    const last = lines.length - 1;
    if (last >= 0 && ctx.measureText(`${lines[last]} ${word}`).width <= LABEL_PX) lines[last] += ` ${word}`;
    else lines.push(word);
  }
  const w = Math.max(...lines.map(l => ctx.measureText(l).width)) + 16, h = lines.length * 16 + 10;
  const W = ctx.canvas.clientWidth || ctx.canvas.width;
  const bx = Math.min(Math.max(cx - w / 2, 4), W - w - 4), above = cy - r - LABEL_GAP_PX - h, by = above >= 4 ? above : cy + r + LABEL_GAP_PX;
  ctx.fillStyle = 'rgba(35,39,43,.9)';
  ctx.beginPath(); ctx.roundRect(bx, by, w, h, 6); ctx.fill();
  ctx.fillStyle = '#fff';
  lines.forEach((l, k) => ctx.fillText(l, bx + 8, by + 18 + 16 * k));
  ctx.restore();
}
