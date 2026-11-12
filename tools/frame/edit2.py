import os
os.chdir('/home/user/primordial-simulation/.claude/worktrees/agent-a2f9648fe0863f025')
p = 'web/src/ground.ts'
s = open(p).read()
def sub(a, b):
    global s
    assert s.count(a) == 1, a
    s = s.replace(a, b)
sub("private changed: () => void, error", "private changed: (shown: boolean) => void, error")
sub("if (m.kind === 'ready') { w.ready = true; this.rock ??= m.rock; this.changed(); return this.next(); }",
    "if (m.kind === 'ready') { w.ready = true; this.rock ??= m.rock; this.changed(true); return this.next(); }")
sub("    if (depth !== this.depth) { this.depth = depth; this.pageDue = true; }",
    "    if (depth !== this.depth) { this.depth = depth; this.makePage(); }")
sub("""    this.numbers.fill(0);
    this.numbersDue = this.pageDue = true;
    if (this.last) this.view(this.last);""", """    this.numbers.fill(0);
    this.numbersDue = true;
    this.makePage();
    if (this.last) this.view(this.last);""")
sub("""  /** Keep the tile that came, on a free layer or on the oldest's. */""",
    """  /** Keep the tile that came, on a free layer or on the oldest's. Only a tile that changes what the view shows draws
   * the map anew and fades in: one ahead of the view, or under finer tiles kept, is kept for later. */""")
sub("""      this.numbers.set([depth, x, y, came], 4 * layer);
      this.lastCame = came;
      this.numbersDue = this.pageDue = true;
      this.changed();""", """      this.numbers.set([depth, x, y, came], 4 * layer);
      this.numbersDue = true;
      const before = this.page;
      this.makePage();
      const shown = this.showsOtherThan(before);
      if (shown) this.lastCame = came;
      this.changed(shown);""")
sub("""  /** The page at the view's depth: for each of its tiles, the layer of the finest kept tile that holds it and of the
   * one kept before that (its nearest kept ancestor), -1 for none; found depth by depth from the coarsest. */
  private makePage(): void {""", """  /** Whether the page shows another layer than page `before` anywhere in the last view. */
  private showsOtherThan(before: { w: number; h: number; data: Float32Array }): boolean {
    const v = this.last, { w, h, data } = this.page;
    if (!v || before.w !== w || before.h !== h) return true;
    const s = this.squaresOf(this.depth), clip = (c: number, n: number) => Math.min(Math.max(Math.floor(c / s), 0), n - 1);
    for (let y = clip(v.y, h); y <= clip(v.y + v.h, h); y++) {
      for (let x = clip(v.x, w); x <= clip(v.x + v.w, w); x++) if (data[4 * (y * w + x)] !== before.data[4 * (y * w + x)]) return true;
    }
    return false;
  }

  /** The page at the view's depth, made anew as the tiles kept or that depth change (sent to the GPU at the next
   * draw): for each of its tiles, the layer of the finest kept tile that holds it and of the one kept before that (its
   * nearest kept ancestor), -1 for none; found depth by depth from the coarsest. */
  private makePage(): void {""")
sub("""    this.page = { w: nx, h: ny, data };
  }""", """    this.page = { w: nx, h: ny, data };
    this.pageDue = true;
  }""")
sub("""    if (this.pageDue) {
      this.makePage();
      if""", """    if (this.pageDue) {
      if""")
open(p, 'w').write(s)

p = 'web/src/main.ts'
s = open(p).read()
sub("m.tiles, () => { map.invalidate(); if (live) showStats(live.stats); }, say);",
    "m.tiles, shown => { if (shown) map.invalidate(); if (live) showStats(live.stats); }, say);")
open(p, 'w').write(s)
