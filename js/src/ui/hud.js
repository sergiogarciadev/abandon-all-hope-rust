import { state, zombies } from '../core/state.js';
import { pixelIcon } from '../core/math.js';
import { UPGRADE_ICONS } from './upgrades.js';

export const hud = {
  wave: document.getElementById('waveNum'),
  kills: document.getElementById('kills'),
  zleft: document.getElementById('zleft'),
  health: document.getElementById('health'),
  powerups: document.getElementById('powerups'),
  toast: document.getElementById('toast'),
  picker: document.getElementById('picker'),
  pickerTitle: document.getElementById('pickerTitle'),
  pickerSub: document.getElementById('pickerSub'),
  cards: document.getElementById('cards'),
  mag: document.getElementById('mag'),
  reloadBar: document.getElementById('reloadBar'),
  reloadFill: document.getElementById('reloadFill'),
  flash: document.getElementById('flash'),
  banner: document.getElementById('banner'),
  bannerText: document.getElementById('bannerText'),
  bannerSub: document.getElementById('bannerSub'),
};

export function updateHUD() {
  hud.wave.textContent = state.wave;
  hud.kills.textContent = state.kills;
  const remaining = zombies.length + state.toSpawn;
  hud.zleft.textContent = state.intermission > 0 || state.choosing ? '' : remaining + ' LEFT';
  if (hud.health.children.length !== state.maxHp) hud.health.innerHTML = pixelIcon(UPGRADE_ICONS.heal, 'pip').repeat(state.maxHp);
  [...hud.health.children].forEach((p, i) => p.classList.toggle('dead', i >= state.hp));
  // power-up time left as a clock: 0:10 … 0:01
  const clock = t => { const s = Math.ceil(t); return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`; };
  let pu = '';
  if (state.pDamage > 0) pu += `<div style="color:#ff7a2e">DOUBLE DAMAGE ${clock(state.pDamage)}</div>`;
  if (state.pAmmo > 0) pu += `<div style="color:#e8d28a">BOTTOMLESS MAG ${clock(state.pAmmo)}</div>`;
  hud.powerups.innerHTML = pu;
  let m = '';
  for (let i = 0; i < state.magSize; i++) {
    m += `<span class="${i < state.mag ? 'live' : 'empty'}">▮</span>`;
  }
  hud.mag.innerHTML = m;
  hud.reloadBar.classList.toggle('active', state.reloading);
  if (state.reloading) hud.reloadFill.style.width = (state.reloadT / state.reloadTime * 100) + '%';
}

export let flashT = 0;
export function damageFlash() {
  hud.flash.style.background = '#ff1e1e';
  flashT = 0.4;
  state.shake = 0.5;
}

export function updateDamageFlash(dt) {
  if (flashT > 0) {
    flashT -= dt;
    hud.flash.style.opacity = Math.max(0, flashT) * 0.6;
  }
}

export function showToast(text, color) {
  hud.toast.textContent = text;
  hud.toast.style.color = color;
  hud.toast.style.opacity = 1;
  clearTimeout(showToast._t);
  showToast._t = setTimeout(() => { hud.toast.style.opacity = 0; }, 1400);
}

export function showBanner(text, sub, dur = 1.6) {
  hud.bannerText.textContent = text;
  hud.bannerSub.textContent = sub || '';
  hud.banner.style.opacity = 1;
  clearTimeout(showBanner._t);
  showBanner._t = setTimeout(() => { hud.banner.style.opacity = 0; }, dur * 1000);
}
