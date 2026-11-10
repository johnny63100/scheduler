<script lang="ts">
  // Fixed expenses on the Budget tab: payments due every month with a "paid" tick.
  // Ticking records the expense on its due date; unticking removes that record (src-tauri/src/fixed.rs).
  import { Check, Pencil, Plus, Repeat2, Trash, X } from '@lucide/svelte';
  import { api, type Expense } from '../lib/api';
  import { monthStart, parseYmd, today } from '../lib/dates';
  import {
    categoriesFor, categoryOf, expenseSettings, fmtMoney, loadFixed, newFixedId, saveFixed, type FixedExpense,
  } from '../lib/expenses.svelte';
  import { t } from '../lib/i18n.svelte';
  import { data, toast } from '../lib/state.svelte';

  let { month, spending, left, hasBudgets }: { month: string; spending: Expense[]; left: number; hasBudgets: boolean } = $props();

  const key = $derived(month.slice(0, 7));
  const daysInMonth = $derived(new Date(parseYmd(month).getFullYear(), parseYmd(month).getMonth() + 1, 0).getDate());
  const isCurrentMonth = $derived(month === monthStart(today()));
  const todayDay = $derived(parseYmd(today()).getDate());

  // Paid marks change when ticking from the phone too: re-read them whenever data changes.
  $effect(() => {
    data.version;
    loadFixed();
  });

  /** Paid if marked for this month and the recorded expense still exists. */
  function paidId(f: FixedExpense): number | null {
    const id = expenseSettings.fixedPaid[key]?.[f.id];
    return id && spending.some((e) => e.id === id) ? id : null;
  }
  const dueDay = (f: FixedExpense) => Math.min(Math.max(1, f.day), daysInMonth);

  const items = $derived([...expenseSettings.fixed].sort((a, b) => dueDay(a) - dueDay(b)));
  const total = $derived(items.reduce((s, f) => s + f.amount, 0));
  const paidTotal = $derived(items.filter((f) => paidId(f)).reduce((s, f) => s + f.amount, 0));
  const unpaid = $derived(items.filter((f) => !paidId(f)));
  const unpaidTotal = $derived(total - paidTotal);
  /** Unpaid fixed costs in categories that have a budget target (they will eat into the budget). */
  const unpaidInBudget = $derived(unpaid.filter((f) => expenseSettings.categoryBudgets[f.category]).reduce((s, f) => s + f.amount, 0));
  const free = $derived(left - unpaidInBudget);

  let busy = $state<string | null>(null);
  async function toggle(f: FixedExpense) {
    if (busy) return;
    busy = f.id;
    try {
      if (paidId(f)) await api.unpayFixed(f.id, key);
      else await api.payFixed(f.id, key);
      await loadFixed();
      data.version++;
    } catch (e) {
      toast(String(e), 'error');
    } finally {
      busy = null;
    }
  }

  // ---- editing the list
  let editing = $state(false);
  let draft = $state<{ id: string; name: string; amount: string; category: string; day: string }[]>([]);
  function startEdit() {
    draft = expenseSettings.fixed.map((f) => ({ id: f.id, name: f.name, amount: String(f.amount), category: f.category, day: String(f.day) }));
    if (!draft.length) addRow();
    editing = true;
  }
  function addRow() {
    draft.push({ id: newFixedId(), name: '', amount: '', category: 'bills', day: String(todayDay) });
  }
  async function saveList() {
    const list: FixedExpense[] = [];
    for (const d of draft) {
      const name = d.name.trim();
      const amount = Number(d.amount);
      if (!name && !d.amount) continue;
      if (!name || !(amount > 0)) return toast(t('fx.invalid'), 'error');
      list.push({ id: d.id, name, amount, category: d.category, day: Math.min(31, Math.max(1, Math.round(Number(d.day) || 1))) });
    }
    await saveFixed(list);
    editing = false;
    toast(t('fx.saved'), 'success');
  }
</script>

<div class="card fixed">
  <div class="head">
    <span class="title-icon"><Repeat2 size={17} /></span>
    <h3>{t('fx.title')}</h3>
    {#if !editing && items.length}
      <span class="muted small tabular">{t('fx.summary', { paid: fmtMoney(paidTotal), total: fmtMoney(total) })}</span>
    {/if}
    <span class="spacer"></span>
    {#if editing}
      <button class="btn small" onclick={() => (editing = false)}><X size={15} /> {t('common.cancel')}</button>
      <button class="btn small primary" onclick={saveList}><Check size={15} /> {t('common.save')}</button>
    {:else}
      <button class="btn small" onclick={startEdit}>
        {#if items.length}<Pencil size={14} /> {t('fx.edit')}{:else}<Plus size={15} /> {t('fx.add')}{/if}
      </button>
    {/if}
  </div>

  {#if editing}
    <div class="edit">
      <div class="edit-row labels faint small">
        <span>{t('fx.name')}</span><span>{t('fx.amount')}</span><span>{t('fx.category')}</span><span>{t('fx.day')}</span><span></span>
      </div>
      {#each draft as d, i (d.id)}
        <div class="edit-row">
          <input class="input" placeholder={t('fx.namePlaceholder')} maxlength="40" bind:value={d.name} />
          <input class="input num" type="number" inputmode="numeric" min="0" step="any" placeholder="0" bind:value={d.amount} />
          <select class="input" bind:value={d.category}>
            {#each categoriesFor('expense') as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
          </select>
          <div class="day-in">
            <input class="input num" type="number" inputmode="numeric" min="1" max="31" bind:value={d.day} />
            <span class="faint small">{t('fx.daySuffix')}</span>
          </div>
          <button class="icon-btn danger" onclick={() => draft.splice(i, 1)} aria-label={t('menu.delete')}><Trash size={15} /></button>
        </div>
      {/each}
      <button class="btn small add-row" onclick={addRow}><Plus size={15} /> {t('fx.addRow')}</button>
      <p class="faint small">{t('fx.editHint')}</p>
    </div>
  {:else if !items.length}
    <p class="muted small empty">{t('fx.empty')}</p>
  {:else}
    <div class="list">
      {#each items as f (f.id)}
        {@const c = categoryOf(f.category)}
        {@const isPaid = !!paidId(f)}
        {@const late = !isPaid && ((isCurrentMonth && dueDay(f) < todayDay) || month < monthStart(today()))}
        {@const soon = !isPaid && isCurrentMonth && !late && dueDay(f) - todayDay <= 3}
        <div class="item" class:paid={isPaid}>
          <button
            class="check"
            class:on={isPaid}
            disabled={busy === f.id}
            onclick={() => toggle(f)}
            title={isPaid ? t('fx.unpay') : t('fx.pay')}
            aria-label={isPaid ? t('fx.unpay') : t('fx.pay')}
            aria-pressed={isPaid}
          >
            <Check size={14} />
          </button>
          <span class="cat" style:--c={c.color}><c.icon size={14} /></span>
          <span class="name truncate">{f.name}</span>
          <span class="when small" class:late class:soon>
            {isPaid ? t('fx.paidOn', { day: dueDay(f) }) : late ? t('fx.late', { day: dueDay(f) }) : t('fx.due', { day: dueDay(f) })}
          </span>
          <span class="amt tabular">{fmtMoney(f.amount)}</span>
        </div>
      {/each}
    </div>
    <div class="foot">
      <span>{t('fx.left')} <b class="tabular">{fmtMoney(unpaidTotal)}</b> <span class="faint small">({unpaid.length})</span></span>
      {#if hasBudgets}
        <span class="sep"></span>
        <span>{t('fx.free')} <b class="tabular" class:neg={free < 0}>{free < 0 ? '−' : ''}{fmtMoney(Math.abs(free))}</b></span>
      {/if}
    </div>
  {/if}
</div>

<style>
  .fixed {
    margin-bottom: 16px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 12px;
    flex-wrap: wrap;
  }
  .head h3 {
    margin: 0;
  }
  .title-icon {
    width: 30px;
    height: 30px;
    border-radius: 10px;
    display: grid;
    place-items: center;
    color: var(--primary);
    background: color-mix(in srgb, var(--primary) 12%, transparent);
  }
  .spacer {
    flex: 1;
  }
  .empty {
    margin: 0;
  }
  .list {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(330px, 1fr));
    gap: 6px 18px;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 4px;
    border-bottom: 1px solid var(--border);
  }
  .check {
    width: 24px;
    height: 24px;
    flex: none;
    border-radius: 8px;
    border: 2px solid var(--faint);
    background: transparent;
    display: grid;
    place-items: center;
    padding: 0;
    color: transparent;
    cursor: pointer;
  }
  .check:hover {
    border-color: var(--success);
    color: color-mix(in srgb, var(--success) 50%, transparent);
  }
  .check.on {
    background: var(--success);
    border-color: var(--success);
    color: #fff;
  }
  .cat {
    width: 26px;
    height: 26px;
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
  .item.paid .name,
  .item.paid .amt {
    color: var(--muted);
    text-decoration: line-through;
    text-decoration-color: color-mix(in srgb, var(--muted) 60%, transparent);
  }
  .when {
    color: var(--muted);
    white-space: nowrap;
  }
  .when.soon {
    color: var(--warning);
    font-weight: 600;
  }
  .when.late {
    color: var(--danger);
    font-weight: 600;
  }
  .amt {
    font-weight: 700;
    white-space: nowrap;
  }
  .foot {
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
    margin-top: 12px;
    font-size: 13.5px;
    color: var(--muted);
  }
  .foot b {
    color: var(--text);
    margin-left: 4px;
  }
  .foot b.neg {
    color: var(--danger);
  }
  .sep {
    width: 1px;
    height: 14px;
    background: var(--border);
  }
  .edit {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .edit-row {
    display: grid;
    grid-template-columns: minmax(0, 1.6fr) 120px minmax(0, 1fr) 84px 32px;
    gap: 8px;
    align-items: center;
  }
  .edit-row.labels {
    padding: 0 2px;
  }
  .num {
    text-align: right;
  }
  .day-in {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .day-in .input {
    width: 56px;
  }
  .danger {
    color: var(--danger);
  }
  .add-row {
    align-self: flex-start;
    margin-top: 4px;
  }
  .edit p {
    margin: 4px 0 0;
  }
</style>
