// Inspect: the square under the cursor, or the one picked, in its card: its class on the map, where it lies on a globe,
// and each layer's value there; or the cover under the cursor where the map is close (slots.ts). The soil's texture
// legend marks the square's grain. Only while the card or the legend is shown: the map's classes are read back from
// the GPU for it.
import { World, plane, fmt, unitText, latOf, lonOf } from './data';
import { CLASSES } from './earth';
import type { MapView } from './map';
import type { CoverSlots } from './slots';
import { textureClass, textureLegend } from './soil';

/** The layers' names in plain words, in the groups and the order the Map panel lists them. */
export const LAYER_GROUPS: [string, Record<string, string>][] = [
  ['Air and climate', { t_ground: 'Ground temperature', t_air: 'Air temperature', rain: 'Rain', vapor: 'Vapor', absorber: 'Absorber', rises: 'Evaporation' }],
  ['Water and ice', { depth: 'Liquid depth', flux: 'River flux', catch: 'Catchment', wet: 'Ground wetness', ice: 'Ice', sheds: 'Runoff and drainage (rain less evaporation)' }],
  ['Ground', { z: 'Elevation', loose: 'Loose sediment', coarse: 'Sand (coarse grain)', fine: 'Clay (fine grain)', organic: 'Organic matter', conduct: 'Soil conductivity',
    avail: 'Available water', filled: 'Soil water (pores filled)', roots: 'Water for roots (share)' }],
  ['Life and decay', { litter: 'Fresh litter', slow: 'Humus (slow organic pool)', mineral_free: 'Nitrogen, dissolved', mineral_held: 'Nitrogen, held on the soil',
    rot: 'Rot speed', seeds: 'Seeds on the ground', grown: 'Growth last year', takers: 'Takers' }],
];
const LAYER_LABELS: Record<string, string> = Object.assign({}, ...LAYER_GROUPS.map(([, g]) => g));
/** A layer's name in plain words. */
export const layerLabel = (n: string) => LAYER_LABELS[n] ?? (n[0].toUpperCase() + n.slice(1).replace(/_/g, ' '));

export class Inspector {
  private cls: Uint8Array = new Uint8Array(0);
  /** The frames drawn, and the one whose classes `cls` were last asked for: they are read back from the GPU only when a
   * square is inspected, not at every frame. */
  private paints = 0;
  private classesOf = -1;
  /** The world and frame last shown. */
  private world: World | null = null;
  private frame = 0;

  constructor(private card: HTMLElement, private el: HTMLElement, private texture: HTMLElement, private map: MapView, private slots: CoverSlots) {}

  /** A new world of `n` squares. */
  newWorld(n: number): void { this.cls = new Uint8Array(n); }

  /** The map was drawn anew: its classes are asked for again at the next look. */
  painted(): void { this.paints++; }

  /** Show the square under the cursor, or the one picked, of frame `frame` of `world`. */
  show(world: World | null, frame: number): void {
    this.world = world; this.frame = frame;
    if (this.card.hidden && this.texture.hidden) return;
    const { el, map } = this, i = map.hover >= 0 ? map.hover : map.pick;
    const cover = this.slots.inspect();
    if (cover) { el.innerHTML = cover; return; }
    if (!world || i < 0) { el.innerHTML = '<p class="hint">Hover or tap the map.</p>'; return; }
    if (this.classesOf !== this.paints) {
      this.classesOf = this.paints;
      map.mapClasses().then(c => { if (this.world === world) { this.cls = c; this.show(this.world, this.frame); } });
    }
    const { meta } = world, [name, color] = CLASSES[this.cls[i]];
    const share = (n: string) => world.lut[world.index.get(n)!][plane(world, frame, n)[i]];
    const grain: [number, number] | null = world.index.has('coarse') ? [share('coarse'), share('fine')] : null;
    if (!this.texture.hidden) this.texture.innerHTML = textureLegend(grain);
    let rows = grain ? `<tr><td>Soil texture</td><td>${textureClass(...grain)}</td></tr>` : '';
    meta.layers.forEach((l, k) => {
      if (/^ground_[rgb]$/.test(l.name)) return;
      rows += `<tr><td>${layerLabel(l.name)}</td><td>${fmt(world.lut[k][plane(world, frame, l.name)[i]])} <u>${unitText(l.unit)}</u></td></tr>`;
    });
    el.innerHTML = `<div class="cell"><s style="background:${color}"></s><b>${name}</b><span>x ${i % meta.w}, y ${(i / meta.w) | 0}</span></div>${where(world, i)}<table>${rows}</table>`;
  }
}

/** A globe cell's latitude and longitude, as a line above the table (nothing on an island). */
function where(world: World, i: number): string {
  const m = world.meta;
  if (!m.globe) return '';
  const lat = latOf(m, (i / m.w) | 0), east = lonOf(m, i % m.w), lon = east > 180 ? east - 360 : east;
  const deg = (v: number, pos: string, neg: string) => `${Math.abs(v).toFixed(1)}° ${v < 0 ? neg : pos}`;
  return `<p class="where">${deg(lat, 'N', 'S')}, ${deg(lon, 'E', 'W')}</p>`;
}
