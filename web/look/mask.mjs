// The crowns' mask: the page's programs built again (web/shade) from a copy of web/shaders in which the trees' crowns
// are pure magenta: the bodies of the marks of the lattice of stands (cover_marks.wgsl `lattice` names the lattice,
// cover.wgsl and crowns.wgsl `plant` paint the body) and the far field of stands (cover_field.wgsl `lattice_field`).
// The page itself never holds it: the tool builds it.
import { execFileSync } from 'node:child_process';
import { cpSync, readFileSync, writeFileSync, rmSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { ROOT, OUT } from './page.mjs';

const MAGENTA = 'vec3f(1.0, 0.0, 1.0)';
/** The changes to the shaders' text, each [file, text, its replacement]; each text must stand once in its file. */
const PATCHES = [
  ['lib/cover_marks.wgsl', 'fn lattice(dst: ptr<function, vec4f>',
    'var<private> mask_stands: bool = false;\nfn lattice(dst: ptr<function, vec4f>'],
  ['lib/cover_marks.wgsl', '      plant(dst, shade, mark_plant(',
    '      mask_stands = l == STANDS;\n      plant(dst, shade, mark_plant('],
  ['cover.wgsl', 'if (m.a > 0.0) { over(dst, m.rgb, m.a * fade); }',
    `if (m.a > 0.0) { over(dst, select(m.rgb, ${MAGENTA}, mask_stands), m.a * fade); }`],
  ['crowns.wgsl', 'over(dst, mark_colour(mix(leaf, bark, body.wood) * body.tone, body.lit, body.rim), body.a * fade);',
    `over(dst, select(mark_colour(mix(leaf, bark, body.wood) * body.tone, body.lit, body.rim), ${MAGENTA}, mask_stands), body.a * fade);`],
  ['lib/cover_field.wgsl', '  let dark = FIELD_SHADOW',
    `  if (l == STANDS) { c = ${MAGENTA}; }\n  let dark = FIELD_SHADOW`],
];

/** The programs of the mask build, as the JSON web/shade writes (for `page.site`). */
export function maskShaders() {
  const dir = join(OUT, 'mask-shaders'), json = join(OUT, 'mask-shaders.json');
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(OUT, { recursive: true });
  cpSync(join(ROOT, 'web/shaders'), dir, { recursive: true });
  for (const [file, text, by] of PATCHES) {
    const f = join(dir, file), s = readFileSync(f, 'utf8');
    if (s.split(text).length !== 2) throw new Error(`The crowns' mask no longer fits ${file} (web/look/mask.mjs): "${text}"`);
    writeFileSync(f, s.replace(text, by));
  }
  execFileSync('cargo', ['run', '-q', '-p', 'ew-shade', '--', dir, json], { cwd: ROOT, stdio: ['ignore', 'ignore', 'inherit'] });
  return readFileSync(json, 'utf8');
}
