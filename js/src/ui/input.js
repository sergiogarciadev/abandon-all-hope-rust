import * as THREE from 'three';
import { state } from '../core/state.js';
import { audio } from '../audio/audio.js';

export const keys = {};
export const mouse = new THREE.Vector2();
export let firing = false;
export function setFiring(val) {
  firing = val;
}

export const aimPoint = new THREE.Vector3();
export const aimPlane = new THREE.Plane(new THREE.Vector3(0, 1, 0), -0.9);
export const raycaster = new THREE.Raycaster();

export function setupInputListeners({
  onStartGame,
  onSetPaused,
  onQuitToTitle,
  onReload,
  onChoose,
  menuKey,
}) {
  window.addEventListener('keydown', e => {
    if (menuKey?.(e)) { e.preventDefault(); return; }
    if (e.code === 'Enter' && !state.running && !document.getElementById('death').classList.contains('hidden')) {
      return onStartGame();
    }
    keys[e.code] = true;
    if (e.code === 'Escape' || e.code === 'KeyP') {
      if (!e.repeat) onSetPaused(!state.paused);
      return;
    }
    if (state.paused) return;
    if (e.code === 'KeyR') onReload();
    if (state.choosing && /^Digit[1-3]$/.test(e.code)) onChoose(+e.code.slice(5) - 1);
    if (e.code === 'KeyM') {
      const m = audio.toggleMute();
      document.getElementById('reloadHint').textContent = m ? '[R] RELOAD   ·   [M] MUTED' : '[R] RELOAD';
    }
  });

  window.addEventListener('keyup', e => { keys[e.code] = false; });
  window.addEventListener('mousemove', e => {
    mouse.x = (e.clientX / window.innerWidth) * 2 - 1;
    mouse.y = -(e.clientY / window.innerHeight) * 2 + 1;
  });
  window.addEventListener('mousedown', () => { firing = true; });
  window.addEventListener('mouseup', () => { firing = false; });
  window.addEventListener('contextmenu', e => e.preventDefault());

  // the death screen: go again, or back to the title
  document.getElementById('retryBtn').addEventListener('click', () => onStartGame());
  document.getElementById('deathQuitBtn').addEventListener('click', onQuitToTitle);
  document.getElementById('resumeBtn').addEventListener('click', () => onSetPaused(false));
  document.getElementById('quitBtn').addEventListener('click', onQuitToTitle);

  // alt-tabbing away mid-run pauses (and would otherwise leave WASD stuck down)
  window.addEventListener('blur', () => onSetPaused(true));

  document.getElementById('cards').addEventListener('click', e => {
    const c = e.target.closest('.card');
    if (c) onChoose(+c.dataset.i);
  });
  document.getElementById('cards').addEventListener('mouseover', e => {
    const c = e.target.closest('.card');
    if (c && !c.contains(e.relatedTarget) && state.choosing) audio.hover(+c.dataset.i);
  });
}
