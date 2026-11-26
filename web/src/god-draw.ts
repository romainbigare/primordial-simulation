// What god mode draws over the map (god.ts `draw`): the round brush at its true size, the sight on a phone, the heat
// it holds as a ring that fills and as a fire that grows in it, a lake's water as it is dug, the brush's size beside it,
// and the reason a tool cannot act there. (A burn's fire once it spreads is the map's own: fire.wgsl, burns.ts.)

/** The width of the ring that shows the heat held under the brush, css px. */
const PROGRESS_PX = 5;
/** A dug lake's water fills its ring over this long, ms, as strong as this; then a ripple runs out every this long, ms. */
const FILL_MS = 450, FILL_ALPHA = 0.55, RIPPLE_MS = 1100;
/** The widest line of the box that says why a tool cannot act, css px, and its gap to the brush. */
const LABEL_PX = 220, LABEL_GAP_PX = 10;
/** How strong a planting's trail is drawn (`drawTrail`). */
const TRAIL_ALPHA = 0.35;
/** The sight's arms reach this far from its centre, from this gap, css px. */
const SIGHT_PX = 11, SIGHT_GAP_PX = 4;
/** The angle between one flame and the next of a patch, radians: the golden angle, so they spread evenly over it. */
const GOLDEN_ANGLE = Math.PI * (3 - Math.sqrt(5));

/** The brush at (cx, cy), `r` css px: a disc of `ink` (`held`: while a stroke is under way, stronger; `rim`: none,
 * its rim alone), its rim, its centre; the heat it holds (`progress`, 0 to 1; less than 0: none asked) as a ring that
 * fills. */
export function drawBrush(ctx: CanvasRenderingContext2D, cx: number, cy: number, r: number, ink: string, held: boolean, rim: boolean, progress: number): void {
  ctx.save();
  ctx.beginPath(); ctx.arc(cx, cy, r, 0, 2 * Math.PI);
  ctx.globalAlpha = rim ? 0 : held ? 0.3 : 0.15; ctx.fillStyle = ink; ctx.fill();
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

/** The fire in the brush while it heats burns in patches at fixed places on the ground: the brush holds about
 * `PATCH_SITES` places, one in each square of a grid fixed to the map (moved within it by its own draw); a place lights
 * once the heat it holds passes its own draw, the heat falling from the brush's centre as the core adds it, by
 * (1 − (d/r)²)² (the core's `World::heat`), so the patches light from the middle out and none moves. A patch's flames
 * grow to their height over `GROW` of the heat past its lighting; they are as tall as this share of the brush's radius,
 * between these heights (css px); each holds this many flames, packed within this many of their heights of its centre.
 * Sparks and puffs of smoke rise from them, and the ground under each is scorched about it. */
const PATCH_SITES = 30, GROW = 0.3, FLAME_OF_RADIUS = 0.3, FLAME_LEAST_PX = 10, FLAME_MOST_PX = 34;
const PATCH_FLAMES = 7, PATCH_SPREAD = 0.55, FLAME_WIDTH = 0.55, SPARKS = 3, PUFFS = 1;
/** A place's draw is under the heat it holds times `LIGHT_GAIN` (so the middle lights soon, the rim only as the
 * ground lights). */
const LIGHT_GAIN = 1.4;
/** The scorched ground under a patch reaches this many flames' heights about it, as dark as this. */
const SCORCH_REACH = 1.6, SCORCH_ALPHA = 0.45;
/** A flame's colours from its foot to its tip: its glowing edge, its body, and its hot core; the light it casts on the
 * ground; the embers; the smoke. */
const EDGE_STOPS: [number, string][] = [[0, 'rgba(200,50,10,.35)'], [0.6, 'rgba(160,30,5,.18)'], [1, 'rgba(90,15,5,0)']];
const BODY_STOPS: [number, string][] = [[0, 'rgba(255,150,40,.9)'], [0.4, 'rgba(240,96,22,.75)'], [0.8, 'rgba(180,40,10,.3)'], [1, 'rgba(110,20,5,0)']];
const CORE_STOPS: [number, string][] = [[0, 'rgba(255,246,200,.9)'], [0.45, 'rgba(255,200,80,.55)'], [1, 'rgba(255,150,40,0)']];
const LIGHT = [255, 120, 30], EMBER = [255, 90, 20], SMOKE = [58, 54, 52], SCORCH = [30, 18, 10];
/** The smoke drifts this way across the screen as it rises, a share of its rise (east: a steady breeze). */
const DRIFT = 0.6;

/** A wavering number of `k` at time `t` (s), about -1 to 1: a sum of waves of unlike speeds, so it never repeats soon. */
const waver = (k: number, t: number) => 0.5 * Math.sin(2.1 * t + 1.7 * k) + 0.3 * Math.sin(5.3 * t + 0.9 * k) + 0.2 * Math.sin(11.7 * t + 2.3 * k);
/** The fractional part of `x`. */
const frac = (x: number) => x - Math.floor(x);
/** A draw 0..1 of the grid square (x, y) and the stream k, the same every time. */
function draw01(x: number, y: number, k: number): number {
  let h = Math.imul(x | 0, 0x27d4eb2d) ^ Math.imul(y | 0, 0x165667b1) ^ Math.imul(k, 0x9e3779b9);
  h = Math.imul(h ^ (h >>> 15), 0x85ebca6b); h = Math.imul(h ^ (h >>> 13), 0xc2b2ae35);
  return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
}

/** Where the map lies on the screen: the css px of its north-west corner, and css px a metre. */
export interface MapAt { ox: number; oy: number; k: number }

/** A place in a grid fixed to the map where the brush's heat has lit the ground (`litSites`): where, m, how far past
 * its lighting it is, and its own number (its flames' draws). */
export interface Lit { x: number; y: number; past: number; k: number }

/** The places in the brush at `at` (m) of radius `radiusM` (m) that the heat it holds, `progress` (0 to 1) of what
 * lights the ground under it, has lit (`drawFlames`), the most lit first. */
export function litSites(at: [number, number], radiusM: number, progress: number): Lit[] {
  const p = Math.min(Math.max(progress, 0), 1), step = (2 * radiusM) / Math.sqrt((4 / Math.PI) * PATCH_SITES), out: Lit[] = [];
  for (let gy = Math.floor((at[1] - radiusM) / step); gy * step <= at[1] + radiusM; gy++) {
    for (let gx = Math.floor((at[0] - radiusM) / step); gx * step <= at[0] + radiusM; gx++) {
      const x = (gx + draw01(gx, gy, 1)) * step, y = (gy + draw01(gx, gy, 2)) * step, d = Math.hypot(x - at[0], y - at[1]) / radiusM;
      if (d >= 1) continue;
      const past = LIGHT_GAIN * p * (1 - d * d) ** 2 - draw01(gx, gy, 3);
      if (past > 0) out.push({ x, y, past, k: (gx * 31 + gy * 17) & 1023 });
    }
  }
  return out.sort((a, b) => b.past - a.past);
}

/** The fire in the brush at `at` (m) of radius `radiusM` (m) on the map lying as `map` says, as much of the heat that
 * lights the ground under it held as `progress` (0 to 1), at time `now` (ms): patches of it burning at fixed places on
 * the ground, more as it nears lighting, each scorching the ground about it, a light cast on it, embers, a cluster of
 * flames that sway and flicker in soft layers, sparks, and smoke drifting off. A picture of the heat held, not the
 * core's flames: those play when the stroke settles (burns.ts). */
export function drawFlames(ctx: CanvasRenderingContext2D, map: MapAt, at: [number, number], radiusM: number, progress: number, now: number): void {
  const t = now / 1000, r = radiusM * map.k, cx = map.ox + at[0] * map.k, cy = map.oy + at[1] * map.k;
  const full = Math.min(Math.max(r * FLAME_OF_RADIUS, FLAME_LEAST_PX), FLAME_MOST_PX);
  // The places the heat has lit, on the screen, their flames as tall as they are past lighting.
  const patches = litSites(at, radiusM, progress).map(s => ({ x: map.ox + s.x * map.k, y: map.oy + s.y * map.k, k: s.k, tall: full * (0.55 + 0.45 * Math.min(s.past / GROW, 1)) }));
  ctx.save();
  // The ground scorched about each patch, the light each casts on it, and its embers.
  ctx.beginPath(); ctx.arc(cx, cy, r, 0, 2 * Math.PI); ctx.clip();
  const sp = sprites();
  for (const { x, y, tall } of patches) glow(ctx, sp.scorch, x, y, SCORCH_REACH * tall, 1);
  ctx.globalCompositeOperation = 'lighter';
  for (const { x, y, k, tall } of patches) {
    glow(ctx, sp.light, x, y, tall * (2 + 0.3 * waver(k, t)), 1);
    for (let e = 0; e < 2 * PATCH_FLAMES; e++) {
      const [ex, ey] = scatter(x, y, tall * 1.4, e + 3, 2 * PATCH_FLAMES + 3), glow = 0.5 + 0.5 * waver(e + 5 * k, 1.5 * t);
      ctx.fillStyle = `rgba(${EMBER.join()},${0.6 * glow})`;
      ctx.beginPath(); ctx.arc(ex, ey, 0.8 + glow, 0, 2 * Math.PI); ctx.fill();
    }
  }
  ctx.restore();
  ctx.save();
  // The flames, the farthest (highest on the screen) first, so the nearer stand in front.
  const flames = patches.flatMap(({ x, y, k, tall }) => Array.from({ length: PATCH_FLAMES }, (_, f) => {
    const [fx, fy] = scatter(x, y, tall * PATCH_SPREAD, f, PATCH_FLAMES), id = k * PATCH_FLAMES + f;
    return { x: fx, y: fy + tall * 0.3, h: tall * (0.55 + 0.45 * frac(id * 0.618)) * (0.8 + 0.2 * waver(id, 1.4 * t)), id };
  })).sort((a, b) => a.y - b.y);
  const base = ctx.getTransform();
  for (const { x, y, h, id } of flames) {
    const w = h * FLAME_WIDTH, sway = w * 0.45 * waver(id + 11, 1.2 * t);
    flame(ctx, base, sp.edge, x, y, 1.7 * w, 1.15 * h, 1.2 * sway, 'source-over');
    flame(ctx, base, sp.body, x, y, w, h, sway, 'source-over');
    // (A second, smaller tongue beside it, on its own sway: a flame's edge is never smooth.)
    flame(ctx, base, sp.body, x + 0.3 * w * Math.sign(waver(id + 3, 0) || 1), y, 0.6 * w, 0.65 * h, w * 0.5 * waver(id + 23, 1.7 * t), 'source-over');
    flame(ctx, base, sp.core, x, y + 0.05 * h, 0.45 * w, 0.45 * h, 0.5 * sway, 'lighter');
  }
  ctx.setTransform(base);
  // Sparks rising from the patches, fading as they go; and the smoke drifting off them.
  ctx.globalCompositeOperation = 'lighter';
  for (const { x, y, k, tall } of patches) {
    for (let f = 0; f < SPARKS; f++) {
      const age = frac(0.6 * t + (k * SPARKS + f) * 0.377), sx = x + tall * (0.6 * waver(f + k, t) + DRIFT * age), sy = y - 3 * tall * age;
      ctx.fillStyle = `rgba(255,${210 - 130 * age},70,${1 - age})`;
      ctx.beginPath(); ctx.arc(sx, sy, 1.2, 0, 2 * Math.PI); ctx.fill();
    }
  }
  ctx.globalCompositeOperation = 'source-over';
  for (const { x, y, k, tall } of patches) {
    for (let f = 0; f < PUFFS; f++) {
      const age = frac(0.22 * t + (k + f) * 0.31), rise = tall * (1.2 + 3 * age), size = tall * (0.8 + 1.8 * age);
      glow(ctx, sp.puff, x + DRIFT * rise, y - rise, size, 0.3 * Math.sin(Math.PI * age));
    }
  }
  ctx.restore();
}

/** The pictures the brush's fire is stamped from (`drawFlames`), drawn once: a tongue of flame of each of its colour
 * layers (`FLAME_SPRITE_W` wide at its foot, `FLAME_SPRITE_H` tall, upright), and a disc fading from its middle to its
 * edge of each glow (`GLOW_SPRITE_R` in radius: the scorch, the light, a puff of smoke at full strength). So a frame
 * draws a few hundred images, not a gradient for each. */
const FLAME_SPRITE_W = 64, FLAME_SPRITE_H = 96, GLOW_SPRITE_R = 128;
/** The room about a tongue's picture, px: its foot's curve reaches below its foot by a quarter of its width. */
const FLAME_SPRITE_ROOM = 0.25 * FLAME_SPRITE_W;
let made: Record<'edge' | 'body' | 'core' | 'scorch' | 'light' | 'puff', HTMLCanvasElement> | null = null;
function sprites(): NonNullable<typeof made> {
  if (made) return made;
  const canvas = (w: number, h: number) => { const c = document.createElement('canvas'); c.width = w; c.height = h; return c; };
  const tongue = (stops: [number, string][]) => {
    const c = canvas(FLAME_SPRITE_W + 2 * FLAME_SPRITE_ROOM, FLAME_SPRITE_H + 2 * FLAME_SPRITE_ROOM), g = c.getContext('2d')!;
    const x = c.width / 2, y = FLAME_SPRITE_ROOM + FLAME_SPRITE_H, w = FLAME_SPRITE_W, h = FLAME_SPRITE_H, fill = g.createLinearGradient(x, y, x, y - h);
    for (const [at, colour] of stops) fill.addColorStop(at, colour);
    g.fillStyle = fill;
    g.beginPath();
    g.moveTo(x - w / 2, y);
    g.bezierCurveTo(x - w / 2, y - 0.45 * h, x - 0.15 * w, y - 0.7 * h, x, y - h);
    g.bezierCurveTo(x + 0.15 * w, y - 0.7 * h, x + w / 2, y - 0.45 * h, x + w / 2, y);
    g.quadraticCurveTo(x, y + 0.25 * w, x - w / 2, y);
    g.fill();
    return c;
  };
  const disc = (rgb: number[], alpha: number) => {
    const c = canvas(2 * GLOW_SPRITE_R, 2 * GLOW_SPRITE_R), g = c.getContext('2d')!, r = GLOW_SPRITE_R, fill = g.createRadialGradient(r, r, 0, r, r, r);
    fill.addColorStop(0, `rgba(${rgb.join()},${alpha})`); fill.addColorStop(1, `rgba(${rgb.join()},0)`);
    g.fillStyle = fill;
    g.beginPath(); g.arc(r, r, r, 0, 2 * Math.PI); g.fill();
    return c;
  };
  return (made = { edge: tongue(EDGE_STOPS), body: tongue(BODY_STOPS), core: tongue(CORE_STOPS), scorch: disc(SCORCH, SCORCH_ALPHA), light: disc(LIGHT, 0.45), puff: disc(SMOKE, 1) });
}

/** Stamp the glow `sprite` about (x, y), `reach` css px in radius, as strong as `alpha`. */
function glow(ctx: CanvasRenderingContext2D, sprite: HTMLCanvasElement, x: number, y: number, reach: number, alpha: number): void {
  ctx.globalAlpha = alpha;
  ctx.drawImage(sprite, x - reach, y - reach, 2 * reach, 2 * reach);
  ctx.globalAlpha = 1;
}

/** Point `k` of `of` scattered evenly over the disc of radius `r` about (cx, cy) (each a turn of the golden angle
 * farther round, and out by the root of its rank). */
function scatter(cx: number, cy: number, r: number, k: number, of: number): [number, number] {
  const a = k * GOLDEN_ANGLE, d = r * Math.sqrt((k + 0.5) / of);
  return [cx + d * Math.cos(a), cy + d * Math.sin(a)];
}

/** A tongue of flame of the picture `sprite` (`sprites`) standing at (x, y), `w` wide at its foot and `h` tall, its tip
 * moved `sway` to the side (a shear: each height moved by its share of the sway), by the composite `op`, on the canvas
 * whose transform is `base`. */
function flame(ctx: CanvasRenderingContext2D, base: DOMMatrix, sprite: HTMLCanvasElement, x: number, y: number, w: number, h: number, sway: number, op: GlobalCompositeOperation): void {
  // The picture's foot to (x, y): across by w over its width, up by h over its height, sheared by the sway.
  const a = w / FLAME_SPRITE_W, c = -sway / FLAME_SPRITE_H, d = h / FLAME_SPRITE_H;
  ctx.setTransform(base.a * a, base.b * a, base.a * c + base.c * d, base.b * c + base.d * d, base.a * x + base.c * y + base.e, base.b * x + base.d * y + base.f);
  ctx.globalCompositeOperation = op;
  ctx.drawImage(sprite, -sprite.width / 2, -(FLAME_SPRITE_ROOM + FLAME_SPRITE_H));
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

/** The trail of a stroke whose making waits for its end (a planting): a disc of `ink` at each of `at` (css px) of its
 * radius `r` (css px), drawn as one shape, so where they overlap it is no stronger. */
export function drawTrail(ctx: CanvasRenderingContext2D, at: [number, number][], r: number[], ink: string): void {
  ctx.save();
  ctx.beginPath();
  at.forEach(([x, y], k) => { ctx.moveTo(x + r[k], y); ctx.arc(x, y, r[k], 0, 2 * Math.PI); });
  ctx.globalAlpha = TRAIL_ALPHA; ctx.fillStyle = ink; ctx.fill('nonzero');
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
