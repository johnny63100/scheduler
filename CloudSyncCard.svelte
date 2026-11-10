<script lang="ts">
  // Settings card: sign in to Supabase so the phone page (mobile/index.html) can see and add
  // events and todos. The sync itself runs in Rust (src-tauri/src/cloud.rs).
  import { onDestroy, onMount } from 'svelte';
  import { Cloud, CloudOff, LoaderCircle, LogOut, RefreshCw, TriangleAlert } from '@lucide/svelte';
  import { api, type CloudStatus } from '../lib/api';
  import { t } from '../lib/i18n.svelte';
  import { toast } from '../lib/state.svelte';
  import { fmtTs } from '../lib/dates';

  let status = $state<CloudStatus | null>(null);
  let form = $state({ url: '', anonKey: '', email: '', password: '' });
  let busy = $state(false);
  let timer: ReturnType<typeof setInterval> | undefined;

  async function refresh() {
    try {
      status = await api.cloudStatus();
      if (!form.url) form.url = status.url;
      if (!form.email) form.email = status.email;
    } catch {
      status = null;
    }
  }

  onMount(() => {
    refresh();
    timer = setInterval(refresh, 5000);
  });
  onDestroy(() => clearInterval(timer));

  async function signIn() {
    if (!form.url || !form.anonKey || !form.email || !form.password) return;
    busy = true;
    try {
      await api.cloudSignIn(form.url, form.anonKey, form.email, form.password);
      form.password = '';
      toast(t('cloud.signedInToast'), 'success');
      await refresh();
    } catch (e) {
      toast(String(e), 'error');
    } finally {
      busy = false;
    }
  }

  async function signOut() {
    await api.cloudSignOut().catch((e) => toast(String(e), 'error'));
    await refresh();
  }

  async function syncNow() {
    await api.cloudSyncNow().catch(() => {});
    setTimeout(refresh, 2500);
  }

  const lastPush = $derived(status?.last_push ? fmtTs(Math.floor(Date.parse(status.last_push) / 1000), { month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit', hour12: false }) : null);
</script>

<section class="card">
  <div class="section-head">
    <div class="icon">{#if status?.configured}<Cloud size={18} />{:else}<CloudOff size={18} />{/if}</div>
    <div>
      <h2>{t('cloud.title')}</h2>
      <p class="muted small">{t('cloud.body')}</p>
    </div>
  </div>

  {#if status?.configured}
    <div class="row status-row">
      <span class="strong">{t('cloud.signedIn', { email: status.email })}</span>
      <span class="muted small">{lastPush ? t('cloud.lastPush', { t: lastPush }) : t('cloud.never')}</span>
      <span class="spacer"></span>
      <button class="btn" onclick={syncNow}><RefreshCw size={16} /> {t('cloud.syncNow')}</button>
      <button class="btn ghost" onclick={signOut}><LogOut size={16} /> {t('cloud.signOut')}</button>
    </div>
    {#if status.last_error}
      <p class="error small"><TriangleAlert size={14} /> {status.last_error}</p>
    {/if}
  {:else}
    <div class="grid">
      <div class="field">
        <label for="cloud-url">{t('cloud.url')}</label>
        <input id="cloud-url" class="input" placeholder="https://xxxx.supabase.co" bind:value={form.url} autocomplete="off" />
      </div>
      <div class="field">
        <label for="cloud-key">{t('cloud.anonKey')}</label>
        <input id="cloud-key" class="input" placeholder="sb_publishable_… / eyJ…" bind:value={form.anonKey} autocomplete="off" />
      </div>
      <div class="field">
        <label for="cloud-email">{t('cloud.email')}</label>
        <input id="cloud-email" class="input" type="email" bind:value={form.email} autocomplete="username" />
      </div>
      <div class="field">
        <label for="cloud-pw">{t('cloud.password')}</label>
        <input id="cloud-pw" class="input" type="password" bind:value={form.password} autocomplete="current-password"
          onkeydown={(e) => e.key === 'Enter' && signIn()} />
      </div>
    </div>
    <div class="row">
      <span class="faint small">{t('cloud.help')}</span>
      <span class="spacer"></span>
      <button class="btn primary" onclick={signIn} disabled={busy || !form.url || !form.anonKey || !form.email || !form.password}>
        {#if busy}<span class="spin"><LoaderCircle size={16} /></span> {t('cloud.signingIn')}{:else}<Cloud size={16} /> {t('cloud.signIn')}{/if}
      </button>
    </div>
  {/if}
</section>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 10px 14px;
    margin: 12px 0;
  }
  .status-row {
    margin-top: 12px;
    flex-wrap: wrap;
    gap: 10px;
  }
  .spacer {
    flex: 1;
  }
  .error {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 10px 0 0;
    color: var(--danger, #e5484d);
  }
  .spin {
    display: inline-flex;
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
