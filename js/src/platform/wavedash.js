// ===================================================================
// WAVEDASH — platform SDK (loading screen + leaderboard, achievements & stats)
// ===================================================================
const WV_LEADERBOARD = "high-scores";
let wvLeaderboardId = null;

const getWavedash = () => (typeof window !== 'undefined' ? window.Wavedash : null);

// Submit the run's score (total kills) on death. Defensive: any SDK/network
// failure is logged but never interrupts the game.
//
// We use getOrCreateLeaderboard rather than getLeaderboard so the board is
// provisioned automatically on first use — getLeaderboard only returns a board
// that already exists in the dashboard, so scores were silently dropped until
// one was created by hand. More kills = better → DESC; the score is a plain
// kill count → NUMERIC.
export async function wvSubmitScore(score) {
  const Wavedash = getWavedash();
  if (!Wavedash) { console.warn("[wavedash] no SDK — score not submitted"); return; }
  try {
    if (wvLeaderboardId == null) {
      const lb = await Wavedash.getOrCreateLeaderboard(
        WV_LEADERBOARD,
        Wavedash.LeaderboardSortOrder.DESC,
        Wavedash.LeaderboardDisplayType.NUMERIC,
      );
      if (!lb?.success) {
        console.warn("[wavedash] could not get/create leaderboard:", lb?.message);
        return;
      }
      wvLeaderboardId = lb.data.id;
    }
    const res = await Wavedash.uploadLeaderboardScore(wvLeaderboardId, score, true); // keepBest
    if (!res?.success) {
      console.warn("[wavedash] score upload rejected:", res?.message);
    }
  } catch (e) {
    console.warn("[wavedash] score upload threw", e);
  }
}

// Two kinds of achievements are configured in the Developer Portal
// (your game → Achievements):
//
//   "Triggered in the game"  → we unlock them directly with setAchievement():
//     WAVE_1   survive your first wave
//     WAVE_5   reach wave 5
//     WAVE_10  reach wave 10
//     WAVE_15  reach wave 15
//     PACIFIST die on wave 1 without firing a shot
//
//   "Triggered by Wavedash"  → the platform unlocks them from a stat threshold,
//   so we never call setAchievement for these; we just report the stat:
//     KILLS_100   Total Kills >= 100
//     KILLS_1000  Total Kills >= 1000
//
// setAchievement/setStat with an id not defined in the portal is a harmless
// no-op, which is exactly why the old code failed silently when ids didn't
// match. The Set dedupes so we don't spam the SDK with repeat unlocks.
const wvUnlocked = new Set();
export function wvUnlock(id) {
  const Wavedash = getWavedash();
  if (!Wavedash || wvUnlocked.has(id)) return;
  wvUnlocked.add(id);
  try {
    Wavedash.setAchievement(id, true); // storeNow → persist immediately
  } catch (e) {
    console.warn("[wavedash] achievement unlock failed", id, e);
  }
}

// The KILLS_* achievements trigger off a cumulative "Total Kills" stat. setStat
// overwrites and the stored value persists across sessions, so we read the
// current total and add this kill to it (there is no increment method). This id
// MUST match the stat configured under your game → In-Game Stats.
const WV_STAT_KILLS = "KILLS_TOTAL";
export function wvAddKill() {
  const Wavedash = getWavedash();
  if (!Wavedash) return;
  try {
    Wavedash.setStat(WV_STAT_KILLS, (Wavedash.getStat(WV_STAT_KILLS) || 0) + 1, true);
  } catch (e) {
    console.warn("[wavedash] kill stat failed", e);
  }
}

export function isOnWavedash() {
  const Wavedash = getWavedash();
  if (Wavedash) return true;
  if (typeof window === 'undefined' || typeof location === 'undefined') return false;
  return [location.href, document.referrer, ...(location.ancestorOrigins || [])].some(u => {
    try { return /(^|\.)wavedash\.com$/.test(new URL(u).hostname); } catch { return false; }
  });
}

export function initWavedash() {
  const Wavedash = getWavedash();
  if (Wavedash) {
    try {
      Wavedash.init({ debug: true });
    } catch (e) {
      console.warn("[wavedash] init failed", e);
    }
  }
}
