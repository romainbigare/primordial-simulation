import os
os.chdir('/home/user/primordial-simulation/.claude/worktrees/agent-a2f9648fe0863f025')
def edit(p, pairs):
    s = open(p).read()
    for a, b in pairs:
        assert s.count(a) == 1, (p, a)
        s = s.replace(a, b)
    open(p, 'w').write(s)

edit('web/src/gpu.ts', [('''/** The bindings a program reads,''', '''/** The share of the metres a pixel takes that a program made for draws seen from at least so many metres (web/shade)
 * is chosen within, so that every pixel of the draw takes at least that many (they differ by a pixel's rounding). */
const LEAST_SPARE = 0.99;
/** Of program `name`, the one made for draws whose pixels take at least the most metres no more than `e`, the metres a
 * pixel of the draw takes (`name` itself where none is). */
export function seenFrom(name: string, e: number): string {
  let best = name, most = 0;
  for (const key of Object.keys(SHADERS)) {
    const [base, m] = key.split(':');
    if (base === name && m !== undefined && +m <= e * LEAST_SPARE && +m > most) { best = key; most = +m; }
  }
  return best;
}

/** The bindings a program reads,''')])
edit('web/src/map.ts', [("import { Draw, Gpu, Pass, Texture, Uniforms, openGpu } from './gpu';", "import { Draw, Gpu, Pass, Texture, Uniforms, openGpu, seenFrom } from './gpu';"),
("""    if (land) xs.forEach((_, c) => p.draw('land', { rect""", """    // The live map's program made for pixels as large as these (gpu.ts `seenFrom`).
    const program = seenFrom('land', this.cellM / (s * k));
    if (land) xs.forEach((_, c) => p.draw(program, { rect""")])
