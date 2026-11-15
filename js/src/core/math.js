export const midiHz = m => 440 * Math.pow(2, (m - 69) / 12);
export const rnd = (a, b) => a + Math.random() * (b - a);

// small seeded RNG so baked models come out the same every load
export function seeded(n) {
  return () => ((n = (n * 1664525 + 1013904223) >>> 0) / 4294967296);
}

export const wrapA = a => a - Math.round(a / (Math.PI * 2)) * Math.PI * 2;
export const damp = (a, b, k, dt) => a + (b - a) * (1 - Math.exp(-k * dt));
export const ease = t => t * t * (3 - 2 * t);

export function pixelIcon(rows, cls = 'cicon') {
  let r = '';
  rows.forEach((row, y) => [...row].forEach((c, x) => {
    if (c !== '.') r += `<rect x="${x}" y="${y}" width="1" height="1"${c === 'o' ? ' opacity=".35"' : ''}/>`;
  }));
  return `<svg class="${cls}" viewBox="0 0 8 8" shape-rendering="crispEdges" fill="currentColor">${r}</svg>`;
}
