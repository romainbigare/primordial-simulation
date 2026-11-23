// The page's own errors, each with the time it came, shown on it (a phone has no console to read them in) until tapped
// away: the first `ERRORS_FIRST`, which say what went wrong first, and the last `ERRORS_LAST`, with how many came
// between. Those without an error object are the browser's notes (a ResizeObserver's loop), not failures.

const ERRORS_FIRST = 3, ERRORS_LAST = 3;

/** Show the page's errors in `box` from now on. */
export function showErrors(box: HTMLElement): void {
  const errors: string[] = [];
  const failed = (text: string) => {
    errors.push(`${new Date().toLocaleTimeString()} ${text}`);
    const shown = errors.length <= ERRORS_FIRST + ERRORS_LAST ? errors
      : [...errors.slice(0, ERRORS_FIRST), `(${errors.length - ERRORS_FIRST - ERRORS_LAST} more)`, ...errors.slice(-ERRORS_LAST)];
    box.textContent = `${shown.join('\n')}\n(tap to hide)`;
    box.hidden = false;
  };
  box.onclick = () => { box.hidden = true; };
  addEventListener('error', e => { if (e.error) failed(e.error instanceof Error ? e.error.message : String(e.error)); });
  addEventListener('unhandledrejection', e => failed(e.reason instanceof Error ? e.reason.message : String(e.reason)));
}
