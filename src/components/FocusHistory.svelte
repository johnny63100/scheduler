<script lang="ts">
  // Pomodoro focus history for a day, a week or a month: totals, a chart and what the time
  // went to. Sessions count toward the day (and hour) they started, like the Analytics page.
  import { ChevronLeft, ChevronRight, Flame } from '@lucide/svelte';
  import BarChart from './BarChart.svelte';
  import { api, type PomodoroSession } from '../lib/api';
  import {
    addDays, addMonths, dayStartTs, fmt, fmtDuration, fmtRange, fmtTs, monthStart, pad, range, today, tsToDate, weekStart,
    weekdayNames,
  } from '../lib/dates';
  import { t } from '../lib/i18n.svelte';
  import { data, load } from '../lib/state.svelte';

  type Scope = 'day' | 'week' | 'month';
  const SCOPES: { id: Scope; label: () => string }[] = [
    { id: 'day', label: () => t('pomo.historyDay') },
    { id: 'week', label: () => t('pomo.historyWeek') },
    { id: 'month', label: () => t('pomo.historyMonth') },
  ];

  let scope = $state<Scope>('week');
  let cursor = $state(today());
  let sessions = $state<PomodoroSession[]>([]);

  /** The days in the chosen period, first to last. */
  const days = $derived.by(() => {
    if (scope === 'day') return [cursor];
    if (scope === 'week') return range(weekStart(cursor), 7);
    const first = monthStart(cursor);
    return range(first, Number(addDays(addMonths(first, 1), -1).slice(8)));
  });

  $effect(() => {
    data.version;
    const from = dayStartTs(days[0]);
    const to = dayStartTs(addDays(days[days.length - 1], 1));
    const want = `${from}-${to}`;
    load(api.pomodoroSessions(from, to), []).then((s) => {
      // Ignore an answer for a period that is no longer shown.
      if (want === `${dayStartTs(days[0])}-${dayStartTs(addDays(days[days.length - 1], 1))}`) sessions = s;
    });
  });

  const inPeriod = $derived(sessions.filter((s) => days.includes(tsToDate(s.started_at))));
  const total = $derived(inPeriod.reduce((sum, s) => sum + s.focused_secs, 0));
  const finished = $derived(inPeriod.filter((s) => s.completed).length);
  const stopped = $derived(inPeriod.length - finished);

  const secsOn = (day: string) => inPeriod.filter((s) => tsToDate(s.started_at) === day).reduce((sum, s) => sum + s.focused_secs, 0);

  /** Days that have already started (the average ignores days still to come). */
  const elapsed = $derived(days.filter((d) => d <= today()).length);
  const average = $derived(elapsed ? total / elapsed : 0);
  const best = $derived.by(() => {
    let top: { day: string; secs: number } | null = null;
    for (const day of days) {
      const secs = secsOn(day);
      if (secs > 0 && (!top || secs > top.secs)) top = { day, secs };
    }
    return top;
  });

  /** Minutes per hour (day view) or per day (week and month views). */
  const chart = $derived.by(() => {
    if (scope === 'day') {
      const mins = Array.from({ length: 24 }, () => 0);
      for (const s of inPeriod) mins[new Date(s.started_at * 1000).getHours()] += s.focused_secs / 60;
      return mins.map((value, h) => ({ label: pad(h), value: Math.round(value) }));
    }
    const names = weekdayNames('narrow');
    return days.map((day) => ({
      label: scope === 'week' ? names[new Date(dayStartTs(day) * 1000).getDay()] : String(Number(day.slice(8))),
      value: Math.round(secsOn(day) / 60),
    }));
  });

  /** Focus time per label, largest first. */
  const byLabel = $derived.by(() => {
    const totals = new Map<string, number>();
    for (const s of inPeriod) {
      const label = s.label.trim() || t('pomo.focus');
      totals.set(label, (totals.get(label) ?? 0) + s.focused_secs);
    }
    return [...totals.entries()].sort((a, b) => b[1] - a[1]);
  });
  const topLabel = $derived(byLabel[0]?.[1] ?? 1);

  const title = $derived.by(() => {
    if (scope === 'day') return fmt(cursor, { year: 'numeric', month: 'long', day: 'numeric', weekday: 'short' });
    if (scope === 'week') return fmtRange(days[0], days[6], { month: 'short', day: 'numeric' });
    return fmt(cursor, { year: 'numeric', month: 'long' });
  });

  const isCurrent = $derived(days.includes(today()));

  function step(dir: number) {
    cursor = scope === 'day' ? addDays(cursor, dir) : scope === 'week' ? addDays(cursor, 7 * dir) : addMonths(cursor, dir);
  }

  /** A bar in the week or month chart opens that day. */
  function openBar(i: number) {
    if (scope === 'day') return;
    cursor = days[i];
    scope = 'day';
  }
</script>

<div class="card history">
  <div class="head">
    <h3>{t('pomo.history')}</h3>
    <span class="spacer"></span>
    <div class="segmented">
      {#each SCOPES as s (s.id)}
        <button class:active={scope === s.id} onclick={() => (scope = s.id)}>{s.label()}</button>
      {/each}
    </div>
  </div>

  <div class="nav">
    <button class="icon-btn" onclick={() => step(-1)} aria-label={t('common.previous')}><ChevronLeft size={18} /></button>
    <span class="period">{title}</span>
    <button class="icon-btn" onclick={() => step(1)} aria-label={t('common.next')}><ChevronRight size={18} /></button>
    {#if !isCurrent}<button class="btn small" onclick={() => (cursor = today())}>{t('common.today')}</button>{/if}
  </div>

  <div class="numbers">
    <div><div class="big">{fmtDuration(total)}</div><div class="muted small">{t('pomo.totalFocus')}</div></div>
    <div><div class="big">{finished}</div><div class="muted small">{t('pomo.finishedSessions')}</div></div>
    {#if scope === 'day'}
      <div><div class="big">{stopped}</div><div class="muted small">{t('pomo.stoppedCount')}</div></div>
    {:else}
      <div><div class="big">{fmtDuration(average)}</div><div class="muted small">{t('pomo.dailyAvg')}</div></div>
      <div>
        <div class="big">{best ? fmt(best.day, { month: 'short', day: 'numeric' }) : '—'}</div>
        <div class="muted small">{t('pomo.bestDay')}{#if best} · {fmtDuration(best.secs)}{/if}</div>
      </div>
    {/if}
  </div>

  {#if inPeriod.length}
    <BarChart
      data={chart}
      color="var(--focus)"
      height={180}
      format={(m) => fmtDuration(m * 60)}
      onselect={scope === 'day' ? undefined : openBar}
    />
    {#if scope !== 'day'}<p class="faint small hint">{t('pomo.chartHint')}</p>{/if}

    <div class="split">
      <div>
        <h4>{t('pomo.byLabel')}</h4>
        {#each byLabel as [label, secs] (label)}
          <div class="label-row">
            <span class="truncate">{label}</span>
            <span class="bar"><span style:width="{(secs / topLabel) * 100}%"></span></span>
            <span class="tabular muted">{fmtDuration(secs)}</span>
          </div>
        {/each}
      </div>
      {#if scope === 'day'}
        <div>
          <h4>{t('pomo.sessionList')}</h4>
          {#each inPeriod as s (s.id)}
            <div class="session" class:partial={!s.completed}>
              <Flame size={14} />
              <span class="tabular muted">{fmtTs(s.started_at)} – {fmtTs(s.ended_at)}</span>
              <span class="truncate">{s.label || t('pomo.focus')}</span>
              <span class="meta tabular">
                {fmtDuration(s.focused_secs)}{#if !s.completed}{` · ${t('pomo.stoppedEarly')}`}{/if}
              </span>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {:else}
    <p class="muted empty">{t('pomo.historyEmpty')}</p>
  {/if}
</div>

<style>
  .history {
    margin-top: 20px;
  }
  .head {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
  }
  .nav {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 12px 0 16px;
  }
  .period {
    min-width: 170px;
    text-align: center;
    font-weight: 700;
  }
  .numbers {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(130px, 1fr));
    gap: 12px;
    margin-bottom: 18px;
  }
  .big {
    font-size: 20px;
    font-weight: 750;
  }
  .hint {
    margin-top: 4px;
    text-align: center;
  }
  .split {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: 20px;
    margin-top: 18px;
  }
  h4 {
    margin: 0 0 8px;
    font-size: 13px;
    color: var(--muted);
  }
  .label-row {
    display: grid;
    grid-template-columns: minmax(80px, 1fr) 2fr auto;
    align-items: center;
    gap: 10px;
    padding: 5px 0;
    font-size: 13.5px;
  }
  .bar {
    height: 8px;
    border-radius: 8px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    border-radius: 8px;
    background: var(--focus);
  }
  .session {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 10px;
    margin-bottom: 6px;
    border-radius: 12px;
    background: var(--surface-2);
    color: var(--focus);
    font-size: 13px;
  }
  .session > span {
    color: var(--text);
  }
  .session .muted {
    color: var(--muted);
  }
  .session.partial {
    opacity: 0.7;
  }
  .meta {
    margin-left: auto;
    flex: none;
    font-size: 12px;
  }
  .session .meta {
    color: var(--faint);
  }
  .empty {
    padding: 24px 0 8px;
    text-align: center;
  }
</style>
