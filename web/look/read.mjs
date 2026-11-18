// What the tool reads back from the page: the kinds of cover of a square, from the cells' table (tape/src/table.rs,
// tape/src/stands.rs `row`).

/** Of a kind's row (`KIND_ROW` numbers), where its starting kind, built cover, height, crown's share of the ground and
 * walls gene stand (tape/src/stands.rs `row`). */
const K = { root: 0, kg: 1, height: 4, cover: 6, walls: 19 };

/** The kinds of cover of the square that holds point `at` (x, y in squares) of the world the page stands on: for each,
 * its starting kind, height, m, the share of the ground its crowns cover and the share of walls in what it builds (of
 * its walls gene, log10 kg of walls per kg of matter), as a line of text; and the land's share of the square. */
export function kinds(page, [x, y]) {
  return page.evaluate(({ x, y, K }) => {
    const e = window.__ew, m = e.world.meta, t = m.tiles, rows = e.live.table, i = Math.floor(y) * m.w + Math.floor(x);
    const land = rows[1 + t.row * i + 10], at = 1 + m.w * m.h * t.row, out = [];
    for (let k = 0; k < t.kinds; k++) {
      const o = at + (i * t.kinds + k) * t.kind_row;
      if (rows[o + K.root] < 0 || rows[o + K.kg] <= 0) continue;
      const walls = 10 ** rows[o + K.walls] / (1 + 10 ** rows[o + K.walls]);
      out.push(`kind ${rows[o + K.root]} ${rows[o + K.height].toFixed(1)} m, crowns ${(100 * rows[o + K.cover]).toFixed(0)} %, walls ${(100 * walls).toFixed(0)} %`);
    }
    return `land ${(100 * land).toFixed(0)} %: ${out.join(', ') || 'no cover'}`;
  }, { x, y, K });
}

/** A mark's place among its kind's groups is kept in this many steps a group (lib/mark.wgsl `PLACE_STEPS`). */
const PLACE_STEPS = 16;

/** The marks the page decided for its last draw, read back from the GPU (the first of the marks' textures,
 * lib/cells.wgsl `mark_word`: one more than the mark's code, 0 for none): of each, its lattice and cell (fixed in the
 * world), its square, its kind's slot (a label the kind keeps while it lives) and its group, as `l x y square slot group`
 * keys. The cells' rows are laid out as textures.ts `prepare` lays them. */
export function marks(page) {
  return page.evaluate(async PLACE_STEPS => {
    const e = window.__ew, cm = e.vegetation.marks, k = cm.kinds, d = e.vegetation.drawn, L = e.lattices;
    if (!k || !d || !cm.decided) return [];
    const shown = cm.shown(d.seenMpp), size = [d.rect[2] * d.mpp, d.rect[3] * d.mpp], angled = d.style === 'angled';
    let row = 0;
    const grids = L.LATTICES.map((a, l) => {
      const g = [d.at[0] / a, d.at[1] / a], frac = g.map(v => v - Math.floor(v));
      const rows = angled ? Math.min(Math.max(Math.ceil((L.STAND * k.tallest[l]) / a) - L.REACH[l], 0), L.ROWS_MOST) : 0;
      const span = (c, n) => (shown[l] > 0 ? Math.floor(frac[c] + size[c] / a) + 1 + 2 * L.MARK_MARGIN + L.MARK_SPARE + n : 0);
      const cells = { cols: span(0, 0), rows: span(1, rows), row };
      row += cells.rows;
      return { base: g.map(Math.floor), cells };
    });
    const t = cm.marksTex[0], words = await e.map.gpu.read(t), P = k.planes, n = k.w;
    const texel = (plane, i) => 4 * ((((plane % L.PLANES_DOWN) * k.h + Math.floor(i / n)) * k.cols + Math.floor(plane / L.PLANES_DOWN)) * n + (i % n));
    const out = [];
    grids.forEach((g, l) => {
      for (let y = 0; y < g.cells.rows; y++) for (let x = 0; x < g.cells.cols; x++) {
        const o = 4 * ((g.cells.row + y) * t.w + x), word = (words[o] | (words[o + 1] << 8) | (words[o + 2] << 16) | (words[o + 3] << 24)) >>> 0;
        if (!word) continue;
        const c = word - 1, step = c % (L.GROUPS * PLACE_STEPS), kd = Math.floor(c / (L.GROUPS * PLACE_STEPS));
        const kind = kd % P.kinds, sq = Math.floor(kd / P.kinds), slot = k.table[texel(P.label + (kind >> 2), sq) + (kind & 3)];
        out.push(`${l} ${g.base[0] + x - L.MARK_MARGIN} ${g.base[1] + y - L.MARK_MARGIN} ${sq} ${slot} ${Math.floor(step / PLACE_STEPS)}`);
      }
    });
    return out;
  }, PLACE_STEPS);
}

/** From the marks of one year (`marks`) to those of the next, in % (README.md): of the first year's, those whose cell
 * holds a mark of the same square, slot and group (`stay`), another (`other`) or none (`go`); of the next year's, those
 * whose cell held none (`appear`); and the marks of each year. */
export function persist(a, b) {
  const cell = m => m.split(' ').slice(0, 3).join(' '), was = new Map(a.map(m => [cell(m), m])), now = new Map(b.map(m => [cell(m), m]));
  let stay = 0, other = 0, go = 0;
  for (const [c, m] of was) { const q = now.get(c); if (q === undefined) go++; else if (q === m) stay++; else other++; }
  const appear = b.filter(m => !was.has(cell(m))).length, pc = (v, of) => +(100 * v / Math.max(of, 1)).toFixed(1);
  return { stay: pc(stay, a.length), other: pc(other, a.length), go: pc(go, a.length), appear: pc(appear, b.length), marks: [a.length, b.length] };
}
