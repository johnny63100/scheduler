<script lang="ts">
  // Budget tab of the money book: a monthly spending target per category (the same every month),
  // spending against each target, overall numbers, and the last six months.
  import { CalendarClock, Check, Gauge, PiggyBank, Pencil, Target, TrendingUp, X } from '@lucide/svelte';
  import StatCard from './StatCard.svelte';
  import { api, type Expense } from '../lib/api';
  import { addMonths, fmt, monthStart, parseYmd, today } from '../lib/dates';
  import { categoriesFor, categoryBudgetTotal, expenseSettings, fmtMoney, fmtMoneyShort, saveCategoryBudgets, type Category } from '../lib/expenses.svelte';
  import { t } from '../lib/i18n.svelte';
  import { data, load, toast } from '../lib/state.svelte';

  let { month, spending }: { month: string; spending: Expense[] } = $props();

  const MONTHS = 6;
  const daysInMonth = $derived(new Date(parseYmd(month).getFullYear(), parseYmd(month).getMonth() + 1, 0).getDate());
  const isCurrentMonth = $derived(month === monthStart(today()));
  const isFuture = $derived(month > monthStart(today()));
  /** Days of the month that have passed, counting today. */
  const daysSoFar = $derived(isFuture ? 0 : isCurrentMonth ? parseYmd(today()).getDate() : daysInMonth);
  const daysLeft = $derived(daysInMonth - daysSoFar + (isCurrentMonth ? 1 : 0));
  /** How far through the month we are (0–1); the "on pace" line on each bar. */
  const elapsed = $derived(isCurrentMonth ? daysSoFar / daysInMonth : isFuture ? 0 : 1);

  const budgets = $derived(expenseSettings.categoryBudgets);
  const totalBudget = $derived(categoryBudgetTotal());
  const hasBudgets = $derived(totalBudget > 0);

  const spentBy = $derived.by(() => {
    const map: Record<string, number> = {};
    for (const e of spending) map[e.category] = (map[e.category] ?? 0) + e.amount;
    return map;
  });
  const spent = $derived(spending.reduce((s, e) => s + e.amount, 0));
  /** Spending in categories that have a target (the total compares against these only). */
  const spentBudgeted = $derived(Object.keys(budgets).reduce((s, c) => s + (spentBy[c] ?? 0), 0));
  const spentOutside = $derived(spent - spentBudgeted);
  const left = $derived(totalBudget - spentBudgeted);
  const used = $derived(totalBudget ? spentBudgeted / totalBudget : 0);
  const projected = $derived(isCurrentMonth && daysSoFar > 0 ? (spentBudgeted / daysSoFar) * daysInMonth : spentBudgeted);
  const perDayLeft = $derived(isCurrentMonth && daysLeft > 0 ? Math.max(0, left) / daysLeft : 0);

  type Row = { id: string; label: string; color: string; icon: Category['icon']; budget: number; spent: number };
  const rows = $derived<Row[]>(
    categoriesFor('expense', true).map((c) => ({ id: c.id, label: c.name, color: c.color, icon: c.icon, budget: budgets[c.id] ?? 0, spent: spentBy[c.id] ?? 0 }))
      .filter((r) => r.budget > 0 || r.spent > 0)
      .sort((a, b) => Number(b.budget > 0) - Number(a.budget > 0) || ratio(b) - ratio(a) || b.spent - a.spent),
  );
  const overCount = $derived(rows.filter((r) => r.budget > 0 && r.spent > r.budget).length);
  const budgetedCount = $derived(rows.filter((r) => r.budget > 0).length);
  function ratio(r: Row) {
    return r.budget > 0 ? r.spent / r.budget : 0;
  }
  const pct = (n: number) => `${Math.round(n * 100)}%`;

  // ---- last six months (compared with today's targets)
  let history = $state<Expense[]>([]);
  $effect(() => {
    data.version;
    const from = addMonths(month, -(MONTHS - 1));
    const end = month.slice(0, 8) + String(daysInMonth).padStart(2, '0');
    load(api.expensesBetween(from, end), []).then((list) => (history = list.filter((e) => e.kind !== 'income')));
  });
  const trend = $derived.by(() => {
    const keys = Object.keys(budgets);
    return Array.from({ length: MONTHS }, (_, i) => {
      const m = addMonths(month, i - (MONTHS - 1));
      const list = history.filter((e) => e.date.startsWith(m.slice(0, 7)));
      const inBudget = list.filter((e) => keys.includes(e.category)).reduce((s, e) => s + e.amount, 0);
      const all = list.reduce((s, e) => s + e.amount, 0);
      return { month: m, label: fmt(m, { month: 'short' }), value: hasBudgets ? inBudget : all, all };
    });
  });
  const trendMax = $derived(Math.max(totalBudget, ...trend.map((m) => m.value), 1));
  const pastMonths = $derived(trend.filter((m) => m.month < monthStart(today()) && m.all > 0));
  const withinMonths = $derived(pastMonths.filter((m) => m.value <= totalBudget).length);
  const avgSpent = $derived(pastMonths.length ? pastMonths.reduce((s, m) => s + m.value, 0) / pastMonths.length : 0);

  // ---- editing targets
  let editing = $state(false);
  let form = $state<Record<string, string>>({});
  function startEdit() {
    form = Object.fromEntries(categoriesFor('expense').map((c) => [c.id, budgets[c.id] ? String(budgets[c.id]) : '']));
    editing = true;
  }
  const formTotal = $derived(Object.values(form).reduce((s, v) => s + (Number(v) > 0 ? Number(v) : 0), 0));
  async function saveTargets() {
    const next: Record<string, number> = {};
    for (const [id, v] of Object.entries(form)) if (Number(v) > 0) next[id] = Number(v);
    await saveCategoryBudgets(next);
    editing = false;
    toast(t('bd.saved'), 'success');
  }
</script>

<div class="stats">
  <StatCard label={t('bd.total')} value={hasBudgets ? fmtMoney(totalBudget) : '—'} sub={hasBudgets ? t('bd.categories', { n: budgetedCount }) : t('bd.noneYet')} icon={Target} />
  <StatCard
    label={t('bd.spent')}
    value={fmtMoney(spentBudgeted)}
    sub={hasBudgets ? t('bd.usedPct', { pct: pct(used) }) + (spentOutside > 0 ? ` · ${t('bd.outside', { amount: fmtMoney(spentOutside) })}` : '') : ''}
    icon={Gauge}
    tint={used > 1 ? 'var(--danger)' : used > elapsed + 0.1 ? 'var(--warning)' : 'var(--primary)'}
  />
  <StatCard
    label={left < 0 ? t('bd.over') : t('bd.left')}
    value={hasBudgets ? fmtMoney(Math.abs(left)) : '—'}
    sub={hasBudgets && isCurrentMonth && left > 0 ? t('bd.perDay', { amount: fmtMoney(perDayLeft), n: daysLeft }) : ''}
    icon={PiggyBank}
    tint={left < 0 ? 'var(--danger)' : 'var(--success)'}
  />
  <StatCard
    label={isCurrentMonth ? t('bd.projected') : t('bd.result')}
    value={hasBudgets ? fmtMoney(projected) : '—'}
    sub={hasBudgets
      ? projected > totalBudget
        ? t('bd.projOver', { amount: fmtMoney(projected - totalBudget) })
        : t('bd.projUnder', { amount: fmtMoney(totalBudget - projected) })
      : ''}
    icon={isCurrentMonth ? CalendarClock : TrendingUp}
    tint={projected > totalBudget ? 'var(--danger)' : 'var(--success)'}
  />
</div>

<div class="layout">
  <div class="card">
    <div class="card-head">
      <h3>{t('bd.byCategory')}</h3>
      {#if editing}
        <span class="faint small">{t('bd.formTotal', { amount: fmtMoney(formTotal) })}</span>
        <button class="btn small" onclick={() => (editing = false)}><X size={15} /> {t('common.cancel')}</button>
        <button class="btn small primary" onclick={saveTargets}><Check size={15} /> {t('common.save')}</button>
      {:else}
        <button class="btn small" onclick={startEdit}><Pencil size={14} /> {hasBudgets ? t('bd.edit') : t('bd.set')}</button>
      {/if}
    </div>

    {#if editing}
      <p class="faint small hint">{t('bd.editHint')}</p>
      <div class="edit-grid">
        {#each categoriesFor('expense') as c (c.id)}
          <label class="edit-row" style:--c={c.color}>
            <span class="cat-icon"><c.icon size={15} /></span>
            <span class="name truncate">{c.name}</span>
            <input class="input" type="number" inputmode="numeric" min="0" step="any" placeholder="0" bind:value={form[c.id]} />
          </label>
        {/each}
      </div>
    {:else if rows.length === 0 && !hasBudgets}
      <div class="empty-budget">
        <Target size={26} />
        <p class="strong">{t('bd.emptyTitle')}</p>
        <p class="muted small">{t('bd.emptyBody')}</p>
        <button class="btn primary" onclick={startEdit}><Pencil size={15} /> {t('bd.set')}</button>
      </div>
    {:else}
      {#if isCurrentMonth && hasBudgets}
        <p class="faint small hint"><span class="pace-key"></span> {t('bd.paceHint', { pct: pct(elapsed) })}</p>
      {/if}
      <div class="rows">
        {#each rows as r (r.id)}
          {@const over = r.budget > 0 && r.spent > r.budget}
          <div class="row" style:--c={r.color}>
            <span class="cat-icon"><r.icon size={15} /></span>
            <div class="main">
              <div class="line">
                <span class="name">{r.label}</span>
                <span class="tabular amt">
                  <b>{fmtMoney(r.spent)}</b>
                  {#if r.budget > 0}<span class="faint"> / {fmtMoney(r.budget)}</span>{/if}
                </span>
              </div>
              {#if r.budget > 0}
                <div class="track" class:over>
                  <span class="fill" style:width="{Math.min(100, ratio(r) * 100)}%"></span>
                  {#if isCurrentMonth}<span class="pace" style:left="{elapsed * 100}%"></span>{/if}
                </div>
                <div class="foot small">
                  <span class:danger={over} class:muted={!over}>
                    {over ? t('bd.catOver', { amount: fmtMoney(r.spent - r.budget) }) : t('bd.catLeft', { amount: fmtMoney(r.budget - r.spent) })}
                  </span>
                  <span class="faint tabular">{pct(ratio(r))}</span>
                </div>
              {:else}
                <div class="track none"><span class="fill" style:width="100%"></span></div>
                <div class="foot small"><span class="faint">{t('bd.noTarget')}</span></div>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <div class="card">
    <div class="card-head">
      <h3>{t('bd.trend')}</h3>
    </div>
    <div class="chart" role="img" aria-label={t('bd.trend')}>
      {#each trend as m, i (m.month)}
        <div class="col" class:current={m.month === month}>
          <span class="val tabular">{m.value ? fmtMoneyShort(m.value) : ''}</span>
          <div class="bar-area">
            <span class="bar" class:over={hasBudgets && m.value > totalBudget} style:height="{(m.value / trendMax) * 100}%"></span>
            {#if hasBudgets}
              <span class="budget-line" style:bottom="{(totalBudget / trendMax) * 100}%">
                {#if i === trend.length - 1}<em>{fmtMoneyShort(totalBudget)}</em>{/if}
              </span>
            {/if}
          </div>
          <span class="lbl">{m.label}</span>
        </div>
      {/each}
    </div>
    {#if hasBudgets}
      <div class="trend-stats">
        <div><span class="muted small">{t('bd.avg')}</span><b class="tabular">{pastMonths.length ? fmtMoney(avgSpent) : '—'}</b></div>
        <div><span class="muted small">{t('bd.within')}</span><b class="tabular">{pastMonths.length ? `${withinMonths} / ${pastMonths.length}` : '—'}</b></div>
        <div><span class="muted small">{t('bd.overCats')}</span><b class="tabular" class:danger={overCount > 0}>{overCount} / {budgetedCount}</b></div>
      </div>
      <p class="faint small">{t('bd.trendNote')}</p>
    {:else}
      <p class="faint small">{t('bd.trendNoBudget')}</p>
    {/if}
  </div>
</div>

<style>
  .stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(200px, 100%), 1fr));
    gap: 14px;
    margin-bottom: 16px;
  }
  .layout {
    display: grid;
    grid-template-columns: minmax(0, 1.35fr) minmax(0, 1fr);
    gap: 16px;
    align-items: start;
  }
  @media (max-width: 980px) {
    .layout {
      grid-template-columns: 1fr;
    }
  }
  .card-head {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 12px;
  }
  .card-head h3 {
    margin: 0;
    flex: 1;
  }
  .hint {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: -4px 0 12px;
  }
  .pace-key {
    width: 2px;
    height: 12px;
    background: var(--text);
    opacity: 0.45;
    border-radius: 1px;
  }
  .cat-icon {
    width: 30px;
    height: 30px;
    flex: none;
    border-radius: 10px;
    display: grid;
    place-items: center;
    color: var(--c);
    background: color-mix(in srgb, var(--c) 14%, transparent);
  }
  .rows {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .row {
    display: flex;
    gap: 12px;
    align-items: flex-start;
  }
  .main {
    flex: 1;
    min-width: 0;
  }
  .line {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 6px;
  }
  .name {
    font-weight: 600;
    flex: 1;
    min-width: 0;
  }
  .amt {
    font-size: 13px;
    white-space: nowrap;
  }
  .track {
    position: relative;
    height: 10px;
    border-radius: 99px;
    background: var(--surface-3);
    overflow: visible;
  }
  .fill {
    display: block;
    height: 100%;
    border-radius: 99px;
    background: var(--c);
    transition: width 0.3s ease;
  }
  .track.over .fill {
    background: var(--danger);
  }
  .track.none .fill {
    background: color-mix(in srgb, var(--c) 30%, transparent);
  }
  .pace {
    position: absolute;
    top: -3px;
    bottom: -3px;
    width: 2px;
    margin-left: -1px;
    border-radius: 1px;
    background: var(--text);
    opacity: 0.45;
  }
  .foot {
    display: flex;
    justify-content: space-between;
    margin-top: 5px;
  }
  .danger {
    color: var(--danger);
  }
  .edit-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
    gap: 8px 14px;
  }
  .edit-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .edit-row .input {
    width: 130px;
    text-align: right;
  }
  .empty-budget {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 26px 10px;
    text-align: center;
    color: var(--muted);
  }
  .empty-budget p {
    margin: 0;
  }
  .empty-budget .btn {
    margin-top: 8px;
  }
  .chart {
    display: flex;
    gap: 10px;
    height: 200px;
    padding-top: 18px;
    margin-bottom: 14px;
  }
  .col {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    min-width: 0;
  }
  .bar-area {
    position: relative;
    flex: 1;
    width: 100%;
    display: flex;
    align-items: flex-end;
    justify-content: center;
  }
  .bar {
    width: min(34px, 70%);
    border-radius: 7px 7px 3px 3px;
    background: color-mix(in srgb, var(--primary) 35%, transparent);
    transition: height 0.3s ease;
  }
  .col.current .bar {
    background: var(--primary);
  }
  .bar.over {
    background: color-mix(in srgb, var(--danger) 45%, transparent);
  }
  .col.current .bar.over {
    background: var(--danger);
  }
  .val {
    font-size: 10.5px;
    color: var(--muted);
    height: 14px;
  }
  .lbl {
    font-size: 12px;
    color: var(--muted);
  }
  .col.current .lbl {
    color: var(--text);
    font-weight: 700;
  }
  .budget-line {
    position: absolute;
    left: -5px;
    right: -5px;
    border-top: 1.5px dashed color-mix(in srgb, var(--text) 35%, transparent);
    pointer-events: none;
    z-index: 1;
  }
  .budget-line em {
    position: absolute;
    right: 0;
    top: -17px;
    font-style: normal;
    font-size: 10.5px;
    color: var(--muted);
    background: var(--surface);
    padding: 0 4px;
  }
  .trend-stats {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
    margin-bottom: 10px;
  }
  .trend-stats div {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .trend-stats b {
    font-size: 16px;
  }
</style>
