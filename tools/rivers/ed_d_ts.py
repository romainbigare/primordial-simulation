def edit(p, pairs):
    t = open(p).read()
    for a, b in pairs:
        assert t.count(a) == 1, (p, a[:70], t.count(a))
        t = t.replace(a, b)
    open(p, 'w').write(t)

edit('web/src/land.ts', [
 ("""import { half } from './canopy';
""", """import { half, unhalf } from './canopy';
"""),
 ("""const SEA_BLUR = 1.5, SEA_REACH = 4, OPEN_SEA_M = 20;
""", """const SEA_BLUR = 1.5, SEA_REACH = 4, OPEN_SEA_M = 20;
/** The signs of the flow on a channel's water slide along it (lib/flow.wgsl): they step this many times a second, the
 * slowest water's by this many metres a second; they show where a pixel takes no more than `FLOW_SHOWS_E` m (the
 * shader's streaks fade out there) on water at least `FLOW_PX` pixels wide. */
export const FLOW_FPS = 6;
const FLOW_M_S = 0.35, FLOW_SHOWS_E = 0.6, FLOW_PX = 4;
"""),
 ("""  /** The bindings of lib/land.wgsl on `gpu` (its uniforms, the tiles and the table), or null before the first table. */""",
  """  /** How far the flow's signs have slid along the slowest water at `now` (performance.now, ms), m: in steps of
   * 1 / `FLOW_FPS` s, modulo the length a place along a channel is kept modulo (the signs come again after it, so the
   * slide has no jump). */
  private slid(now: number): number {
    return ((Math.floor((now * FLOW_FPS) / 1000) / FLOW_FPS) * FLOW_M_S) % this.tiles.meta.along_wrap;
  }

  /** Whether a channel in the window of squares from (`x`, `y`), `w` x `h` of them, shows the signs of its flow where a
   * pixel takes `e` m: one of the samples there carries water at least `FLOW_PX` pixels wide (the bed's width by the
   * channel law at the flood's discharge; the water fills most of it). */
  flowing(x: number, y: number, w: number, h: number, e: number): boolean {
    if (e > FLOW_SHOWS_E) return false;
    const s = this.samples, per = s.across / this.tiles.map.w, l = this.laws;
    // The discharge whose bed is `FLOW_PX` pixels wide.
    const least = (FLOW_PX * e / l.channel_width) ** (1 / l.channel_width_exp) / l.flood;
    const x0 = Math.max(0, Math.floor(x * per)), x1 = Math.min(s.across - 1, Math.ceil((x + w) * per));
    const y0 = Math.max(0, Math.floor(y * per)), y1 = Math.min(s.down - 1, Math.ceil((y + h) * per));
    for (let r = y0; r <= y1; r++) for (let c = x0; c <= x1; c++) if (unhalf(this.discharge[r * s.across + c]) >= least) return true;
    return false;
  }

  /** The bindings of lib/land.wgsl on `gpu` (its uniforms, the tiles and the table), or null before the first table. */"""),
 (""".vec4(s.across, s.down, s.depth, devicePixelRatio).vec4(tm.along_wrap, l.flood);""", """.vec4(s.across, s.down, s.depth, devicePixelRatio).vec4(tm.along_wrap, l.flood, this.slid(performance.now()));"""),
])
edit('web/src/map.ts', [
 ("""  /** Whether the GPU's picture is to be drawn anew at the next frame. */
  private gpuDue = false;""", """  /** Whether the GPU's picture is to be drawn anew at the next frame. */
  private gpuDue = false;
  /** The timer of the next frame of the flow's signs (`flowSoon`). */
  private flowTimer = 0;"""),
 ("""    gpu.submit();
    this.time(gpu, r);
    if (r < 1 && !fading) this.sharpTimer = window.setTimeout(() => this.sharpenSoon(xs, d), REST_MS);
  }
""", """    gpu.submit();
    this.time(gpu, r);
    if (r < 1 && !fading) this.sharpTimer = window.setTimeout(() => this.sharpenSoon(xs, d), REST_MS);
    this.flowSoon();
  }

  /** Draw the map again when the signs of the flow next step along the channels (land.ts `FLOW_FPS`), while a channel
   * in view shows them, the page is seen, and the map is drawn at the screen's own pixels (a card that cannot keep up
   * shows them still: the sharp picture is not drawn over and over). Else no frame is drawn until something changes. */
  private flowSoon(): void {
    clearTimeout(this.flowTimer);
    const v = this.view();
    if (document.hidden || this.scale < 1 || !this.land?.flowing(v.x, v.y, v.w, v.h, this.cellM / (v.s * this.dpr()))) return;
    const step = 1000 / FLOW_FPS;
    this.flowTimer = window.setTimeout(() => this.invalidate(), step - (performance.now() % step) + 1);
  }
"""),
])
t = open('web/src/map.ts').read()
import re
m = re.search(r"import \{[^}]*\} from './land';", t)
print('land import:', m.group(0) if m else None)
