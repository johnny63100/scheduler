<script lang="ts">
  // Stickers placed on an open diary. Positions and sizes are fractions of the book, so they stay
  // put when the window is resized. Outside decorate mode they ignore the mouse.
  import { BringToFront, MoveDiagonal2, RotateCw, X } from '@lucide/svelte';
  import type { DiarySticker } from '../lib/api';
  import { imageUrl } from '../lib/images';
  import { t } from '../lib/i18n.svelte';

  let {
    stickers,
    editing,
    onchange,
  }: { stickers: DiarySticker[]; editing: boolean; onchange: (stickers: DiarySticker[]) => void } = $props();

  let layer: HTMLDivElement;
  let selected = $state<string | null>(null);
  type Gesture = { kind: 'move' | 'resize' | 'rotate'; id: string; dx: number; dy: number };
  let gesture: Gesture | null = null;

  const clamp = (n: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, n));

  function center(s: DiarySticker) {
    const r = layer.getBoundingClientRect();
    return { cx: r.left + s.x * r.width, cy: r.top + s.y * r.height, r };
  }

  function update(id: string, patch: Partial<DiarySticker>) {
    onchange(stickers.map((s) => (s.id === id ? { ...s, ...patch } : s)));
  }

  function start(e: PointerEvent, s: DiarySticker, kind: Gesture['kind']) {
    if (!editing || e.button !== 0) return;
    e.stopPropagation();
    e.preventDefault();
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    selected = s.id;
    const { cx, cy } = center(s);
    gesture = { kind, id: s.id, dx: e.clientX - cx, dy: e.clientY - cy };
  }

  function move(e: PointerEvent, s: DiarySticker) {
    if (!gesture || gesture.id !== s.id) return;
    const { cx, cy, r } = center(s);
    if (gesture.kind === 'move') {
      update(s.id, { x: clamp((e.clientX - gesture.dx - r.left) / r.width, 0, 1), y: clamp((e.clientY - gesture.dy - r.top) / r.height, 0, 1) });
    } else if (gesture.kind === 'resize') {
      // The corner handle sits on the diagonal; its distance from the center sets the size.
      update(s.id, { w: clamp((Math.hypot(e.clientX - cx, e.clientY - cy) * Math.SQRT2) / r.width, 0.03, 0.6) });
    } else {
      const deg = (Math.atan2(e.clientY - cy, e.clientX - cx) * 180) / Math.PI + 90;
      update(s.id, { rot: e.shiftKey ? Math.round(deg / 15) * 15 : Math.round(deg) });
    }
  }

  const remove = (id: string) => {
    onchange(stickers.filter((s) => s.id !== id));
    selected = null;
  };
  function toFront(id: string) {
    const s = stickers.find((x) => x.id === id);
    if (s) onchange([...stickers.filter((x) => x.id !== id), s]);
  }

  function onkeydown(e: KeyboardEvent) {
    if (!editing || !selected) return;
    const typing = e.target instanceof HTMLElement && (e.target.isContentEditable || e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement);
    if ((e.key === 'Delete' || e.key === 'Backspace') && !typing) remove(selected);
  }
</script>

<svelte:window
  {onkeydown}
  onpointerdown={(e) => {
    if (editing && !(e.target as Element).closest?.('.d-sticker')) selected = null;
  }}
/>

<div class="d-stickers" class:editing bind:this={layer} aria-hidden={!editing}>
  {#each stickers as s (s.id)}
    {@const isSelected = editing && selected === s.id}
    <div
      class="d-sticker"
      class:selected={isSelected}
      style:left="{s.x * 100}%"
      style:top="{s.y * 100}%"
      style:width="{s.w * 100}%"
      style:transform="translate(-50%, -50%) rotate({s.rot}deg)"
      onpointerdown={(e) => start(e, s, 'move')}
      onpointermove={(e) => move(e, s)}
      onpointerup={() => (gesture = null)}
      onpointercancel={() => (gesture = null)}
      role="presentation"
    >
      <!-- Re-created when decorating ends, so animated GIFs keep playing. -->
      {#key editing}
        <img src={imageUrl(s.image, 'diary')} alt="" draggable="false" />
      {/key}
      {#if isSelected}
        <button class="handle del" onpointerdown={(e) => e.stopPropagation()} onclick={() => remove(s.id)} title={t('sticker.remove')}><X size={13} /></button>
        <button class="handle front" onpointerdown={(e) => e.stopPropagation()} onclick={() => toFront(s.id)} title={t('sticker.front')}><BringToFront size={13} /></button>
        <span class="handle rot" onpointerdown={(e) => start(e, s, 'rotate')} onpointermove={(e) => move(e, s)} onpointerup={() => (gesture = null)} title={t('sticker.rotate')} role="presentation"><RotateCw size={13} /></span>
        <span class="handle size" onpointerdown={(e) => start(e, s, 'resize')} onpointermove={(e) => move(e, s)} onpointerup={() => (gesture = null)} title={t('sticker.resize')} role="presentation"><MoveDiagonal2 size={13} /></span>
      {/if}
    </div>
  {/each}
</div>

<style>
  .d-stickers {
    position: absolute;
    inset: 0;
    z-index: 5;
    pointer-events: none;
  }
  .d-sticker {
    position: absolute;
    touch-action: none;
    pointer-events: none;
  }
  .d-stickers.editing .d-sticker {
    pointer-events: auto;
    cursor: grab;
  }
  .d-sticker img {
    display: block;
    width: 100%;
    height: auto;
    user-select: none;
    filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.15));
  }
  .d-sticker.selected {
    outline: 2px dashed var(--primary);
    outline-offset: 4px;
    border-radius: 4px;
  }
  .handle {
    position: absolute;
    width: 24px;
    height: 24px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: 50%;
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow);
    cursor: pointer;
  }
  .del {
    top: -16px;
    right: -16px;
    color: var(--danger);
  }
  .front {
    top: -16px;
    left: -16px;
  }
  .rot {
    bottom: -16px;
    left: -16px;
    cursor: grab;
  }
  .size {
    bottom: -16px;
    right: -16px;
    cursor: nwse-resize;
  }
</style>
