// --- saved settings and best run (localStorage can throw in a sandboxed frame, so guard it) ---
export const store = {
  get(k, fallback) {
    try {
      return JSON.parse(localStorage.getItem('aah-' + k)) ?? fallback;
    } catch {
      return fallback;
    }
  },
  set(k, v) {
    try {
      localStorage.setItem('aah-' + k, JSON.stringify(v));
    } catch {
      /* not persisted */
    }
  },
};

export const settings = Object.assign(
  { master: 8, music: 8, shake: true },
  store.get('settings', {})
); // volumes are 0..10
