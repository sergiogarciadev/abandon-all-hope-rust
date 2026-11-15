import { state } from '../core/state.js';
import { pixelIcon } from '../core/math.js';
import { audio } from '../audio/audio.js';
import { hud, showToast } from './hud.js';

export const DROP_CHANCE = 0.05;

// UPGRADES — pick one of three after every cleared wave
// Each card can be taken up to `max` times per run. `ok` (optional) hides a
// card while it would do nothing. `stat` returns [now, after] for the card.
export const UPGRADES = [
  {
    id: 'mag',
    cat: 'handling',
    name: 'EXTENDED MAG',
    desc: '+4 rounds per magazine',
    max: 5,
    stat: s => [s.magSize, s.magSize + 4 + ' rds'],
    apply: s => { s.magSize += 4; s.mag = s.magSize; }
  },
  {
    id: 'reload',
    cat: 'handling',
    name: 'QUICK HANDS',
    desc: 'reload 25% faster',
    max: 4,
    stat: s => [s.reloadTime.toFixed(2), (s.reloadTime * 0.75).toFixed(2) + 's'],
    apply: s => { s.reloadTime *= 0.75; }
  },
  {
    id: 'damage',
    cat: 'firepower',
    name: 'HOLLOW POINTS',
    desc: '+50% bullet damage',
    max: 6,
    stat: s => [s.damage + '×', s.damage + 0.5 + '×'],
    apply: s => { s.damage += 0.5; }
  },
  {
    id: 'rate',
    cat: 'firepower',
    name: 'HAIR TRIGGER',
    desc: 'fire 20% faster',
    max: 4,
    stat: s => [(1 / s.fireRate).toFixed(1), (1 / (s.fireRate * 0.8)).toFixed(1) + '/s'],
    apply: s => { s.fireRate *= 0.8; }
  },
  {
    id: 'pierce',
    cat: 'firepower',
    name: 'PIERCING',
    desc: 'shots pierce +1 zombie',
    max: 3,
    stat: s => [s.pierce, s.pierce + 1 + ' thru'],
    apply: s => { s.pierce++; }
  },
  {
    id: 'spread',
    cat: 'firepower',
    name: 'BUCKSHOT',
    desc: '+1 bullet per shot',
    max: 3,
    stat: s => [1 + s.spread, 2 + s.spread + ' per shot'],
    apply: s => { s.spread++; }
  },
  {
    id: 'hp',
    cat: 'survival',
    name: 'THICK SKIN',
    desc: '+1 max health, and heal it',
    max: 3,
    stat: s => [s.maxHp, s.maxHp + 1 + ' max hp'],
    apply: s => { s.maxHp++; s.hp++; }
  },
  {
    id: 'speed',
    cat: 'survival',
    name: 'FLEET FOOT',
    desc: 'move 12% faster',
    max: 3,
    stat: s => [Math.round(s.speed / 6.2 * 100) + '%', Math.round(s.speed * 1.12 / 6.2 * 100) + '%'],
    apply: s => { s.speed *= 1.12; }
  },
  {
    id: 'luck',
    cat: 'fortune',
    name: 'SCAVENGER',
    desc: 'more supply drops',
    max: 3,
    stat: s => [Math.round((DROP_CHANCE + s.dropBonus) * 100) + '%', Math.round((DROP_CHANCE + s.dropBonus + 0.03) * 100) + '% drops'],
    apply: s => { s.dropBonus += 0.03; }
  },
  {
    id: 'heal',
    cat: 'survival',
    name: 'PATCH UP',
    desc: 'restore all health',
    max: Infinity,
    ok: s => s.hp < s.maxHp,
    stat: s => [s.hp, s.maxHp + ' hp'],
    apply: s => { s.hp = s.maxHp; }
  },
];

export const UPGRADE_CATS = {
  firepower: '#d6453c',
  handling:  '#e8d28a',
  survival:  '#8fae6b',
  fortune:   '#6fa8dc',
};

// 8x8 pixel icons in the game's blocky style: X = solid, o = dim
export const UPGRADE_ICONS = {
  mag:    [
    'XX.XX.XX',
    'XX.XX.XX',
    'oo.oo.oo',
    'oo.oo.oo',
    'oo.oo.oo',
    'oo.oo.oo',
    'XX.XX.XX',
    '........'
  ],
  reload: [
    '..XXXX..',
    '.X....XX',
    'X....XXX',
    'X.......',
    'X......X',
    'X......X',
    '.X....X.',
    '..XXXX..'
  ],
  damage: [
    '.XXXXXX.',
    'XXXXXXXX',
    'X..XX..X',
    'X..XX..X',
    'XXXXXXXX',
    '.XXX.XX.',
    '..XXXX..',
    '..X.X.X.'
  ],
  rate:   [
    '....XXX.',
    '...XXX..',
    '..XXX...',
    '.XXXXXX.',
    '...XXX..',
    '..XXX...',
    '.XX.....',
    '.X......'
  ],
  pierce: [
    '.o..o...',
    '.o..o.X.',
    '.o..o..X',
    'XXXXXXXX',
    '.o..o..X',
    '.o..o.X.',
    '.o..o...',
    '........'
  ],
  spread: [
    '......XX',
    '....oo..',
    '..oo....',
    'XXooooXX',
    '..oo....',
    '....oo..',
    '......XX',
    '........'
],
  hp:     [
    'XXXXXXXX',
    'XooooooX',
    'XoooXooX',
    'XooXXXoX',
    '.XooXoX.',
    '.XooooX.',
    '..XooX..',
    '...XX...'
  ],
  speed:  [
    '........',
    'XX..XX..',
    '.XX..XX.',
    '..XX..XX',
    '.XX..XX.',
    'XX..XX..',
    '........',
    '........'
  ],
  luck:   [
    'XXXXXXXX',
    'XX....XX',
    'X.X..X.X',
    'X..XX..X',
    'X..XX..X',
    'X.X..X.X',
    'XX....XX',
    'XXXXXXXX'
  ],
  heal:   [
    '.XX..XX.',
    'XXXXXXXX',
    'XXXXXXXX',
    'XXXXXXXX',
    '.XXXXXX.',
    '..XXXX..',
    '...XX...',
    '........'
  ],
};

export let offered = [];
export const PICK_LOCKOUT_MS = 650; // ~ when the last card finishes dealing in

export function openPicker(healed) {
  const pool = UPGRADES.filter(u => (state.upgrades[u.id] || 0) < u.max && (!u.ok || u.ok(state)));
  for (let i = pool.length - 1; i > 0; i--) { const j = (Math.random() * (i + 1)) | 0; [pool[i], pool[j]] = [pool[j], pool[i]]; }
  offered = pool.slice(0, 3);
  if (!offered.length) { state.intermission = 3.2; return; } // everything maxed out
  state.choosing = true;
  clearTimeout(choose._t);
  clearTimeout(openPicker._t);
  hud.pickerTitle.textContent = 'WAVE ' + state.wave + ' CLEARED';
  hud.pickerSub.textContent = (healed ? '+1 HEALTH  ·  ' : '') + 'CHOOSE ONE';
  hud.cards.innerHTML = offered.map((u, i) => {
    const owned = state.upgrades[u.id] || 0;
    const [now, after] = u.stat(state);
    const pips = u.max === Infinity ? '' :
      Array.from({ length: u.max }, (_, k) => `<i class="${k < owned ? 'own' : k === owned ? 'next' : ''}"></i>`).join('');
    return `<div class="card" data-i="${i}" style="--accent:${UPGRADE_CATS[u.cat]};animation-delay:${i * 90}ms">` +
      `<div class="ctop"><span>${u.cat.toUpperCase()}</span><span class="ckey">${i + 1}</span></div>` +
      pixelIcon(UPGRADE_ICONS[u.id]) +
      `<div class="cname">${u.name}</div><div class="cdesc">${u.desc}</div>` +
      `<div class="cstat">${now} → <b>${after}</b></div>` +
      `<div class="clvl">${pips}</div></div>`;
  }).join('');
  hud.picker.classList.remove('hidden');
  hud.cards.classList.remove('ready');
  audio.deal(offered.length);
  openPicker._t = setTimeout(() => hud.cards.classList.add('ready'), PICK_LOCKOUT_MS);
}

export function choose(i) {
  const u = offered[i];
  if (!state.choosing || !u) return;
  state.upgrades[u.id] = (state.upgrades[u.id] || 0) + 1;
  u.apply(state);
  state.choosing = false;
  // brief beat so the pick registers visually before the picker closes
  [...hud.cards.children].forEach((c, k) => c.classList.add(k === i ? 'picked' : 'dropped'));
  clearTimeout(choose._t);
  choose._t = setTimeout(() => hud.picker.classList.add('hidden'), 320);
  state.intermission = 2.0;
  showToast(u.name, UPGRADE_CATS[u.cat]);
  audio.upgrade(u.cat);
}
