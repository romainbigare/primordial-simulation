// The page's icons, all here: line drawings on a 24 px grid, stroked 1.75 px with round caps and joins, in the colour
// of the text around them (`currentColor`). Inline SVG, so the page needs no font and no network for them. An element
// with `data-icon="name"` gets its icon when the page starts (`fillIcons`).

/** Each icon's drawing: the SVG elements inside a 24 x 24 box. */
const ICONS = {
  globe: '<circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3c2.5 2.6 3.8 5.6 3.8 9s-1.3 6.4-3.8 9M12 3c-2.5 2.6-3.8 5.6-3.8 9s1.3 6.4 3.8 9"/>',
  play: '<path d="M7.5 5v14l11-7z" fill="currentColor"/>',
  pause: '<rect x="6.5" y="5" width="3.5" height="14" rx="1" fill="currentColor"/><rect x="14" y="5" width="3.5" height="14" rx="1" fill="currentColor"/>',
  layers: '<path d="M12 3.5 3 8.2l9 4.7 9-4.7z"/><path d="m3 12.2 9 4.7 9-4.7"/><path d="m3 16.2 9 4.7 9-4.7"/>',
  inspect: '<circle cx="12" cy="12" r="9"/><path d="M12 11v5.5M12 7.8v.01"/>',
  wand: '<path d="m4 20 10-10"/><path d="M16 3v2.5M16 10.5V13M11 8h2.5M18.5 8H21M12.5 4.5 14 6M19.5 4.5 18 6M18 10l1.5 1.5"/>',
  fit: '<path d="M4 9V4h5M15 4h5v5M20 15v5h-5M9 20H4v-5"/>',
  close: '<path d="m6 6 12 12M18 6 6 18"/>',
  plant: '<path d="M12 21v-9M7 21h10"/><path d="M12 12c0-4.2 2.8-7 8-7 0 4.2-2.8 7-8 7zM12 14.5c0-3.3-2.3-5.5-7-5.5 0 3.4 2.3 5.5 7 5.5z"/>',
  raise: '<path d="M3 20h18M5 20l5-7 3 3 2-2 4 6"/><path d="M17 4v7M14 7l3-3 3 3"/>',
  lower: '<path d="M3 20h18M5 20l5-7 3 3 2-2 4 6"/><path d="M17 4v7M14 8l3 3 3-3"/>',
  lake: '<path d="M3 9c2-1.5 4-1.5 6 0s4 1.5 6 0 4-1.5 6 0M3 14c2-1.5 4-1.5 6 0s4 1.5 6 0 4-1.5 6 0M3 19c2-1.5 4-1.5 6 0s4 1.5 6 0 4-1.5 6 0"/>',
  undo: '<path d="M9 14 4 9l5-5"/><path d="M4 9h10.5a5.5 5.5 0 0 1 0 11H11"/>',
  fire: '<path d="M12 21c-3.9 0-7-2.7-7-6.6 0-3.4 2.5-5.4 3.8-8.4 1.1 2 1.6 3 3.2 4 0-3 1-5.4 3-7 .6 3 4 5.4 4 10.6 0 4.5-3.1 7.4-7 7.4z"/><path d="M12 21c-1.7 0-3-1.2-3-2.9 0-1.6 1.3-2.5 1.9-4 .9 1 1.8 1.7 2.6 1 .7 1 1.5 1.8 1.5 3 0 1.7-1.3 2.9-3 2.9z"/>',
  grid: '<path d="M4 4h6v6H4zM14 4h6v6h-6zM4 14h6v6H4zM14 14h6v6h-6z"/>',
  human: '<circle cx="12" cy="7" r="3.5"/><path d="M4.5 21c0-4.4 3.4-7.5 7.5-7.5s7.5 3.1 7.5 7.5"/>',
  animals: '<path d="M12 12.5c-3 0-5.5 3.2-5.5 5.6 0 1.6 1.3 2.4 2.8 2.4 1.2 0 1.8-.7 2.7-.7s1.5.7 2.7.7c1.5 0 2.8-.8 2.8-2.4 0-2.4-2.5-5.6-5.5-5.6z"/><ellipse cx="5.5" cy="10" rx="1.6" ry="2"/><ellipse cx="9.5" cy="5.5" rx="1.6" ry="2"/><ellipse cx="14.5" cy="5.5" rx="1.6" ry="2"/><ellipse cx="18.5" cy="10" rx="1.6" ry="2"/>',
  nature: '<path d="M12 21v-9M7 21h10"/><path d="M12 12c0-4.2 2.8-7 8-7 0 4.2-2.8 7-8 7zM12 14.5c0-3.3-2.3-5.5-7-5.5 0 3.4 2.3 5.5 7 5.5z"/>',
  destruction: '<path d="M13 2 4 14h7l-1 8 9-12h-7z"/>',
  population: '<circle cx="9" cy="8" r="3"/><path d="M3 20c0-3.3 2.7-6 6-6s6 2.7 6 6M15.5 5.2a3 3 0 0 1 0 5.6M17.5 14.3c2 .7 3.5 2.9 3.5 5.7"/>',
  war: '<path d="m5 3 12 12M15 17l2-2 3 3-2 2zM19 3 7 15M9 17l-2-2-3 3 2 2z"/>',
  culture: '<path d="M4 5c3-1 6-1 8 1v14c-2-2-5-2-8-1zM20 5c-3-1-6-1-8 1v14c2-2 5-2 8-1z"/>',
  technology: '<circle cx="12" cy="12" r="3"/><path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9 7 7M17 17l2.1 2.1M4.9 19.1 7 17M17 7l2.1-2.1"/>',
  built: '<path d="M3 11 12 4l9 7M5.5 9.5V20h13V9.5M10 20v-5h4v5"/>',
  create: '<path d="m11 3 1.8 5.2L18 10l-5.2 1.8L11 17l-1.8-5.2L4 10l5.2-1.8zM19 15v5M16.5 17.5h5"/>',
  edit: '<path d="m14.5 4.5 5 5L10 19H5v-5zM12.5 6.5l5 5"/>',
  smooth: '<path d="M3 16c3 0 4-5 9-5s6 5 9 5M3 20h18M8 6h8M10 3h4"/>',
  mountains: '<path d="M2 20 9 8l4 6 2.5-3.5L22 20zM7.2 11 9 12.5l1.6-2"/>',
  rivers: '<path d="M8 3c0 4 5 4 5 9s-6 5-6 9M14 3c0 4 5 4 5 9s-6 5-6 9"/>',
  lakes: '<ellipse cx="12" cy="13" rx="8.5" ry="5.5"/><path d="M8 13c1.3-.8 2.7-.8 4 0s2.7.8 4 0"/>',
  meteor: '<circle cx="8" cy="16" r="3.5"/><path d="M10.5 13.5 20 4M12.8 15.8 21 8M8.2 11.2 16 3"/>',
  volcano: '<path d="M2 21 8.5 11h7L22 21zM12 4v4M9 6l1.3 2M15 6l-1.3 2"/>',
  tsunami: '<path d="M2 19c2.5 0 3-2 5-2s2.5 2 5 2 3-2 5-2 2.5 2 5 2M3 14c0-5.5 4-10 9.5-10 2.6 0 4.5 1.6 4.5 4-2-1.4-5.5-1-5.5 2.6 0 2 1.4 3.4 3.5 3.4"/>',
  flood: '<path d="M5 11 12 5l7 6v3M7.5 9.5V14M16.5 9.5V14M2 17.5c2 0 3-1.5 5-1.5s3 1.5 5 1.5 3-1.5 5-1.5 3 1.5 5 1.5M2 21.5c2 0 3-1.5 5-1.5s3 1.5 5 1.5 3-1.5 5-1.5 3 1.5 5 1.5"/>',
  drought: '<circle cx="12" cy="7" r="3"/><path d="M12 1.5v1M17.5 7h1M5.5 7h1M3 20h18M7.5 20 9 17.5l-1-2M14 20l-1-2 2-2"/>',
  heatwave: '<circle cx="12" cy="12" r="4"/><path d="M12 2v2.5M12 19.5V22M2 12h2.5M19.5 12H22M4.9 4.9l1.8 1.8M17.3 17.3l1.8 1.8M4.9 19.1l1.8-1.8M17.3 6.7l1.8-1.8"/>',
  glacial: '<path d="M12 2v20M3.3 7l17.4 10M3.3 17 20.7 7M9.5 3.5 12 6l2.5-2.5M9.5 20.5 12 18l2.5 2.5"/>',
  storm: '<path d="M3 5h18M5 9h14M7 13h10M9 17h5M11 21h2"/>',
};
export type IconName = keyof typeof ICONS;

/** The icon `name` as SVG markup, its element given the class `cls` where asked. */
export function iconSvg(name: IconName, cls = ''): string {
  return `<svg class="i ${cls}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${ICONS[name]}</svg>`;
}

/** Put its icon at the start of each element under `root` that names one (`data-icon`). */
export function fillIcons(root: ParentNode): void {
  root.querySelectorAll<HTMLElement>('[data-icon]').forEach(el => el.insertAdjacentHTML('afterbegin', iconSvg(el.dataset.icon as IconName)));
}
