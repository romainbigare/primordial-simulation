// The controls of god mode (god.ts): the brush's sizes as chips (a mouse) or between a smaller and a larger button (a
// phone), and the big button held for a stroke at the sight on a phone.

/** A brush's radius `m` (m) in words. */
export const radiusText = (m: number) => (m >= 1000 ? `${m / 1000} km` : `${m} m`);

/** The brush's radii `radii` (m) as chips, the one on (`on`) pressed; a chip pressed calls `pick` with its radius. */
export function sizeChips(radii: number[], on: number, pick: (m: number) => void): HTMLElement {
  const list = document.createElement('div');
  list.className = 'chips sizes'; list.setAttribute('aria-label', 'Brush size');
  for (const m of radii) {
    const b = document.createElement('button');
    b.textContent = radiusText(m); b.classList.toggle('on', m === on);
    b.onclick = () => pick(m);
    list.append(b);
  }
  return list;
}

/** The brush's radius `on` (one of `radii`, m) between a smaller and a larger button, which call `pick` with the next. */
export function sizeStepper(radii: number[], on: number, pick: (m: number) => void): HTMLElement {
  const box = document.createElement('div'), k = radii.indexOf(on);
  box.className = 'stepper';
  const step = (text: string, label: string, to: number) => {
    const b = document.createElement('button');
    b.textContent = text; b.setAttribute('aria-label', label); b.disabled = to < 0 || to >= radii.length;
    b.onclick = () => pick(radii[to]);
    return b;
  };
  const size = document.createElement('b');
  size.textContent = radiusText(on);
  box.append(step('−', 'Smaller brush', k - 1), size, step('+', 'Larger brush', k + 1));
  return box;
}

/** A button held for a stroke: `start` when pressed (true: a stroke began, and the button keeps the pointer till it
 * lifts), `end` when let go or lost. */
export function holdButton(start: () => boolean, end: () => void): HTMLButtonElement {
  const b = document.createElement('button');
  b.className = 'hold';
  b.addEventListener('pointerdown', e => {
    if (!start()) return;
    b.setPointerCapture(e.pointerId); b.classList.add('held');
  });
  const up = () => { if (!b.classList.contains('held')) return; b.classList.remove('held'); end(); };
  for (const k of ['pointerup', 'pointercancel', 'lostpointercapture']) b.addEventListener(k, up);
  b.addEventListener('contextmenu', e => e.preventDefault());
  return b;
}
