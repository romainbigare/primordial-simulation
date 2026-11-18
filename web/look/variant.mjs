// The page's programs built again (web/shade) from a copy of web/shaders the tool changes: the crowns' mask (mask.mjs)
// and the counts of a pixel's reads (cost.mjs). The page itself never holds them.
import { execFileSync } from 'node:child_process';
import { cpSync, readFileSync, writeFileSync, rmSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { ROOT, OUT } from './page.mjs';

/** In the copy `dir`, `text` of `file` replaced `by`: the text must stand once in its file, else the change no longer
 * fits the page's shaders and `who` (the tool's file that holds it) is changed with them. */
export function patch(dir, file, text, by, who) {
  const f = join(dir, file), s = readFileSync(f, 'utf8');
  if (s.split(text).length !== 2) throw new Error(`web/look/${who} no longer fits ${file}: "${text}"`);
  writeFileSync(f, s.replace(text, by));
}

/** The programs of the build `name`: web/shaders copied to `out/look/<name>-shaders`, changed by `change(dir)`, and
 * translated by web/shade; as the JSON it writes (for `page.site`). */
export function programs(name, change) {
  const dir = join(OUT, `${name}-shaders`), json = join(OUT, `${name}-shaders.json`);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(OUT, { recursive: true });
  cpSync(join(ROOT, 'web/shaders'), dir, { recursive: true });
  change(dir);
  execFileSync('cargo', ['run', '-q', '-p', 'ew-shade', '--', dir, json], { cwd: ROOT, stdio: ['ignore', 'ignore', 'inherit'] });
  return readFileSync(json, 'utf8');
}
