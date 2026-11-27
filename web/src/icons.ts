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
  grid: '<rect x="4" y="4" width="6.5" height="6.5" rx="1.5"/><rect x="13.5" y="4" width="6.5" height="6.5" rx="1.5"/><rect x="4" y="13.5" width="6.5" height="6.5" rx="1.5"/><rect x="13.5" y="13.5" width="6.5" height="6.5" rx="1.5"/>',
  human: '<circle cx="12" cy="7.5" r="3.5"/><path d="M5 20.5c.6-3.9 3.4-6.5 7-6.5s6.4 2.6 7 6.5"/>',
  animals: '<path d="M12 13c-2.8 0-5.5 3-5.5 5.3 0 1.5 1.2 2.2 2.6 2.2 1.2 0 1.9-.6 2.9-.6s1.7.6 2.9.6c1.4 0 2.6-.7 2.6-2.2 0-2.3-2.7-5.3-5.5-5.3z"/><ellipse cx="5.3" cy="10.3" rx="1.6" ry="2.1" transform="rotate(-20 5.3 10.3)"/><ellipse cx="9.4" cy="5.9" rx="1.6" ry="2.2"/><ellipse cx="14.6" cy="5.9" rx="1.6" ry="2.2"/><ellipse cx="18.7" cy="10.3" rx="1.6" ry="2.1" transform="rotate(20 18.7 10.3)"/>',
  nature: '<path d="M5 19c0-8.5 5-14 15-15 .5 9.5-5 15-15 15z"/><path d="M5 19c3-4 6-6.8 9.5-9"/>',
  destruction: '<path d="M13.5 2.5 5 13.5h6.5l-1 8 8.5-11h-6.5z"/>',
  population: '<circle cx="9" cy="8" r="3.2"/><path d="M3 20c.4-3.6 2.8-6 6-6s5.6 2.4 6 6"/><path d="M15.5 5a3.2 3.2 0 0 1 0 6.1M17.5 14.4c2 .8 3.2 2.9 3.5 5.6"/>',
  war: '<path d="M5 5l10.5 10.5M13 18l5-5M16.5 16.5l3 3M19 5 8.5 15.5M6 13l5 5M7.5 16.5l-3 3"/>',
  culture: '<path d="M12 6.5c-2-1.6-4.8-2.2-8.5-1.8v13.6c3.7-.4 6.5.2 8.5 1.8 2-1.6 4.8-2.2 8.5-1.8V4.7c-3.7-.4-6.5.2-8.5 1.8zM12 6.5v13.6"/>',
  technology: '<path d="M9.7 5.4 10.2 2.9h3.6l.5 2.5 2.3 1.3 2.4-.9 1.8 3.2-1.9 1.7v2.6l1.9 1.7-1.8 3.2-2.4-.9-2.3 1.3-.5 2.5h-3.6l-.5-2.5-2.3-1.3-2.4.9-1.8-3.2 1.9-1.7v-2.6L3.2 9l1.8-3.2 2.4.9z"/><circle cx="12" cy="12" r="3"/>',
  built: '<path d="M3.5 11 12 4l8.5 7M5.5 9.5V20h13V9.5M10 20v-5.5h4V20"/>',
  create: '<path d="m11 3.5 1.7 4.8 4.8 1.7-4.8 1.7-1.7 4.8-1.7-4.8L4.5 10l4.8-1.7zM18.5 14.5v6M15.5 17.5h6"/>',
  edit: '<path d="m15 4.5 4.5 4.5L9 19.5H4.5V15zM12.5 7l4.5 4.5"/>',
  smooth: '<path d="m3 16 2-5 2 3 2.5-5c3 0 4 6 7.5 6 2 0 3-1 4.5-1M3 20h18"/>',
  mountains: '<path d="M2 20 9 7.5l4.5 7.5 2.5-4 6 9zM6.9 11.2l1.5 1.3 1.2-1.1 1.5-.3"/>',
  rivers: '<path d="M10.5 3c-1.5 2 1.5 4-.5 6.5S5 15 4.5 21M13.5 3c-1 2 1.5 4.5 0 7S12 16 19.5 21"/>',
  lakes: '<path d="M4 13.5c0-4.2 3.5-6.5 8-6.5s8 2 8 5.8-3.4 6.2-8 6.2-8-1.3-8-5.5z"/><path d="M8.5 13c1.2-.8 2.3-.8 3.5 0s2.3.8 3.5 0"/>',
  meteor: '<circle cx="8" cy="16" r="4"/><path d="M11 13 20.5 3.5M13.5 16 19 10.5M8 11l5.5-5.5"/>',
  volcano: '<path d="M3 20.5 8.5 11h7L21 20.5zM12 7.5v-4M9.2 8.2 7.5 5.5M14.8 8.2l1.7-2.7M12 11v2.5l-1.5 2.5"/>',
  tsunami: '<path d="M3 19c.5-7.5 5-13 11-13 3.2 0 5.5 2.1 5.5 4.7 0 2.1-1.6 3.6-3.5 3.6-1.8 0-3-1.3-3-3 0-1.2.7-2.1 1.7-2.5M3 19h18"/>',
  flood: '<path d="M5 11 12 5l7 6M7 9.5V14M17 9.5V14M2.5 17.5c1.5 0 2.5-1.2 4-1.2s2.5 1.2 4 1.2 2.5-1.2 4-1.2 2.5 1.2 4 1.2 2.2-1 3-1.2M2.5 21c1.5 0 2.5-1.2 4-1.2s2.5 1.2 4 1.2 2.5-1.2 4-1.2 2.5 1.2 4 1.2 2.2-1 3-1.2"/>',
  drought: '<circle cx="12" cy="8" r="3.2"/><path d="M12 2v1M17.5 8H19M5 8h1.5M16 4l-.8.8M8 4l.8.8M3 15.5h18M7 15.5l1.5 2.2-1 2.8M13 15.5l-1 2 1.8 1.6-.6 1.4M17.5 15.5l1 2.5"/>',
  heatwave: '<path d="M8 14.2V5a2 2 0 0 1 4 0v9.2a3.8 3.8 0 1 1-4 0zM10 11v5M16 6c1 1 1 2 0 3s-1 2 0 3M19.5 6c1 1 1 2 0 3s-1 2 0 3"/>',
  glacial: '<path d="M12 21.5v-19M3.8 16.8l16.4-9.6M3.8 7.3l16.4 9.5M12 5.8l-1.8-2.1M12 5.8l1.8-2.1M17.4 8.9l.9-2.6M17.4 8.9l2.7.5M17.4 15.1l2.7-.5M17.4 15.1l.9 2.6M12 18.2l1.8 2.1M12 18.2l-1.8 2.1M6.6 15.1l-.9 2.6M6.6 15.1l-2.7-.5M6.6 8.9l-2.7.5M6.6 8.9l-.9-2.6"/>',
  storm: '<path d="M3 4.5h18M4.5 8.5h14M7 12.5h9.5M9 16.5h5.5M10.5 20.5h2.5"/>',
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
