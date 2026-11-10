// Self-update from this fork's GitHub releases (tauri-plugin-updater).
// Checked when the app starts (and on demand in Settings → About); can be turned off.
// The installer is signed in CI and verified against the public key in tauri.conf.json.
import { getVersion } from '@tauri-apps/api/app';
import { relaunch } from '@tauri-apps/plugin-process';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { api } from './api';
import { t } from './i18n.svelte';
import { toast } from './state.svelte';

const SETTING = 'updates';
/** Minimum time between automatic checks. */
const CHECK_EVERY_MS = 10 * 60 * 1000;
let pending: Update | null = null;
let installing = false;

export const updates = $state({
  /** Check by itself when the app starts (once a day). */
  auto: true,
  current: '',
  latest: null as string | null,
  /** The release page of `latest`. */
  url: null as string | null,
  status: 'idle' as 'idle' | 'checking' | 'latest' | 'newer' | 'failed',
  checkedAt: 0,
});

/** "0.10.0" > "0.9.2": compares number by number. */
export function isNewer(latest: string, current: string): boolean {
  const a = latest.split('.').map(Number);
  const b = current.split('.').map(Number);
  for (let i = 0; i < Math.max(a.length, b.length); i++) {
    const d = (a[i] ?? 0) - (b[i] ?? 0);
    if (d) return d > 0;
  }
  return false;
}

async function persist() {
  const { auto, latest, url, checkedAt } = updates;
  await api.setSetting(SETTING, JSON.stringify({ auto, latest, url, checkedAt })).catch(() => {});
}

export async function loadUpdates() {
  updates.current = await getVersion().catch(() => '');
  try {
    const saved = JSON.parse((await api.getSetting(SETTING).catch(() => null)) ?? 'null');
    if (saved && typeof saved === 'object') {
      updates.auto = saved.auto !== false;
      updates.checkedAt = Number(saved.checkedAt) || 0;
      if (typeof saved.latest === 'string') updates.latest = saved.latest;
      if (typeof saved.url === 'string') updates.url = saved.url;
    }
  } catch {
    /* defaults */
  }
  if (updates.latest && updates.current) updates.status = isNewer(updates.latest, updates.current) ? 'newer' : 'latest';
}

/** Downloads the found update, installs it and restarts the app. */
export async function installUpdate() {
  if (installing) return;
  if (!pending) {
    await checkForUpdates(true);
    if (!pending) return;
  }
  installing = true;
  toast(t('upd.installing'), 'info');
  try {
    await pending.downloadAndInstall();
    await relaunch();
  } catch (e) {
    toast(String(e), 'error');
  } finally {
    installing = false;
  }
}

/** Kept for the Settings button: now installs instead of opening a web page. */
export const openReleasePage = () => installUpdate();

/** Asks GitHub for the latest version. `manual`: from the Settings button (always checks, reports the result). */
export async function checkForUpdates(manual = false) {
  if (updates.status === 'checking') return;
  if (!manual && (!updates.auto || Date.now() - updates.checkedAt < CHECK_EVERY_MS)) return;
  updates.status = 'checking';
  try {
    if (!updates.current) updates.current = await getVersion();
    pending = await check();
    updates.checkedAt = Date.now();
    updates.latest = pending?.version ?? updates.current;
    updates.url = null;
    const newer = !!pending;
    updates.status = newer ? 'newer' : 'latest';
    await persist();
    if (newer) {
      toast(t('upd.available', { v: pending!.version }), 'info', { label: t('upd.see'), run: installUpdate });
    } else if (manual) {
      toast(t('upd.upToDate'), 'success');
    }
  } catch {
    // Offline or GitHub unreachable: try again next time, quietly unless asked.
    updates.status = 'failed';
    if (manual) toast(t('upd.failed'), 'error');
  }
}

export function setAutoCheck(on: boolean) {
  updates.auto = on;
  persist();
}
