<script lang="ts">
  // Money book "Year" tab: a whole year at a glance (totals, month by month, categories, no-spend
  // days) followed by every monthly review written under the calendar, so the year can be read back
  // in one place. Categories left out of stats don't count; commission payments count as income
  // when they are linked to the money book.
  import { NotebookPen } from '@lucide/svelte';
  import { api, type Commission, type Expense } from '../lib/api';
  import { netPayment } from '../lib/commissions.svelte';
  import { today } from '../lib/dates';
  import { categoriesFor, countsInStats, expenseSettings, fmtMoney } from '../lib/expenses.svelte';
  import { t } from '../lib/i18n.svelte';
  import { data, load } from '../lib/state.svelte';

  let { year, commissions, onmonth }: { year: number; commissions: Commission[]; onmonth: (month: string) => void } = $props();

  let expenses = $state<Expense[]>([]);
  let reviews = $state<Record<string, string>>({});
  $effect(() => {
    data.version;
    load(api.expensesBetween(`${year}-01-01`, `${year}-12-31`), []).then((e) => (expenses = e));
    api
      .getSetting('expense.reviews')
      .then((raw) => {
        try {
          reviews = raw ? JSON.parse(raw) : {};
        } catch {
          reviews = {};
        }
      })
      .catch(() => {});
  });

  const todayStr = $derived(today());
  const months = $derived(Array.from({ length: 12 }, (_, i) => `${year}-${String(i + 1).padStart(2, '0')}`));
  const counted = $derived(expenses.filter((e) => countsInStats(e.category)));
  const linked = $derived(
    expenseSettings.linkCommissions
      ? commissions.flatMap((c) => c.payments.map((p) => ({ date: p.date, amount: netPayment(c, p.amount) }))).filter((p) => p.date.startsWith(`${year}-`) && p.amount > 0)
      : [],
  );

  /**
   * Days before the first record of the year (before the money book was in use) aren't counted as
   * no-spend days or in the averages.
   */
  const firstDay = $derived(expenses.reduce<string | null>((min, e) => (!min || e.date < min ? e.date : min), null));
  /** Per month: spent, earned, no-spend days (only tracked days that have started). */
  const perMonth = $derived(
    months.map((m) => {
      const spent = counted.filter((e) => e.kind !== 'income' && e.date.startsWith(m)).reduce((s, e) => s + e.amount, 0);
      const earned =
        counted.filter((e) => e.kind === 'income' && e.date.startsWith(m)).reduce((s, e) => s + e.amount, 0) +
        linked.filter((p) => p.date.startsWith(m)).reduce((s, p) => s + p.amount, 0);
      const [y, mo] = m.split('-').map(Number);
      const len = new Date(y, mo, 0).getDate();
      const thisMonth = todayStr.slice(0, 7);
      const until = m < thisMonth ? len : m === thisMonth ? Number(todayStr.slice(8)) : 0;
      const from = !firstDay || firstDay.slice(0, 7) > m ? until + 1 : firstDay.slice(0, 7) === m ? Number(firstDay.slice(8)) : 1;
      const started = Math.max(0, until - from + 1);
      const spentDays = new Set(counted.filter((e) => e.kind !== 'income' && e.date.startsWith(m)).map((e) => e.date));
      let noSpend = 0;
      for (let d = from; d <= until; d++) if (!spentDays.has(`${m}-${String(d).padStart(2, '0')}`)) noSpend++;
      return { m, spent, earned, noSpend, started, done: m < thisMonth };
    }),
  );
  const active = $derived(perMonth.filter((x) => x.started > 0));
  const spentTotal = $derived(perMonth.reduce((s, x) => s + x.spent, 0));
  const earnedTotal = $derived(perMonth.reduce((s, x) => s + x.earned, 0));
  const net = $derived(earnedTotal - spentTotal);
  const avgMonth = $derived(active.length ? spentTotal / active.length : 0);
  const noSpendTotal = $derived(perMonth.reduce((s, x) => s + x.noSpend, 0));
  const daysTotal = $derived(perMonth.reduce((s, x) => s + x.started, 0));
  // Leanest / biggest among finished months (the current one isn't over yet) once there are two.
  const finished = $derived(active.filter((x) => x.done && x.spent > 0));
  const withSpending = $derived(finished.length >= 2 ? finished : active.filter((x) => x.spent > 0));
  const leanest = $derived([...withSpending].sort((a, b) => a.spent - b.spent)[0]);
  const heaviest = $derived([...withSpending].sort((a, b) => b.spent - a.spent)[0]);
  const chartMax = $derived(Math.max(1, ...perMonth.map((x) => Math.max(x.spent, x.earned))));

  const byCategory = $derived.by(() => {
    const list = categoriesFor('expense', true)
      .filter((c) => !c.excluded)
      .map((c) => ({ ...c, value: counted.filter((e) => e.kind !== 'income' && e.category === c.id).reduce((s, e) => s + e.amount, 0) }))
      .filter((c) => c.value > 0)
      .sort((a, b) => b.value - a.value);
    return list;
  });
  const catMax = $derived(byCategory[0]?.value ?? 1);

  const monthName = (m: string) => t('yr.monthN', { n: Number(m.slice(5)) });
  const written = $derived(months.filter((m) => reviews[m]?.trim()));
  const pct = (n: number) => `${Math.round(n * 100)}%`;
</script>

<div class="year">
  <div class="stats">
    <div class="stat"><span class="faint small">{t('yr.spent')}</span><b class="tabular">{fmtMoney(spentTotal)}</b></div>
    <div class="stat"><span class="faint small">{t('yr.earned')}</span><b class="tabular">{fmtMoney(earnedTotal)}</b></div>
    <div class="stat"><span class="faint small">{t('yr.net')}</span><b class="tabular" class:neg={net < 0}>{net >= 0 ? '+' : '−'}{fmtMoney(Math.abs(net))}</b></div>
    <div class="stat"><span class="faint small">{t('yr.avgMonth')}</span><b class="tabular">{fmtMoney(Math.round(avgMonth))}</b></div>
    <div class="stat"><span class="faint small">{t('yr.noSpend')}</span><b class="tabular">{t('mc.days', { n: noSpendTotal })} <span class="faint small">/ {daysTotal}</span></b></div>
    <div class="stat">
      <span class="faint small">{t('yr.leanHeavy')}</span>
      <b class="tabular small-b">{leanest ? monthName(leanest.m) : '—'} <span class="faint">/</span> {heaviest ? monthName(heaviest.m) : '—'}</b>
    </div>
  </div>

  <div class="card">
    <div class="card-head">
      <span class="strong">{t('yr.byMonth')}</span>
      <span class="spacer"></span>
      <span class="legend"><i class="sw spent"></i>{t('yr.spentShort')} <i class="sw earned"></i>{t('yr.earnedShort')}</span>
    </div>
    <div class="months">
      {#each perMonth as x (x.m)}
        <button class="mcol" class:future={x.started === 0 && !x.spent && !x.earned} onclick={() => onmonth(`${x.m}-01`)} title={t('yr.openMonth')}>
          <span class="val tabular">{x.spent ? Math.round(x.spent / 10000).toLocaleString() + t('yr.manSuffix') : ''}</span>
          <span class="bars">
            <span class="bar spent" style:height="{(x.spent / chartMax) * 100}%"></span>
            <span class="bar earned" style:height="{(x.earned / chartMax) * 100}%"></span>
          </span>
          <span class="mlabel">{Number(x.m.slice(5))}</span>
          {#if reviews[x.m]?.trim()}<span class="dot" title={t('yr.hasReview')}></span>{/if}
        </button>
      {/each}
    </div>
  </div>

  <div class="card">
    <div class="card-head"><span class="strong">{t('yr.byCategory')}</span></div>
    {#each byCategory as c (c.id)}
      <div class="cat-row" style:--c={c.color}>
        <span class="cat-icon"><c.icon size={14} /></span>
        <span class="cat-name truncate">{c.name}</span>
        <span class="cat-bar"><span style:width="{(c.value / catMax) * 100}%"></span></span>
        <span class="cat-val tabular">{fmtMoney(c.value)}</span>
        <span class="cat-pct faint small tabular">{pct(c.value / spentTotal)}</span>
      </div>
    {:else}
      <p class="muted small">{t('yr.noData')}</p>
    {/each}
  </div>

  <div class="card">
    <div class="card-head">
      <span class="review-icon"><NotebookPen size={15} /></span>
      <span class="strong">{t('yr.reviews')}</span>
      <span class="faint small">{t('yr.reviewsCount', { n: written.length })}</span>
    </div>
    {#if written.length}
      <div class="reviews">
        {#each written as m (m)}
          {@const x = perMonth[Number(m.slice(5)) - 1]}
          <button class="review" onclick={() => onmonth(`${m}-01`)}>
            <div class="review-top">
              <span class="strong">{monthName(m)}</span>
              <span class="faint small tabular">{t('yr.reviewMeta', { spent: fmtMoney(x.spent), days: x.noSpend })}</span>
            </div>
            <p class="review-text">{reviews[m]}</p>
          </button>
        {/each}
      </div>
    {:else}
      <p class="muted small">{t('yr.noReviews')}</p>
    {/if}
  </div>
</div>

<style>
  .year {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(6, minmax(0, 1fr));
    gap: 12px;
  }
  .stat {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 14px 16px;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--border);
    min-width: 0;
  }
  .stat b {
    font-size: 18px;
    font-weight: 750;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .stat b.neg {
    color: var(--danger);
  }
  .card {
    padding: 16px 18px;
  }
  .card-head {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 12px;
  }
  .card-head .strong {
    font-weight: 700;
  }
  .spacer {
    flex: 1;
  }
  .legend {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--muted);
  }
  .sw {
    width: 9px;
    height: 9px;
    border-radius: 3px;
    display: inline-block;
  }
  .sw.earned {
    margin-left: 8px;
  }
  .sw.spent,
  .bar.spent {
    background: var(--danger);
  }
  .sw.earned,
  .bar.earned {
    background: color-mix(in srgb, var(--muted) 45%, transparent);
  }
  .months {
    display: grid;
    grid-template-columns: repeat(12, minmax(0, 1fr));
    gap: 6px;
    height: 200px;
  }
  .mcol {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    min-width: 0;
    padding: 4px 0;
    border: none;
    border-radius: 10px;
    background: none;
    font: inherit;
    color: var(--text);
    cursor: pointer;
  }
  .mcol:hover {
    background: var(--surface-2);
  }
  .mcol.future {
    opacity: 0.35;
  }
  .val {
    font-size: 11px;
    font-weight: 700;
    color: var(--muted);
    height: 14px;
  }
  .bars {
    flex: 1;
    width: 100%;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    gap: 3px;
  }
  .bar {
    width: 30%;
    max-width: 16px;
    min-height: 2px;
    border-radius: 4px 4px 2px 2px;
  }
  .mlabel {
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
  }
  .dot {
    position: absolute;
    bottom: 1px;
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--primary);
  }
  .cat-row {
    display: grid;
    grid-template-columns: 26px minmax(0, 130px) 1fr 110px 40px;
    align-items: center;
    gap: 10px;
    padding: 5px 0;
  }
  .cat-icon {
    width: 26px;
    height: 26px;
    border-radius: 8px;
    display: grid;
    place-items: center;
    color: var(--c);
    background: color-mix(in srgb, var(--c) 14%, transparent);
  }
  .cat-name {
    font-weight: 600;
    font-size: 13.5px;
  }
  .cat-bar {
    height: 8px;
    border-radius: 999px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .cat-bar span {
    display: block;
    height: 100%;
    border-radius: 999px;
    background: var(--c);
  }
  .cat-val {
    text-align: right;
    font-weight: 700;
    font-size: 13.5px;
  }
  .cat-pct {
    text-align: right;
  }
  .review-icon {
    width: 28px;
    height: 28px;
    border-radius: 8px;
    display: grid;
    place-items: center;
    color: var(--primary);
    background: color-mix(in srgb, var(--primary) 12%, transparent);
  }
  .reviews {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    align-items: start;
    gap: 10px;
  }
  .review {
    text-align: left;
    padding: 12px 14px;
    border-radius: 12px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    font: inherit;
    color: var(--text);
    cursor: pointer;
  }
  .review:hover {
    border-color: color-mix(in srgb, var(--primary) 40%, transparent);
  }
  .review-top {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 6px;
  }
  .review-top .strong {
    font-weight: 700;
  }
  .review-text {
    margin: 0;
    white-space: pre-wrap;
    line-height: 1.6;
    font-size: 13.5px;
  }
</style>
