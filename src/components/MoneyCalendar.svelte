<script lang="ts">
  // Money book calendar: how much was spent on each day of the month, with weekly totals,
  // no-spend days and the day's records under the grid. Categories left out of stats don't count.
  import { api, type Expense } from '../lib/api';
  import { NotebookPen } from '@lucide/svelte';
  import { addDays, fmt, monthStart, parseYmd, range, today, weekStart, weekdayNames } from '../lib/dates';
  import { categoryOf, countsInStats, fmtMoney } from '../lib/expenses.svelte';
  import { holidayName } from '../lib/holidays';
  import { intlLocale, t } from '../lib/i18n.svelte';

  let { month, expenses, onedit }: { month: string; expenses: Expense[]; onedit: (e: Expense) => void } = $props();

  const todayStr = $derived(today());
  const monthKey = $derived(month.slice(0, 7));
  const daysInMonth = $derived(new Date(parseYmd(month).getFullYear(), parseYmd(month).getMonth() + 1, 0).getDate());
  const lastDay = $derived(`${monthKey}-${String(daysInMonth).padStart(2, '0')}`);
  /** Whole weeks covering the month (Sunday first, like the calendar page). */
  const weeks = $derived.by(() => {
    const start = weekStart(month);
    const count = Math.ceil((Number(new Date(parseYmd(lastDay)).getTime() - parseYmd(start).getTime()) / 86_400_000 + 1) / 7);
    return Array.from({ length: count }, (_, w) => range(addDays(start, w * 7), 7));
  });

  const spentOn = $derived.by(() => {
    const map = new Map<string, number>();
    for (const e of expenses) if (e.kind !== 'income' && countsInStats(e.category)) map.set(e.date, (map.get(e.date) ?? 0) + e.amount);
    return map;
  });
  const earnedOn = $derived.by(() => {
    const map = new Map<string, number>();
    for (const e of expenses) if (e.kind === 'income' && countsInStats(e.category)) map.set(e.date, (map.get(e.date) ?? 0) + e.amount);
    return map;
  });
  const maxDay = $derived(Math.max(1, ...spentOn.values()));

  // ---- the month at a glance
  const isCurrent = $derived(month === monthStart(todayStr));
  const isFuture = $derived(month > monthStart(todayStr));
  /** Days of this month that have already started (all of them for past months). */
  const daysSoFar = $derived(isFuture ? 0 : isCurrent ? parseYmd(todayStr).getDate() : daysInMonth);
  const total = $derived([...spentOn].filter(([d]) => d.startsWith(monthKey)).reduce((s, [, v]) => s + v, 0));
  const perDay = $derived(daysSoFar ? total / daysSoFar : 0);
  const noSpend = $derived(range(month, daysSoFar).filter((d) => !spentOn.get(d)).length);
  const topDay = $derived([...spentOn].filter(([d]) => d.startsWith(monthKey)).sort((a, b) => b[1] - a[1])[0]);

  /** Amounts in the cells: digits only, so they fit (the currency is in the summary above). */
  const num = (n: number) => new Intl.NumberFormat(intlLocale(), { maximumFractionDigits: 0 }).format(n);

  let selected = $state<string | null>(null);
  $effect(() => {
    monthKey;
    selected = isCurrent ? todayStr : null;
  });
  const dayList = $derived(selected ? expenses.filter((e) => e.date === selected) : []);
  const names = $derived(weekdayNames('short'));

  // ---- monthly review: free text per month, saved as you type (setting "expense.reviews": { "YYYY-MM": text })
  let reviews: Record<string, string> = {};
  let review = $state('');
  let reviewsLoaded = $state(false);
  let saved = $state(false);
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    api
      .getSetting('expense.reviews')
      .then((raw) => {
        try {
          reviews = raw ? JSON.parse(raw) : {};
        } catch {
          reviews = {};
        }
      })
      .catch(() => {})
      .finally(() => (reviewsLoaded = true));
  });
  $effect(() => {
    if (!reviewsLoaded) return;
    review = reviews[monthKey] ?? '';
    saved = false;
  });
  function onReview() {
    saved = false;
    const key = monthKey;
    const text = review;
    clearTimeout(saveTimer);
    saveTimer = setTimeout(async () => {
      if (text.trim()) reviews[key] = text;
      else delete reviews[key];
      await api.setSetting('expense.reviews', JSON.stringify(reviews)).catch(() => {});
      if (key === monthKey) saved = true;
    }, 600);
  }
  const monthLabel = $derived(fmt(month, { month: 'long' }));
  /** The current month isn't over yet: the box still works, with a gentler hint. */
  const reviewHint = $derived(isFuture ? t('mc.reviewFuture') : isCurrent ? t('mc.reviewNow') : t('mc.reviewPast'));
</script>

<div class="money-cal">
  <div class="stats">
    <div class="stat"><span class="faint small">{t('mc.total')}</span><b class="tabular">{fmtMoney(total)}</b></div>
    <div class="stat"><span class="faint small">{t('mc.perDay')}</span><b class="tabular">{fmtMoney(Math.round(perDay))}</b></div>
    <div class="stat"><span class="faint small">{t('mc.noSpend')}</span><b class="tabular">{t('mc.days', { n: noSpend })} <span class="faint small">/ {daysSoFar}</span></b></div>
    <div class="stat">
      <span class="faint small">{t('mc.topDay')}</span>
      <b class="tabular">{#if topDay}{fmtMoney(topDay[1])} <span class="faint small">{fmt(topDay[0], { month: 'numeric', day: 'numeric' })}</span>{:else}—{/if}</b>
    </div>
  </div>

  <div class="card grid-card">
    <div class="grid">
      {#each names as n, i (i)}<div class="dow" class:sun={i === 0}>{n}</div>{/each}
      <div class="dow week-h">{t('mc.week')}</div>
      {#each weeks as week, w (w)}
        {#each week as day (day)}
          {@const inMonth = day.startsWith(monthKey)}
          {@const spent = inMonth ? (spentOn.get(day) ?? 0) : 0}
          {@const earned = inMonth ? (earnedOn.get(day) ?? 0) : 0}
          {@const past = day <= todayStr}
          <button
            class="cell"
            class:out={!inMonth}
            class:today={day === todayStr}
            class:sel={selected === day}
            class:hot={spent / maxDay > 0.55}
            class:inked={spent > 0}
            style:--heat={spent ? 0.08 + 0.72 * (spent / maxDay) : 0}
            disabled={!inMonth}
            onclick={() => (selected = selected === day ? null : day)}
          >
            <span class="d" class:red={!!holidayName(day) || parseYmd(day).getDay() === 0}>{Number(day.slice(8))}</span>
            {#if inMonth}
              {#if spent}<span class="spent tabular">{num(spent)}</span>
              {:else if past}<span class="stamp" class:aside={!!earned} title={t('mc.zero')} aria-label={t('mc.zero')}><svg viewBox="0 0 24 24" width="36" height="36" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round"><circle cx="8.5" cy="9.5" r="1.2" fill="currentColor" stroke="none" /><circle cx="15.5" cy="9.5" r="1.2" fill="currentColor" stroke="none" /><path d="M7 14.2c1.2 1.9 2.9 2.8 5 2.8s3.8-.9 5-2.8" /></svg></span>{/if}
              {#if earned}<span class="earned tabular">+{num(earned)}</span>{/if}
            {/if}
          </button>
        {/each}
        {@const wk = week.filter((d) => d.startsWith(monthKey)).reduce((s, d) => s + (spentOn.get(d) ?? 0), 0)}
        <div class="week-total tabular">{wk ? num(wk) : ''}</div>
      {/each}
    </div>
  </div>

  {#if selected}
    <div class="card day-card">
      <div class="day-head">
        <span class="strong">{fmt(selected, { month: 'long', day: 'numeric', weekday: 'short' })}</span>
        <span class="muted small tabular">{t('mc.daySpent', { amount: fmtMoney(spentOn.get(selected) ?? 0) })}</span>
      </div>
      {#each dayList as e (e.id)}
        {@const c = categoryOf(e.category)}
        <button class="row" class:aside={c.excluded} style:--c={c.color} onclick={() => onedit(e)}>
          <span class="icon"><c.icon size={15} /></span>
          <span class="name truncate">{e.note || c.name}</span>
          <span class="faint small">{e.note ? c.name : ''}</span>
          <span class="amt tabular" class:in={e.kind === 'income'}>{e.kind === 'income' ? '+' : '−'}{fmtMoney(e.amount)}</span>
        </button>
      {:else}
        <p class="muted small empty">{t('mc.none')}</p>
      {/each}
    </div>
  {/if}

  <div class="card review-card">
    <div class="review-head">
      <span class="review-icon"><NotebookPen size={15} /></span>
      <span class="strong">{t('mc.review', { month: monthLabel })}</span>
      <span class="spacer"></span>
      {#if saved}<span class="faint small">{t('mc.saved')}</span>{/if}
    </div>
    <textarea class="input review" rows="5" bind:value={review} oninput={onReview} placeholder={reviewHint} disabled={!reviewsLoaded || isFuture}></textarea>
  </div>
</div>

<style>
  .review-card {
    padding: 14px 16px 16px;
  }
  .review-head {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 10px;
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
  .spacer {
    flex: 1;
  }
  .review-head .strong {
    font-weight: 700;
  }
  .review {
    width: 100%;
    padding: 12px 14px;
    min-height: 110px;
    resize: vertical;
    line-height: 1.6;
    font: inherit;
    font-size: 14px;
  }
  .money-cal {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
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
  }
  .stat b {
    font-size: 19px;
    font-weight: 750;
  }
  .grid-card {
    padding: 14px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr)) 92px;
    gap: 4px;
  }
  .dow {
    text-align: center;
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
    padding: 2px 0 6px;
  }
  .dow.sun {
    color: var(--danger);
  }
  .week-h {
    text-align: right;
    padding-right: 6px;
  }
  .cell {
    /* Darker = more spent that day. */
    --heat-ink: var(--danger);
    position: relative;
    min-height: 74px;
    padding: 6px 8px;
    border-radius: 10px;
    border: 1px solid transparent;
    background: color-mix(in srgb, var(--heat-ink) calc(var(--heat) * 100%), var(--surface-2));
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;
    cursor: pointer;
    font: inherit;
    color: var(--text);
    text-align: right;
  }
  /* Light text on the darkest days. */
  .cell.hot .spent,
  .cell.hot .d,
  .cell.inked .earned {
    color: #fff;
  }
  .cell:hover:not(:disabled) {
    border-color: color-mix(in srgb, var(--primary) 40%, transparent);
  }
  .cell.out {
    background: transparent;
    cursor: default;
  }
  .cell.out .d {
    opacity: 0.3;
  }
  .cell.today {
    border-color: var(--primary);
  }
  .cell.sel {
    box-shadow: 0 0 0 2px var(--primary);
  }
  .d {
    align-self: flex-start;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--muted);
  }
  .d.red {
    color: var(--danger);
  }
  .spent {
    margin-top: auto;
    font-size: 13.5px;
    font-weight: 700;
  }
  /* No-spend day: a smiley rubber stamp (moved left when the day also has income). */
  .stamp {
    --ink: #e5484d;
    position: absolute;
    left: 50%;
    top: 54%;
    width: 50px;
    height: 50px;
    margin: -25px 0 0 -25px;
    border-radius: 50%;
    border: 2.5px solid var(--ink);
    box-shadow: inset 0 0 0 2px var(--surface-2), inset 0 0 0 3.5px var(--ink);
    color: var(--ink);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    line-height: 1;
    font-size: 11px;
    font-weight: 900;
    letter-spacing: 0.04em;
    transform: rotate(-14deg);
    opacity: 0.82;
    pointer-events: none;
  }
  /* Income on a day with no spending sits at the bottom. */
  .stamp ~ .earned {
    margin-top: auto;
  }
  .stamp.aside {
    left: 34%;
    top: 50%;
  }
  /* Income: small; white on days with spending (red cells), grey on blank days. */
  .earned {
    font-size: 11.5px;
    font-weight: 600;
    color: var(--muted);
  }
  .week-total {
    align-self: center;
    text-align: right;
    padding-right: 6px;
    font-size: 13px;
    font-weight: 700;
    color: var(--muted);
  }
  .day-card {
    padding: 14px 16px;
  }
  .day-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border);
    margin-bottom: 4px;
  }
  .row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 2px;
    border: none;
    background: none;
    font: inherit;
    color: var(--text);
    cursor: pointer;
    text-align: left;
    border-radius: 8px;
  }
  .row:hover {
    background: var(--surface-2);
  }
  .row.aside {
    opacity: 0.5;
  }
  .icon {
    width: 28px;
    height: 28px;
    flex: none;
    border-radius: 8px;
    display: grid;
    place-items: center;
    color: var(--c);
    background: color-mix(in srgb, var(--c) 14%, transparent);
  }
  .name {
    flex: 1;
    min-width: 0;
    font-weight: 600;
  }
  .amt {
    font-weight: 700;
  }
  .amt.in {
    color: var(--success);
  }
  .empty {
    margin: 8px 2px;
  }
</style>
