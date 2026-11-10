<script lang="ts">
  // Edit money book categories: rename, change icon or color, hide built-ins, add or delete your own.
  // Records keep their category id, so hiding or deleting never changes past entries
  // (entries in a deleted category show under "Other").
  import { ArrowDown, ArrowUp, Eye, EyeOff, Plus, RotateCcw, Trash } from '@lucide/svelte';
  import Modal from './Modal.svelte';
  import type { ExpenseKind } from '../lib/api';
  import {
    CATEGORY_COLORS, CATEGORY_ICONS, FALLBACK, categoryStore, newCategoryId, resolveCategory, saveCategories, type CategoryDef,
  } from '../lib/expenses.svelte';
  import { t } from '../lib/i18n.svelte';
  import { toast } from '../lib/state.svelte';

  let { onclose }: { onclose: () => void } = $props();

  let kind = $state<ExpenseKind>('expense');
  let draft = $state<CategoryDef[]>(categoryStore.list.map((c) => ({ ...c })));
  /** Category whose icon/color picker is open. */
  let picking = $state<string | null>(null);

  const shown = $derived(draft.filter((c) => c.kind === kind));
  const isFallback = (c: CategoryDef) => (Object.values(FALLBACK) as string[]).includes(c.id);

  function add() {
    const c: CategoryDef = { id: newCategoryId(), kind, name: '', icon: 'sparkles', color: CATEGORY_COLORS[draft.length % CATEGORY_COLORS.length], custom: true };
    // New ones go just before the fallback ("Other"), which stays last.
    const fallbackAt = draft.findIndex((x) => x.id === FALLBACK[kind]);
    draft.splice(fallbackAt < 0 ? draft.length : fallbackAt, 0, c);
    picking = c.id;
    queueMicrotask(() => document.getElementById(`cat-name-${c.id}`)?.focus());
  }

  function move(c: CategoryDef, step: -1 | 1) {
    const same = draft.filter((x) => x.kind === c.kind);
    const i = same.indexOf(c);
    const other = same[i + step];
    if (!other) return;
    const a = draft.indexOf(c);
    const b = draft.indexOf(other);
    [draft[a], draft[b]] = [draft[b], draft[a]];
  }

  function remove(c: CategoryDef) {
    draft = draft.filter((x) => x !== c);
    if (picking === c.id) picking = null;
  }

  function reset(c: CategoryDef) {
    c.name = undefined;
  }

  async function save() {
    for (const c of draft) {
      if (c.custom && !c.name?.trim()) {
        toast(t('xc.nameMissing'), 'error');
        kind = c.kind;
        picking = c.id;
        return;
      }
      if (c.name !== undefined) c.name = c.name.trim() || undefined;
    }
    await saveCategories(draft);
    toast(t('xc.saved'), 'success');
    onclose();
  }
</script>

<Modal title={t('xc.title')} {onclose} width={560}>
  <div class="segmented kind">
    <button class:active={kind === 'expense'} onclick={() => ((kind = 'expense'), (picking = null))}>{t('ex.kindExpense')}</button>
    <button class:active={kind === 'income'} onclick={() => ((kind = 'income'), (picking = null))}>{t('ex.kindIncome')}</button>
  </div>
  <p class="faint small hint">{t('xc.hint')}</p>

  <div class="list">
    {#each shown as c, i (c.id)}
      {@const r = resolveCategory(c)}
      <div class="item" class:hidden={c.hidden}>
        <div class="row">
          <button class="icon" style:--c={c.color} onclick={() => (picking = picking === c.id ? null : c.id)} title={t('xc.look')} aria-label={t('xc.look')}>
            <r.icon size={17} />
          </button>
          <input
            id="cat-name-{c.id}"
            class="input name"
            value={c.name ?? ''}
            placeholder={c.label ? t(c.label) : t('xc.namePlaceholder')}
            maxlength="20"
            oninput={(e) => (c.name = e.currentTarget.value)}
          />
          {#if c.label && c.name}
            <button class="icon-btn" onclick={() => reset(c)} title={t('xc.resetName')} aria-label={t('xc.resetName')}><RotateCcw size={15} /></button>
          {/if}
          <button class="icon-btn" onclick={() => move(c, -1)} disabled={i === 0} aria-label={t('xc.up')}><ArrowUp size={15} /></button>
          <button class="icon-btn" onclick={() => move(c, 1)} disabled={i === shown.length - 1} aria-label={t('xc.down')}><ArrowDown size={15} /></button>
          {#if c.custom}
            <button class="icon-btn danger" onclick={() => remove(c)} title={t('xc.delete')} aria-label={t('xc.delete')}><Trash size={15} /></button>
          {:else if !isFallback(c)}
            <button class="icon-btn" onclick={() => (c.hidden = !c.hidden)} title={c.hidden ? t('xc.show') : t('xc.hide')} aria-label={c.hidden ? t('xc.show') : t('xc.hide')}>
              {#if c.hidden}<EyeOff size={15} />{:else}<Eye size={15} />{/if}
            </button>
          {:else}
            <span class="icon-btn-space"></span>
          {/if}
        </div>
        {#if picking === c.id}
          <div class="picker">
            <div class="icons">
              {#each Object.entries(CATEGORY_ICONS) as [key, v] (key)}
                <button class="pick" class:on={c.icon === key} style:--c={c.color} onclick={() => (c.icon = key)} aria-label={key}><v.icon size={16} /></button>
              {/each}
            </div>
            <div class="colors">
              {#each CATEGORY_COLORS as col (col)}
                <button class="dot" class:on={c.color === col} style:background={col} onclick={() => (c.color = col)} aria-label={col}></button>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    {/each}
  </div>

  <button class="btn add" onclick={add}><Plus size={16} /> {t('xc.add')}</button>

  {#snippet footer()}
    <button class="btn" onclick={onclose}>{t('common.cancel')}</button>
    <button class="btn primary" onclick={save}>{t('common.save')}</button>
  {/snippet}
</Modal>

<style>
  .kind {
    margin-bottom: 8px;
  }
  .hint {
    margin: 0 0 12px;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-height: 52vh;
    overflow-y: auto;
    padding-right: 2px;
  }
  .item {
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 6px;
  }
  .item.hidden {
    opacity: 0.5;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .icon {
    width: 34px;
    height: 34px;
    flex: none;
    border: 0;
    border-radius: 10px;
    display: grid;
    place-items: center;
    color: var(--c);
    background: color-mix(in srgb, var(--c) 14%, transparent);
    cursor: pointer;
  }
  .name {
    flex: 1;
    min-width: 0;
    height: 34px;
  }
  .icon-btn-space {
    width: 30px;
    flex: none;
  }
  .danger {
    color: var(--danger);
  }
  .picker {
    padding: 10px 4px 4px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .icons {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(32px, 1fr));
    gap: 4px;
  }
  .pick {
    height: 32px;
    border: 1px solid transparent;
    border-radius: 9px;
    background: var(--surface-2);
    color: var(--muted);
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  .pick.on {
    color: var(--c);
    border-color: var(--c);
    background: color-mix(in srgb, var(--c) 12%, transparent);
  }
  .colors {
    display: flex;
    flex-wrap: wrap;
    gap: 7px;
  }
  .dot {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    border: 2px solid transparent;
    padding: 0;
    cursor: pointer;
  }
  .dot.on {
    border-color: var(--surface);
    box-shadow: 0 0 0 2px var(--text);
  }
  .add {
    margin-top: 10px;
    width: 100%;
    justify-content: center;
  }
</style>
