// The saved worlds the scenes stand on: snapshots `ew run --snapshots-at` writes, kept in `out/look/worlds/` and made
// again when missing or when the build that makes them changed (the snapshot's format version, or the laws' code).
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { ROOT, OUT } from './page.mjs';

/** Months a year (the core's `Config::months_per_year`), to name a snapshot by its date. */
const MONTHS = 12;
/** The folders whose code makes a world: the laws, the world's making, the snapshot and the command line. */
const MAKERS = ['core', 'worldgen', 'tape', 'cli'];

const git = args => execFileSync('git', args, { cwd: ROOT, encoding: 'utf8', maxBuffer: 1 << 28 });

/** What the worlds made now depend on: the snapshot's format version, and the makers' code as it is in the tree
 * (committed and not). */
function build() {
  const version = /SNAPSHOT_VERSION: u32 = (\d+)/.exec(readFileSync(join(ROOT, 'tape/src/snapshot.rs'), 'utf8'))[1];
  const code = git(['rev-parse', ...MAKERS.map(m => `HEAD:${m}`)]) + git(['diff', 'HEAD', '--', ...MAKERS]);
  return { version, code: createHash('sha256').update(code).digest('hex').slice(0, 12) };
}

/** The file name `ew run` gives the snapshot at `date` (years since the start, whole months). */
export function snapName(date) {
  const m = Math.round(date * MONTHS);
  return `snap_${String(Math.floor(m / MONTHS)).padStart(5, '0')}_${String(m % MONTHS).padStart(2, '0')}.bin.gz`;
}

/** The dates of world `w` of scenes.json: its date `at`, and with `years` that date in each of its `years` years. */
export function dates(w, years = false) {
  return Array.from({ length: years ? w.years : 1 }, (_, i) => w.at + i);
}

/** The snapshots of world `name` (`spec`, of scenes.json) at `when` (dates), made where missing: their paths under
 * `out/look`, by date. `log` hears what is made. */
export function worlds(name, spec, when, log = console.log) {
  const b = build(), rel = `worlds/${name}-v${b.version}-${b.code}`, dir = join(OUT, rel);
  const missing = when.filter(d => !existsSync(join(dir, snapName(d))));
  if (missing.length) {
    rmSync(dir, { recursive: true, force: true });
    const last = Math.max(...when), years = String(Math.ceil(last + 1 / (2 * MONTHS)));
    const args = ['run', '--release', '-q', '-p', 'cli', '--', 'run', ...spec.run, '--years', years, '--frames-every', '1000', '--snapshots-at', when.join(','), '--out', dir];
    log(`making world ${name} at ${when.join(', ')}: cargo ${args.join(' ')}`);
    const t0 = Date.now();
    execFileSync('cargo', args, { cwd: ROOT, stdio: ['ignore', 'ignore', 'inherit'] });
    log(`  made in ${((Date.now() - t0) / 1000).toFixed(0)} s`);
  }
  return Object.fromEntries(when.map(d => [d, `${rel}/${snapName(d)}`]));
}
