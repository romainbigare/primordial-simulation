// The crowns' mask: the page's programs built again (variant.mjs) from a copy of web/shaders in which the trees' crowns
// are pure magenta: the bodies of the marks of the lattice of stands (cover.wgsl: `lattice` names the lattice, `plant`
// paints the body) and the far field of stands (cover_field.wgsl `lattice_field`).
import { programs, patch } from './variant.mjs';

const MAGENTA = 'vec3f(1.0, 0.0, 1.0)';
/** The changes to the shaders' text, each [file, text, its replacement]; each text must stand once in its file. */
const PATCHES = [
  ['cover.wgsl', 'fn lattice(dst: ptr<function, vec4f>',
    'var<private> mask_stands: bool = false;\nfn lattice(dst: ptr<function, vec4f>'],
  ['cover.wgsl', '      plant(dst, shade, mark_plant(',
    '      mask_stands = l == STANDS;\n      plant(dst, shade, mark_plant('],
  ['cover.wgsl', 'if (m.a > 0.0) { over(dst, m.rgb, m.a * fade); }',
    `if (m.a > 0.0) { over(dst, select(m.rgb, ${MAGENTA}, mask_stands), m.a * fade); }`],
  ['lib/cover_field.wgsl', '  let dark = FIELD_SHADOW',
    `  if (l == STANDS) { c = ${MAGENTA}; }\n  let dark = FIELD_SHADOW`],
];

/** The programs of the mask build, as the JSON web/shade writes (for `page.site`). */
export const maskShaders = () => programs('mask', dir => PATCHES.forEach(([file, text, by]) => patch(dir, file, text, by, 'mask.mjs')));
