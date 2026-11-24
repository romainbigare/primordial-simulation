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
  undo: '<path d="M9 14 4 9l5-5"/><path d="M4 9h10.5a5.5 5.5 0 0 1 0 11H11"/>',
  fire: '<path d="M12 21c-3.9 0-7-2.7-7-6.6 0-3.4 2.5-5.4 3.8-8.4 1.1 2 1.6 3 3.2 4 0-3 1-5.4 3-7 .6 3 4 5.4 4 10.6 0 4.5-3.1 7.4-7 7.4z"/><path d="M12 21c-1.7 0-3-1.2-3-2.9 0-1.6 1.3-2.5 1.9-4 .9 1 1.8 1.7 2.6 1 .7 1 1.5 1.8 1.5 3 0 1.7-1.3 2.9-3 2.9z"/>',
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
