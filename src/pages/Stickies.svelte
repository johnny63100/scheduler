<script lang="ts">
  // Stickies: short one-line notes pinned on a board. Click a note to edit it, drag to reorder.
  // The phone lists the same notes (see src-tauri/src/cloud.rs).
  import { Check, GripVertical, Plus, StickyNote, X } from '@lucide/svelte';
  import { api, type Sticky } from '../lib/api';
  import { PALETTE, hex, textOn } from '../lib/colors';
  import { data, load, mutate, toast } from '../lib/state.svelte';
  import { t } from '../lib/i18n.svelte';

  /** Classic sticky-note yellow from the palette, used for new notes until another is picked. */
  const DEFAULT_COLOR = PALETTE[3].value;

  let notes = $state<Sticky[]>([]);
  let draft = $state('');
  let color = $state(DEFAULT_COLOR);
  let editingId = $state<number | null>(null);
  let editText = $state('');

  $effect(() => {
    data.version;
    load(api.stickies(), []).then((list) => (notes = list));
  });

  async function add() {
    const text = draft.trim();
    if (!text) return;
    const id = await mutate(api.addSticky(text, color));
    if (id !== undefined) draft = '';
  }

  function startEdit(n: Sticky) {
    editingId = n.id;
    editText = n.text;
  }

  async function saveEdit(n: Sticky) {
    if (editingId !== n.id) return;
    const text = editText.trim();
    editingId = null;
    if (!text || text === n.text) return;
    await mutate(api.updateSticky(n.id, text, n.color));
  }

  function editKey(e: KeyboardEvent, n: Sticky) {
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      saveEdit(n);
    } else if (e.key === 'Escape') {
      editingId = null;
    }
  }

  const recolor = (n: Sticky, value: number) => mutate(api.updateSticky(n.id, n.text, value));
  const remove = (n: Sticky) => mutate(api.deleteSticky(n.id), t('st.deleted'));

  /** Focuses the edit box when it appears and puts the caret at the end. */
  function focusEnd(el: HTMLTextAreaElement) {
    el.focus();
    el.setSelectionRange(el.value.length, el.value.length);
  }

  // ---- drag a note to reorder; notes move live while dragging
  let dragOrder = $state<Sticky[] | null>(null);
  let dragId = $state<number | null>(null);
  let dragChanged = false;
  const shown = $derived(dragOrder ?? notes);

  function dragStart(e: DragEvent, n: Sticky) {
    if (editingId !== null) return e.preventDefault();
    e.dataTransfer?.setData('application/x-nora-sticky', String(n.id));
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
    dragId = n.id;
    dragOrder = [...notes];
    dragChanged = false;
  }

  function dragOver(e: DragEvent, target: Sticky) {
    if (dragId === null || !dragOrder) return;
    e.preventDefault();
    if (target.id === dragId) return;
    const from = dragOrder.findIndex((x) => x.id === dragId);
    const to = dragOrder.findIndex((x) => x.id === target.id);
    if (from < 0 || to < 0) return;
    const next = [...dragOrder];
    next.splice(to, 0, ...next.splice(from, 1));
    dragOrder = next;
    dragChanged = true;
  }

  function dragEnd() {
    if (dragOrder && dragChanged) {
      notes = dragOrder;
      api.reorderStickies(dragOrder.map((x) => x.id)).catch((err) => toast(String(err), 'error'));
    }
    dragId = null;
    dragOrder = null;
  }
</script>

<div class="page">
  <div class="page-header">
    <div>
      <h1>{t('nav.stickies')}</h1>
      <p class="sub">{t('st.subtitle')}</p>
    </div>
  </div>

  <div class="card composer">
    <input
      class="input"
      placeholder={t('st.placeholder')}
      maxlength="500"
      bind:value={draft}
      onkeydown={(e) => e.key === 'Enter' && !e.isComposing && add()}
    />
    <div class="dots" role="group" aria-label={t('st.color')}>
      {#each PALETTE as p (p.value)}
        <button
          type="button"
          class="dot"
          class:on={color === p.value}
          style:background={hex(p.value)}
          title={p.name}
          aria-label={p.name}
          onclick={() => (color = p.value)}
        ></button>
      {/each}
    </div>
    <button class="btn primary" onclick={add} disabled={!draft.trim()}><Plus size={17} /> {t('st.add')}</button>
  </div>

  {#if notes.length === 0}
    <div class="card empty">
      <span class="empty-icon"><StickyNote size={30} /></span>
      <h2>{t('st.emptyTitle')}</h2>
      <p>{t('st.emptyBody')}</p>
    </div>
  {:else}
    {#if notes.length > 1}<p class="hint faint small"><GripVertical size={14} /> {t('st.hint')}</p>{/if}
    <div class="board">
      {#each shown as n (n.id)}
        <div
          class="note"
          class:dragging={dragId === n.id}
          class:editing={editingId === n.id}
          role="listitem"
          draggable={editingId === n.id ? undefined : 'true'}
          ondragstart={(e) => dragStart(e, n)}
          ondragover={(e) => dragOver(e, n)}
          ondrop={(e) => e.preventDefault()}
          ondragend={dragEnd}
          style:background={hex(n.color)}
          style:color={textOn(n.color)}
        >
          {#if editingId === n.id}
            <textarea
              class="edit"
              maxlength="500"
              bind:value={editText}
              use:focusEnd
              onkeydown={(e) => editKey(e, n)}
              onblur={() => saveEdit(n)}
            ></textarea>
            <button class="corner save" title={t('common.save')} aria-label={t('common.save')} onmousedown={(e) => e.preventDefault()} onclick={() => saveEdit(n)}>
              <Check size={15} />
            </button>
          {:else}
            <button class="text" title={t('st.edit')} onclick={() => startEdit(n)}>{n.text}</button>
            <button class="corner del" title={t('st.delete')} aria-label={t('st.delete')} onclick={() => remove(n)}><X size={15} /></button>
            <div class="recolor" role="group" aria-label={t('st.color')}>
              {#each PALETTE as p (p.value)}
                <button
                  type="button"
                  class="mini"
                  class:on={n.color === p.value}
                  style:background={hex(p.value)}
                  aria-label={p.name}
                  title={p.name}
                  onclick={() => recolor(n, p.value)}
                ></button>
              {/each}
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .composer {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 14px;
    margin-bottom: 18px;
    flex-wrap: wrap;
  }
  .composer .input {
    flex: 1 1 260px;
    min-width: 0;
  }
  .dots {
    display: flex;
    gap: 6px;
  }
  .dot {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    border: 2px solid transparent;
    padding: 0;
    cursor: pointer;
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.06);
  }
  .dot.on {
    border-color: var(--surface);
    box-shadow: 0 0 0 2px var(--text);
  }
  .hint {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0 0 10px;
  }
  .board {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
    gap: 14px;
  }
  .note {
    position: relative;
    min-height: 120px;
    border-radius: 6px 6px 6px 18px;
    padding: 14px 14px 30px;
    box-shadow: 0 1px 2px rgba(20, 22, 35, 0.08), 0 6px 16px rgba(20, 22, 35, 0.06);
    cursor: grab;
    transition: transform 0.12s ease, box-shadow 0.12s ease;
  }
  .note:hover {
    transform: translateY(-2px);
    box-shadow: 0 2px 4px rgba(20, 22, 35, 0.1), 0 10px 22px rgba(20, 22, 35, 0.1);
  }
  .note.dragging {
    opacity: 0.45;
  }
  .note.editing {
    cursor: default;
    transform: none;
  }
  .text {
    all: unset;
    display: block;
    width: 100%;
    font-size: 15px;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
    cursor: text;
  }
  .edit {
    width: 100%;
    min-height: 76px;
    resize: none;
    border: 0;
    outline: none;
    background: rgba(255, 255, 255, 0.45);
    border-radius: 6px;
    padding: 6px 8px;
    font: inherit;
    font-size: 15px;
    line-height: 1.5;
    color: inherit;
  }
  .corner {
    position: absolute;
    top: 6px;
    right: 6px;
    width: 26px;
    height: 26px;
    border-radius: 50%;
    border: 0;
    display: grid;
    place-items: center;
    background: rgba(255, 255, 255, 0.55);
    color: inherit;
    cursor: pointer;
  }
  .corner.del,
  .recolor {
    opacity: 0;
    transition: opacity 0.12s ease;
  }
  .note:hover .corner.del,
  .note:hover .recolor,
  .note:focus-within .corner.del,
  .note:focus-within .recolor {
    opacity: 1;
  }
  .corner.save {
    top: auto;
    bottom: 6px;
  }
  .recolor {
    position: absolute;
    left: 12px;
    right: 12px;
    bottom: 8px;
    display: flex;
    gap: 4px;
  }
  .mini {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: 0;
    padding: 0;
    cursor: pointer;
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.12);
  }
  .mini.on {
    box-shadow: 0 0 0 2px rgba(30, 34, 50, 0.55);
  }
</style>
