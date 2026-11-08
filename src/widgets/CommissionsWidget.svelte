<script lang="ts">
  import { onMount } from 'svelte';
  import { Brush } from '@lucide/svelte';
  import { api, type Commission } from '../lib/api';
  import { byDeadline, commissionFocus, daysLeft, stageOf, unpaidOf } from '../lib/commissions.svelte';
  import { today } from '../lib/dates';
  import { expenseSettings, fmtMoney, loadExpenseSettings } from '../lib/expenses.svelte';
  import { t } from '../lib/i18n.svelte';
  import { data, load, ui } from '../lib/state.svelte';

  let commissions = $state<Commission[]>([]);

  onMount(() => {
    if (!expenseSettings.loaded) loadExpenseSettings();
  });

  $effect(() => {
    data.version;
    load(api.commissions(), []).then((c) => (commissions = c));
  });

  const todayStr = $derived(today());
  const upcoming = $derived(commissions.filter((c) => c.deadline && c.status !== 'done').sort(byDeadline).slice(0, 4));
  const unpaid = $derived(commissions.filter((c) => c.status !== 'inquiry').reduce((s, c) => s + unpaidOf(c), 0));

  function due(c: Commission) {
    const n = daysLeft(c, todayStr) ?? 0;
    return n === 0 ? t('cm.dueToday') : n > 0 ? `D-${n}` : t('cm.overdue', { n: -n });
  }

  function open(c: Commission) {
    commissionFocus.id = c.id;
    ui.page = 'commissions';
  }
</script>

<div class="widget">
  <div class="w-head">
    <button class="w-title" onclick={() => (ui.page = 'commissions')}><span class="w-icon"><Brush size={15} /></span>{t('w.commissions')}</button>
  </div>
  <div class="w-body">
    {#if unpaid > 0 && expenseSettings.loaded}<div class="unpaid small">{t('w.commissions.unpaid', { amount: fmtMoney(unpaid) })}</div>{/if}
    {#each upcoming as c (c.id)}
      {@const n = daysLeft(c, todayStr) ?? 0}
      <button class="row" style:--c={stageOf(c.status).color} onclick={() => open(c)}>
        <span class="dot"></span>
        <span class="truncate">{c.title}</span>
        <span class="due" class:late={n < 0} class:soon={n >= 0 && n <= 3}>{due(c)}</span>
      </button>
    {:else}
      <div class="w-empty">{t('w.commissions.empty')}</div>
    {/each}
  </div>
</div>

<style>
  .unpaid {
    margin-bottom: 6px;
    font-weight: 650;
    color: var(--warning);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    min-width: 0;
    padding: 6px 4px;
    border: none;
    border-radius: 8px;
    background: none;
    font-size: 13px;
    font-weight: 600;
    text-align: left;
    cursor: pointer;
  }
  .row:hover {
    background: var(--surface-2);
  }
  .row .truncate {
    flex: 1;
  }
  .dot {
    width: 8px;
    height: 8px;
    flex: none;
    border-radius: 50%;
    background: var(--c);
  }
  .due {
    flex: none;
    font-size: 12px;
    font-weight: 700;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .due.soon {
    color: var(--warning);
  }
  .due.late {
    color: var(--danger);
  }
</style>
