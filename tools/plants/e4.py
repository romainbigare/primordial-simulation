W = '/home/user/primordial-simulation/.claude/worktrees/agent-a257b588d76bf8a62/'
def edit(path, pairs):
    s = open(W + path).read()
    for a, b in pairs:
        assert a in s, (path, a[:70])
        s = s.replace(a, b)
    open(W + path, 'w').write(s)

edit('web/src/vegetation.ts', [
("import { kindsOf } from './canopy';", "import { kindsOf, unhalf } from './canopy';\nimport { Loose, looseOf } from './roots';"),
("""  /** The live map, whose laws mask each mark at its foot (none: a tape's map, no cover drawn over it). */
  land: Land | null = null;""", """  /** The live map, whose laws mask each mark at its foot (none: a tape's map, no cover drawn over it). */
  land: Land | null = null;
  /** The squares' loose ground (roots.ts), once the samples' rock has come. */
  private loose: Loose | null = null;"""),
("""    const falling = cooling(w.meta.frames[0].year, (m.lat_north + m.lat_south) / 2);
    this.marks.set(kindsOf(m.w, m.h, rows, t.row, 1 + n * t.row, t.kinds, t.kind_row, t.walls, falling), m.cell_m);""",
"""    const falling = cooling(w.meta.frames[0].year, (m.lat_north + m.lat_south) / 2), rock = this.land?.tiles.rock;
    if (!this.loose && rock && this.land) {
      const s = this.land.samples;
      this.loose = looseOf(k => unhalf(rock[2 * k]), s.across, s.down, m.w, m.h);
    }
    this.marks.set(kindsOf(m.w, m.h, rows, t.row, 1 + n * t.row, t.kinds, t.kind_row, t.walls, falling, this.loose), m.cell_m);"""),
])
edit('web/src/canopy.ts', [
("""  return sign | ((e << 10) + ((m + 0x1000) >> 13));
}""", """  return sign | ((e << 10) + ((m + 0x1000) >> 13));
}

/** The number whose IEEE binary16 bits are `bits` (`half`'s inverse). */
export function unhalf(bits: number): number {
  const e = (bits >> 10) & 0x1f, m = bits & 0x3ff, sign = bits & 0x8000 ? -1 : 1;
  if (e === 0) return sign * m * 2 ** -24;
  if (e === 31) return m ? NaN : sign * Infinity;
  return sign * (1 + m / 1024) * 2 ** (e - 15);
}"""),
])
