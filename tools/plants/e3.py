W = '/home/user/primordial-simulation/.claude/worktrees/agent-a257b588d76bf8a62/'
def edit(path, pairs):
    s = open(W + path).read()
    for a, b in pairs:
        assert a in s, (path, a[:70])
        s = s.replace(a, b)
    open(W + path, 'w').write(s)

edit('web/src/canopy.ts', [
("import { formOf } from './form';", "import { formOf } from './form';\nimport { Loose, needOf, rootedMean } from './roots';"),
("""/** The planes of the kinds' table (cover.wgsl): of each lattice the chance a cell holds a mark of each kind (on the
 * lattice of stands, `STANDS`, negated for its square's first kind, `Paint`) and the mark's width, m; how wet each kind
 * needs its ground; then""", """/** The planes of the kinds' table (cover.wgsl): of each lattice the chance a cell holds a mark of each kind where its
 * roots hold (over its square's mean, roots.ts; on the lattice of stands, `STANDS`, negated for its square's first kind,
 * `Paint`) and the mark's width, m; how wet each kind needs its ground and how deep its loose ground, m; then"""),
("export const KIND_PLANES = { chance: 0, width: 3, need: 6, form: 7, from: 11, to: 15, petal: 19, shape: 23, count: 27 } as const;",
 "export const KIND_PLANES = { chance: 0, width: 3, need: 6, roots: 7, form: 8, from: 12, to: 16, petal: 20, shape: 24, count: 28 } as const;"),
("""/** The layers of the field (`CoverField`): one a lattice (its marks' plants), then the herbs' sward, the herbs' flowers,
 * and of the sward and of the lattice of stands the part its first kind makes (`Paint`). */
export const FIELD = { sward: LATTICES.length, flowers: LATTICES.length + 1, swardFirst: LATTICES.length + 2, standsFirst: LATTICES.length + 3, count: LATTICES.length + 4 } as const;""",
"""/** The layers of the field (`CoverField`): one a lattice (its marks' plants), then the herbs' sward, the herbs' flowers,
 * of the sward and of the lattice of stands the part its first kind makes (`Paint`), and of each lattice and the sward
 * (red, green, blue, alpha) its plants' mean height, m, and their mean rooted share in the square (roots.ts). */
export const FIELD = { sward: LATTICES.length, flowers: LATTICES.length + 1, swardFirst: LATTICES.length + 2, standsFirst: LATTICES.length + 3, tall: LATTICES.length + 4, rooted: LATTICES.length + 5, count: LATTICES.length + 6 } as const;"""),
("""/** What a layer of the field (`FIELD`) gathers in a square: its kinds' colours summed by the ground each covers, that
 * ground, the ground they leave, the shadow they cast (the plants a m² times their shadow's area), and its first kind:""",
"""/** What a layer of the field (`FIELD`) gathers in a square: its kinds' colours summed by the ground each covers, that
 * ground, the ground they leave, the shadow they cast (the plants a m² times their shadow's area), their heights and
 * rooted shares summed by the ground each covers, and its first kind:"""),
("""  shadow = 0;
  first = { root: Infinity, rgb: [0, 0, 0] as Rgb, covers: 0, k: -1 };

  /** Add kind `k` of starting kind `root` and colour `c`, covering `covers` of the ground and casting `shadow`. */
  add(k: number, root: number, c: Rgb, covers: number, shadow: number): void {
    if (covers <= 0) return;
    this.rgb = this.rgb.map((v, q) => v + covers * c[q]) as Rgb;
    this.covered += covers;""", """  shadow = 0;
  tall = 0;
  held = 0;
  first = { root: Infinity, rgb: [0, 0, 0] as Rgb, covers: 0, k: -1 };

  /** Add kind `k` of starting kind `root` and colour `c`, covering `covers` of the ground and casting `shadow`, its plants
   * `height` m tall and their roots holding `rooted` of the square on the mean. */
  add(k: number, root: number, c: Rgb, covers: number, shadow: number, height: number, rooted: number): void {
    if (covers <= 0) return;
    this.rgb = this.rgb.map((v, q) => v + covers * c[q]) as Rgb;
    this.covered += covers;
    this.tall += covers * height;
    this.held += covers * rooted;"""),
("""    out[firstAt + 3] = share * (cover + shaded);
  }
}""", """    out[firstAt + 3] = share * (cover + shaded);
  }

  /** Into `out` at `tallAt` and `heldAt`: its plants' mean height and rooted share (1 where it holds none). */
  putRoots(out: Float32Array, tallAt: number, heldAt: number): void {
    out[tallAt] = this.tall / Math.max(this.covered, 1e-9);
    out[heldAt] = this.covered > 0 ? this.held / this.covered : 1;
  }
}"""),
("""/** The kinds of the core's table `rows` for a map of `w` x `h` squares (`row` numbers a square, tape/src/table.rs):
 * `kinds` kinds a square of `kindRow` numbers, from `at` on; while the ground cools by `falling` (style.ts
 * `cooling`). */
export function kindsOf(w: number, h: number, rows: Float32Array, row: number, at: number, kinds: number, kindRow: number, walls: Rgb, falling: number): Kinds {""",
"""/** The kinds of the core's table `rows` for a map of `w` x `h` squares (`row` numbers a square, tape/src/table.rs):
 * `kinds` kinds a square of `kindRow` numbers, from `at` on; while the ground cools by `falling` (style.ts
 * `cooling`); on the squares' loose ground `loose` (none: not known yet). */
export function kindsOf(w: number, h: number, rows: Float32Array, row: number, at: number, kinds: number, kindRow: number, walls: Rgb, falling: number, loose: Loose | null): Kinds {"""),
("""      put(KIND_PLANES.need, i, k, 10 ** r[K.moist]);""", """      const need = needOf(r[K.height]), mean = rootedMean(loose, i, need);
      put(KIND_PLANES.need, i, k, 10 ** r[K.moist]);
      put(KIND_PLANES.roots, i, k, need);"""),
("""        sward.add(k, r[K.root], c, covers, 0);
        flowers.add(k, 0, look.petal, covers * look.bloom, 0);""", """        sward.add(k, r[K.root], c, covers, 0, r[K.height], mean);
        flowers.add(k, 0, look.petal, covers * look.bloom, 0, r[K.height], mean);"""),
("""        put(KIND_PLANES.chance + l, i, k, chance);""", """        put(KIND_PLANES.chance + l, i, k, chance / mean);"""),
("""        paint[l].add(k, r[K.root], c, covers, (chance / (a * a)) * width * SHADOW_RUN * r[K.height]);""",
 """        paint[l].add(k, r[K.root], c, covers, (chance / (a * a)) * width * SHADOW_RUN * r[K.height], r[K.height], mean);"""),
("""    sward.put(field, layer(FIELD.sward, i), layer(FIELD.swardFirst, i));""", """    sward.put(field, layer(FIELD.sward, i), layer(FIELD.swardFirst, i));
    [...paint, sward].forEach((p, q) => p.putRoots(field, layer(FIELD.tall, i) + q, layer(FIELD.rooted, i) + q));"""),
])
edit('web/build.mjs', [("'style', 'form', 'canopy'", "'style', 'form', 'roots', 'canopy'")])
edit('web/src/land.ts', [("constructor(readonly tiles: GroundTiles, private laws: Laws, private samples: SamplesOf)", "constructor(readonly tiles: GroundTiles, private laws: Laws, readonly samples: SamplesOf)")])
