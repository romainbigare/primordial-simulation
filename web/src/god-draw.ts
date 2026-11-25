// What god mode draws over the map (god.ts `draw`): the round brush at its true size, the sight on a phone, the heat
// it holds as a ring that fills and as a fire that grows in it, a lake's water as it is dug, the brush's size beside it,
// the reason a tool cannot act there, and the burning edge of a burn as it spreads (`drawFireEdge`).

/** The width of the ring that shows the heat held under the brush, css px. */
const PROGRESS_PX = 5;
/** A dug lake's water fills its ring over this long, ms, as strong as this; then a ripple runs out every this long, ms. */
const FILL_MS = 450, FILL_ALPHA = 0.55, RIPPLE_MS = 1100;
/** The widest line of the box that says why a tool cannot act, css px, and its gap to the brush. */
const LABEL_PX = 220, LABEL_GAP_PX = 10;
/** The sight's arms reach this far from its centre, from this gap, css px. */
const SIGHT_PX = 11, SIGHT_GAP_PX = 4;
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

/** The fire in the brush while it heats burns in patches: its flames are as tall as this share of the brush's radius,
 * between these heights (css px), less while the heat is low; a patch takes about this many flames' heights squared of
 * the brush, between these counts of patches, a share of them burning that grows with the heat; and each holds this many
 * flames, packed within this many of their heights of its centre. Sparks and puffs of smoke rise from them. */
const FLAME_OF_RADIUS = 0.3, FLAME_LEAST_PX = 10, FLAME_MOST_PX = 34, PATCH_AREA = 7, PATCHES_LEAST = 3, PATCHES_MOST = 26;
const PATCH_FLAMES = 7, PATCH_SPREAD = 0.55, FLAME_WIDTH = 0.55, SPARKS = 3, PUFFS = 1;
/** A flame's colours from its foot to its tip: its glowing edge, its body, and its hot core; the light it casts on the
 * ground; the embers; the smoke. */
const EDGE_STOPS: [number, string][] = [[0, 'rgba(200,50,10,.35)'], [0.6, 'rgba(160,30,5,.18)'], [1, 'rgba(90,15,5,0)']];
const BODY_STOPS: [number, string][] = [[0, 'rgba(255,150,40,.9)'], [0.4, 'rgba(240,96,22,.75)'], [0.8, 'rgba(180,40,10,.3)'], [1, 'rgba(110,20,5,0)']];
const CORE_STOPS: [number, string][] = [[0, 'rgba(255,246,200,.9)'], [0.45, 'rgba(255,200,80,.55)'], [1, 'rgba(255,150,40,0)']];
const LIGHT = [255, 120, 30], EMBER = [255, 90, 20], SMOKE = [58, 54, 52];
/** The smoke drifts this way across the screen as it rises, a share of its rise (east: a steady breeze). */
const DRIFT = 0.6;

/** A wavering number of `k` at time `t` (s), about -1 to 1: a sum of waves of unlike speeds, so it never repeats soon. */
const waver = (k: number, t: number) => 0.5 * Math.sin(2.1 * t + 1.7 * k) + 0.3 * Math.sin(5.3 * t + 0.9 * k) + 0.2 * Math.sin(11.7 * t + 2.3 * k);
/** The fractional part of `x`. */
const frac = (x: number) => x - Math.floor(x);

/** The fire in the brush at (cx, cy), `r` css px, as much of the heat that lights the ground under it held as
 * `progress` (0 to 1), at time `now` (ms): patches of it burning on the scorched ground, more as it nears lighting, each
 * a light cast on the ground, embers, a cluster of flames that sway and flicker in soft layers, sparks, and smoke
 * drifting off. A picture of the heat held, not the core's flames: those play when the stroke settles (fire.ts `play`). */
export function drawFlames(ctx: CanvasRenderingContext2D, cx: number, cy: number, r: number, progress: number, now: number): void {
  const p = Math.min(Math.max(progress, 0), 1), t = now / 1000;
  const tall = Math.min(Math.max(r * FLAME_OF_RADIUS, FLAME_LEAST_PX), FLAME_MOST_PX) * (0.55 + 0.45 * p);
  const all = Math.min(Math.max(Math.round((r * r) / (PATCH_AREA * tall * tall)), PATCHES_LEAST), PATCHES_MOST);
  const patches = Array.from({ length: Math.ceil(all * (0.25 + 0.75 * p)) }, (_, k) => scatter(cx, cy, r * 0.8, k, all));
  ctx.save();
  // The ground scorched inside the brush, and the light each patch casts on it, with its embers.
  ctx.beginPath(); ctx.arc(cx, cy, r, 0, 2 * Math.PI); ctx.clip();
  ctx.fillStyle = `rgba(30,18,10,${0.35 * p})`; ctx.fillRect(cx - r, cy - r, 2 * r, 2 * r);
  ctx.globalCompositeOperation = 'lighter';
  patches.forEach(([x, y], k) => {
    const reach = tall * (2 + 0.3 * waver(k, t)), light = ctx.createRadialGradient(x, y, 0, x, y, reach);
    light.addColorStop(0, `rgba(${LIGHT.join()},${0.45 * p})`); light.addColorStop(1, `rgba(${LIGHT.join()},0)`);
    ctx.fillStyle = light;
    ctx.beginPath(); ctx.arc(x, y, reach, 0, 2 * Math.PI); ctx.fill();
    for (let e = 0; e < 2 * PATCH_FLAMES; e++) {
      const [ex, ey] = scatter(x, y, tall * 1.4, e + 3, 2 * PATCH_FLAMES + 3), glow = 0.5 + 0.5 * waver(e + 5 * k, 1.5 * t);
      ctx.fillStyle = `rgba(${EMBER.join()},${0.6 * glow * p})`;
      ctx.beginPath(); ctx.arc(ex, ey, 0.8 + glow, 0, 2 * Math.PI); ctx.fill();
    }
  });
  ctx.restore();
  ctx.save();
  // The flames, the farthest (highest on the screen) first, so the nearer stand in front.
  const flames = patches.flatMap(([x, y], k) => Array.from({ length: PATCH_FLAMES }, (_, f) => {
    const [fx, fy] = scatter(x, y, tall * PATCH_SPREAD, f, PATCH_FLAMES), id = k * PATCH_FLAMES + f;
    return { x: fx, y: fy + tall * 0.3, h: tall * (0.55 + 0.45 * frac(id * 0.618)) * (0.8 + 0.2 * waver(id, 1.4 * t)), id };
  })).sort((a, b) => a.y - b.y);
  for (const { x, y, h, id } of flames) {
    const w = h * FLAME_WIDTH, sway = w * 0.45 * waver(id + 11, 1.2 * t);
    flame(ctx, x, y, 1.7 * w, 1.15 * h, 1.2 * sway, EDGE_STOPS, 'source-over');
    flame(ctx, x, y, w, h, sway, BODY_STOPS, 'source-over');
    // (A second, smaller tongue beside it, on its own sway: a flame's edge is never smooth.)
    flame(ctx, x + 0.3 * w * Math.sign(waver(id + 3, 0) || 1), y, 0.6 * w, 0.65 * h, w * 0.5 * waver(id + 23, 1.7 * t), BODY_STOPS, 'source-over');
    flame(ctx, x, y + 0.05 * h, 0.45 * w, 0.45 * h, 0.5 * sway, CORE_STOPS, 'lighter');
  }
  // Sparks rising from the patches, fading as they go; and the smoke drifting off them.
  ctx.globalCompositeOperation = 'lighter';
  patches.forEach(([x, y], k) => {
    for (let f = 0; f < SPARKS; f++) {
      const age = frac(0.6 * t + (k * SPARKS + f) * 0.377), sx = x + tall * (0.6 * waver(f + k, t) + DRIFT * age), sy = y - 3 * tall * age;
      ctx.fillStyle = `rgba(255,${210 - 130 * age},70,${(1 - age) * p})`;
      ctx.beginPath(); ctx.arc(sx, sy, 1.2, 0, 2 * Math.PI); ctx.fill();
    }
  });
  ctx.globalCompositeOperation = 'source-over';
  patches.forEach(([x, y], k) => {
    for (let f = 0; f < PUFFS; f++) {
      const age = frac(0.22 * t + (k + f) * 0.31), rise = tall * (1.2 + 3 * age), size = tall * (0.8 + 1.8 * age);
      const px = x + DRIFT * rise, py = y - rise, puff = ctx.createRadialGradient(px, py, 0, px, py, size);
      puff.addColorStop(0, `rgba(${SMOKE.join()},${0.3 * Math.sin(Math.PI * age) * p})`); puff.addColorStop(1, `rgba(${SMOKE.join()},0)`);
      ctx.fillStyle = puff;
      ctx.beginPath(); ctx.arc(px, py, size, 0, 2 * Math.PI); ctx.fill();
    }
  });
  ctx.restore();
}

/** The edge of a burn spreading on the map (fire.ts `edgeAt`): tongues of flame this far apart along it (css px), as
 * tall as this (css px), standing out of the burnt ground to this share of the edge's raggedness either way; a glow
 * along it this many times the tongues' height wide. */
const EDGE_STEP_PX = 7, EDGE_FLAME_PX = 15, EDGE_JITTER = 0.6, EDGE_GLOW = 1.2;
/** Its tongues and their glow drawn at most this many around (a large burn's ring seen close takes no more). */
const EDGE_FLAMES_MOST = 220;

/** The burning edge of a burn spreading from (cx, cy), `reach` css px from it and ragged over `edge` css px, at time
 * `now` (ms): a ring of glow and of tongues of flame that sway and flicker, with sparks and smoke rising off it. The
 * scar inside it, and the map's own flames, are the map's (fire.wgsl); these stand on its edge as the fire's light seen
 * at any zoom. */
export function drawFireEdge(ctx: CanvasRenderingContext2D, cx: number, cy: number, reach: number, edge: number, now: number): void {
  if (reach < 1) return;
  const t = now / 1000, n = Math.min(Math.max(Math.round((2 * Math.PI * reach) / EDGE_STEP_PX), 6), EDGE_FLAMES_MOST);
  const tall = Math.min(EDGE_FLAME_PX, Math.max(reach, FLAME_LEAST_PX));
  const along = Array.from({ length: n }, (_, k) => {
    const a = ((k + 0.5 * frac(k * 0.618)) / n) * 2 * Math.PI, d = reach + EDGE_JITTER * edge * waver(k, 0.15 * t);
    return { x: cx + d * Math.cos(a), y: cy + d * Math.sin(a), k };
  });
  ctx.save();
  ctx.globalCompositeOperation = 'lighter';
  for (const { x, y, k } of along) {
    const reachOf = tall * EDGE_GLOW * (0.8 + 0.2 * waver(k + 9, t)), light = ctx.createRadialGradient(x, y, 0, x, y, reachOf);
    light.addColorStop(0, `rgba(${LIGHT.join()},.35)`); light.addColorStop(1, `rgba(${LIGHT.join()},0)`);
    ctx.fillStyle = light;
    ctx.beginPath(); ctx.arc(x, y, reachOf, 0, 2 * Math.PI); ctx.fill();
  }
  for (const { x, y, k } of along.sort((p, q) => p.y - q.y)) {
    const h = tall * (0.55 + 0.45 * frac(k * 0.618)) * (0.8 + 0.2 * waver(k, 1.4 * t)), w = h * FLAME_WIDTH, sway = w * 0.45 * waver(k + 11, 1.2 * t);
    flame(ctx, x, y, 1.7 * w, 1.15 * h, 1.2 * sway, EDGE_STOPS, 'source-over');
    flame(ctx, x, y, w, h, sway, BODY_STOPS, 'source-over');
    flame(ctx, x, y + 0.05 * h, 0.45 * w, 0.45 * h, 0.5 * sway, CORE_STOPS, 'lighter');
    if (k % 3) continue;
    const age = frac(0.6 * t + k * 0.377);
    ctx.globalCompositeOperation = 'lighter';
    ctx.fillStyle = `rgba(255,${210 - 130 * age},70,${1 - age})`;
    ctx.beginPath(); ctx.arc(x + tall * (0.5 * waver(k, t) + DRIFT * age), y - 3 * tall * age, 1.2, 0, 2 * Math.PI); ctx.fill();
  }
  ctx.globalCompositeOperation = 'source-over';
  for (const { x, y, k } of along) {
    if (k % 4) continue;
    const age = frac(0.22 * t + k * 0.31), rise = tall * (1.2 + 3 * age), size = tall * (0.8 + 1.8 * age), px = x + DRIFT * rise, py = y - rise;
    const puff = ctx.createRadialGradient(px, py, 0, px, py, size);
    puff.addColorStop(0, `rgba(${SMOKE.join()},${0.25 * Math.sin(Math.PI * age)})`); puff.addColorStop(1, `rgba(${SMOKE.join()},0)`);
    ctx.fillStyle = puff;
    ctx.beginPath(); ctx.arc(px, py, size, 0, 2 * Math.PI); ctx.fill();
  }
  ctx.restore();
}

/** Point `k` of `of` scattered evenly over the disc of radius `r` about (cx, cy) (each a turn of the golden angle
 * farther round, and out by the root of its rank). */
function scatter(cx: number, cy: number, r: number, k: number, of: number): [number, number] {
  const a = k * GOLDEN_ANGLE, d = r * Math.sqrt((k + 0.5) / of);
  return [cx + d * Math.cos(a), cy + d * Math.sin(a)];
}

/** A tongue of flame standing at (x, y), `w` wide at its foot and `h` tall, its tip moved `sway` to the side, filled
 * from its foot to its tip with the colours `stops` by the composite `op`. */
function flame(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, sway: number, stops: [number, string][], op: GlobalCompositeOperation): void {
  const g = ctx.createLinearGradient(x, y, x + sway, y - h);
  for (const [at, colour] of stops) g.addColorStop(at, colour);
  ctx.globalCompositeOperation = op; ctx.fillStyle = g;
  ctx.beginPath();
  ctx.moveTo(x - w / 2, y);
  ctx.bezierCurveTo(x - w / 2, y - 0.45 * h, x + sway - 0.15 * w, y - 0.7 * h, x + sway, y - h);
  ctx.bezierCurveTo(x + sway + 0.15 * w, y - 0.7 * h, x + w / 2, y - 0.45 * h, x + w / 2, y);
  ctx.quadraticCurveTo(x, y + 0.25 * w, x - w / 2, y);
  ctx.fill();
}

/** The water of a lake dug, at (cx, cy) in a ring of `r` css px, `ms` after the tap: it rises from the centre to the
 * ring over `FILL_MS`, then ripples softly while the world builds its ground (the map shows the lake once its tiles
 * come). */
export function drawFill(ctx: CanvasRenderingContext2D, cx: number, cy: number, r: number, ink: string, ms: number): void {
  const t = Math.min(ms / FILL_MS, 1), grown = r * (1 - (1 - t) ** 3);
  ctx.save();
  ctx.beginPath(); ctx.arc(cx, cy, grown, 0, 2 * Math.PI);
  ctx.globalAlpha = FILL_ALPHA; ctx.fillStyle = ink; ctx.fill();
  if (t >= 1) {
    const ripple = (ms / RIPPLE_MS) % 1;
    ctx.globalAlpha = FILL_ALPHA * (1 - ripple); ctx.lineWidth = 2; ctx.strokeStyle = '#fff';
    ctx.beginPath(); ctx.arc(cx, cy, r * ripple, 0, 2 * Math.PI); ctx.stroke();
  }
  ctx.restore();
}

/** The sight at (cx, cy): a small white cross with a dark edge, where the brush acts on a phone. */
export function drawSight(ctx: CanvasRenderingContext2D, cx: number, cy: number): void {
  ctx.save();
  ctx.lineCap = 'round';
  for (const [width, colour] of [[4, 'rgba(0,0,0,.5)'], [2, '#fff']] as const) {
    ctx.lineWidth = width; ctx.strokeStyle = colour;
    ctx.beginPath();
    ctx.moveTo(cx - SIGHT_PX, cy); ctx.lineTo(cx - SIGHT_GAP_PX, cy); ctx.moveTo(cx + SIGHT_GAP_PX, cy); ctx.lineTo(cx + SIGHT_PX, cy);
    ctx.moveTo(cx, cy - SIGHT_PX); ctx.lineTo(cx, cy - SIGHT_GAP_PX); ctx.moveTo(cx, cy + SIGHT_GAP_PX); ctx.lineTo(cx, cy + SIGHT_PX);
    ctx.stroke();
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
