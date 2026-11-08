<script lang="ts">
  // Diary: one open book spread per day. Pages turn like a real notebook (‹ ›, the page corners or
  // ← →), and the week and year views give an overview of what was written and how the days felt.
  import { onDestroy, onMount } from 'svelte';
  import {
    BookHeart, CalendarDays, CalendarRange, Check, ChevronLeft, ChevronRight, ImagePlus, LoaderCircle, PenLine, RefreshCw,
    Sticker, Trash, X,
  } from '@lucide/svelte';
  import DateField from '../components/DateField.svelte';
  import DiaryEditor from '../components/DiaryEditor.svelte';
  import DiaryStickers from '../components/DiaryStickers.svelte';
  import ConfirmButton from '../components/ConfirmButton.svelte';
  import Select from '../components/Select.svelte';
  import { api, type DiaryEntry, type DiaryLayout, type DiaryMood, type DiaryPaper } from '../lib/api';
  import { addDays, fmt, fmtRange, range, today, weekStart, weekdayNames } from '../lib/dates';
  import { LAYOUTS, MOODS, PAPERS, blankEntry, isBlank, moodOf, textOf } from '../lib/diary';
  import { imageUrl, importImageFile } from '../lib/images';
  import { t } from '../lib/i18n.svelte';
  import { data, load, mutate, toast } from '../lib/state.svelte';
  import { stickers as stickerStore, uploadSticker } from '../lib/stickers.svelte';

  type View = 'day' | 'week' | 'year';
  // A per-viewer convenience, remembered in this browser only.
  const VIEW_KEY = 'nora.diary.view';
  function readView(): View {
    try {
      const v = localStorage.getItem(VIEW_KEY);
      return v === 'week' || v === 'year' ? v : 'day';
    } catch {
      return 'day';
    }
  }
  let view = $state<View>(readView());
  $effect(() => {
    const value = view;
    try {
      localStorage.setItem(VIEW_KEY, value);
    } catch {
      /* not remembered; fine */
    }
  });

  const todayStr = today();
  let day = $state(todayStr);

  // ---- the open page (day view). Saved as you go; blank pages are not kept.
  /** Layout and paper of new pages: the last ones chosen. */
  const STYLE_SETTING = 'diary.style';
  let style = $state<{ layout: DiaryLayout; paper: DiaryPaper } | null>(null);
  let entry = $state<DiaryEntry>(blankEntry(todayStr));
  /** Bumped when a day is loaded, so the editor starts over with that day's text. */
  let loadKey = $state(0);
  let dirty = $state(false);
  let saving = $state(false);
  let saveTimer: ReturnType<typeof setTimeout> | undefined;

  async function flush() {
    clearTimeout(saveTimer);
    if (!dirty) return;
    dirty = false;
    saving = true;
    try {
      await api.saveDiaryEntry($state.snapshot(entry));
    } catch (e) {
      toast(String(e), 'error');
    } finally {
      saving = false;
    }
  }

  function change(patch: Partial<DiaryEntry>) {
    entry = { ...entry, ...patch };
    dirty = true;
    clearTimeout(saveTimer);
    saveTimer = setTimeout(flush, 700);
  }

  async function open(d: string) {
    await flush();
    const list = await load(api.diaryBetween(d, d), []);
    entry = list[0] ?? blankEntry(d, style ? ({ ...style } as DiaryEntry) : null);
    loadKey++;
  }

  onMount(async () => {
    try {
      const saved = JSON.parse((await api.getSetting(STYLE_SETTING).catch(() => null)) ?? 'null');
      if (saved && LAYOUTS.some((l) => l.id === saved.layout) && PAPERS.some((p) => p.id === saved.paper)) style = saved;
    } catch {
      /* default style */
    }
    open(day);
  });
  onDestroy(() => {
    flush();
  });

  function setStyle(patch: Partial<{ layout: DiaryLayout; paper: DiaryPaper }>) {
    change(patch);
    style = { layout: entry.layout, paper: entry.paper };
    api.setSetting(STYLE_SETTING, JSON.stringify(style)).catch(() => {});
  }

  // ---- turning pages
  let turning = $state<'next' | 'prev' | null>(null);
  const TURN_MS = 650;

  /** Turns to `d`: a sheet sweeps over the spine while the new day's page comes in underneath. */
  function goTo(d: string) {
    if (d === day || turning) return;
    const dir = d > day ? 'next' : 'prev';
    if (view !== 'day') {
      day = d;
      return;
    }
    turning = dir;
    flush();
    setTimeout(() => {
      day = d;
      open(d);
    }, TURN_MS / 2);
    setTimeout(() => (turning = null), TURN_MS);
  }
  const step = (n: number) => goTo(addDays(day, n));

  function onkeydown(e: KeyboardEvent) {
    if (view !== 'day' || decorating) return;
    const el = e.target as HTMLElement;
    if (el.isContentEditable || el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) return;
    if (e.key === 'ArrowLeft') step(-1);
    else if (e.key === 'ArrowRight') step(1);
  }

  function openDay(d: string) {
    view = 'day';
    day = d;
    open(d);
  }

  // ---- photo
  let photoInput: HTMLInputElement;
  let photoBusy = $state(false);
  async function usePhoto(file: File | undefined) {
    if (!file) return;
    photoBusy = true;
    try {
      change({ photo: await importImageFile(file) });
    } catch (e) {
      toast(e instanceof Error ? e.message : String(e), 'error');
    } finally {
      photoBusy = false;
      photoInput.value = '';
    }
  }

  // ---- stickers
  let decorating = $state(false);
  let stickerInput: HTMLInputElement;
  let stickerBusy = $state(false);
  let counter = 0;
  function placeSticker(image: string) {
    const jitter = () => (Math.random() - 0.5) * 0.16;
    const id = `${Date.now().toString(36)}-${(counter++).toString(36)}`;
    change({ stickers: [...entry.stickers, { id, image, x: 0.5 + jitter(), y: 0.45 + jitter(), w: 0.12, rot: Math.round(jitter() * 60) }] });
  }
  async function uploadNewSticker(file: File | undefined) {
    if (!file) return;
    stickerBusy = true;
    try {
      placeSticker(await uploadSticker(file));
    } catch (e) {
      toast(e instanceof Error ? e.message : String(e), 'error');
    } finally {
      stickerBusy = false;
      stickerInput.value = '';
    }
  }

  async function removeDay() {
    const date = entry.date;
    clearTimeout(saveTimer);
    dirty = false;
    try {
      const stored = await api.deleteDiaryEntry(date);
      entry = blankEntry(date, style ? ({ ...style } as DiaryEntry) : null);
      loadKey++;
      data.version++;
      toast(t('diary.deleted'), 'info', {
        label: t('common.undo'),
        run: () => {
          if (!stored) return;
          mutate(api.saveDiaryEntry(stored)).then(() => entry.date === stored.date && ((entry = stored), loadKey++));
        },
      });
    } catch (e) {
      toast(String(e), 'error');
    }
  }

  // ---- week and year overviews
  let overview = $state<DiaryEntry[]>([]);
  let year = $state(todayStr.slice(0, 4));
  const weekDays = $derived(range(weekStart(day), 7));
  $effect(() => {
    data.version;
    if (view === 'week') {
      const [from, to] = [weekDays[0], weekDays[6]];
      load(api.diaryBetween(from, to), []).then((e) => (overview = e));
    } else if (view === 'year') {
      const y = year;
      load(api.diaryBetween(`${y}-01-01`, `${y}-12-31`), []).then((e) => (overview = e));
    }
  });
  const byDate = $derived(new Map(overview.map((e) => [e.date, e])));
  const moodCounts = $derived(
    MOODS.map((m) => ({ ...m, n: overview.filter((e) => e.mood === m.id).length })).filter((m) => m.n > 0).sort((a, b) => b.n - a.n),
  );
  const months = $derived(
    Array.from({ length: 12 }, (_, m) => {
      const first = `${year}-${String(m + 1).padStart(2, '0')}-01`;
      const days = new Date(Number(year), m + 1, 0).getDate();
      return { first, lead: new Date(Number(year), m, 1).getDay(), days: range(first, days) };
    }),
  );

  const layoutOptions = $derived(LAYOUTS.map((l) => ({ value: l.id, label: t(l.label) })));
  const paperOptions = $derived(PAPERS.map((p) => ({ value: p.id, label: t(p.label) })));
  const mood = $derived(moodOf(entry.mood));
</script>

<svelte:window {onkeydown} />
<input bind:this={photoInput} type="file" accept="image/*" hidden onchange={(e) => usePhoto(e.currentTarget.files?.[0])} />
<input bind:this={stickerInput} type="file" accept="image/*" hidden onchange={(e) => uploadNewSticker(e.currentTarget.files?.[0])} />

<div class="page diary">
  <div class="page-header">
    <div>
      <h1>{t('nav.diary')}</h1>
      <p class="sub">{t('diary.subtitle')}</p>
    </div>
    <span class="spacer"></span>
    <div class="segmented" role="group" aria-label={t('diary.view')}>
      <button class:active={view === 'day'} onclick={() => openDay(day)}><BookHeart size={15} /> {t('diary.viewDay')}</button>
      <button class:active={view === 'week'} onclick={() => (view = 'week')}><CalendarRange size={15} /> {t('diary.viewWeek')}</button>
      <button class:active={view === 'year'} onclick={() => { year = day.slice(0, 4); view = 'year'; }}><CalendarDays size={15} /> {t('diary.viewYear')}</button>
    </div>
  </div>

  {#if view === 'day'}
    <div class="toolbar">
      <div class="nav">
        <button class="icon-btn" onclick={() => step(-1)} aria-label={t('diary.prevDay')} title={t('diary.prevDay')}><ChevronLeft size={18} /></button>
        <div class="date-pick"><DateField value={day} label={t('common.date')} onchange={(v) => v && goTo(v)} /></div>
        <button class="icon-btn" onclick={() => step(1)} aria-label={t('diary.nextDay')} title={t('diary.nextDay')}><ChevronRight size={18} /></button>
        {#if day !== todayStr}<button class="btn small" onclick={() => goTo(todayStr)}>{t('common.today')}</button>{/if}
      </div>
      <span class="spacer"></span>
      <span class="faint small save-state">{saving || dirty ? t('memo.saving') : isBlank(entry) ? t('diary.emptyPage') : t('memo.allSaved')}</span>
      <div class="pick"><Select value={entry.layout} options={layoutOptions} label={t('diary.layout')} onchange={(v) => setStyle({ layout: v })} /></div>
      <div class="pick"><Select value={entry.paper} options={paperOptions} label={t('diary.paper')} onchange={(v) => setStyle({ paper: v })} /></div>
      <button class="btn" class:primary={decorating} onclick={() => (decorating = !decorating)}>
        {#if decorating}<Check size={16} /> {t('home.done')}{:else}<Sticker size={16} /> {t('diary.decorate')}{/if}
      </button>
      {#if !isBlank(entry)}<ConfirmButton question={t('diary.deleteQ')} onconfirm={removeDay} />{/if}
    </div>

    {#if decorating}
      <div class="tray card">
        <span class="muted small">{t('diary.stickerHint')}</span>
        <div class="library">
          <button class="add" onclick={() => stickerInput.click()} disabled={stickerBusy} title={t('sticker.upload')}>
            {#if stickerBusy}<span class="spin"><LoaderCircle size={20} /></span>{:else}<ImagePlus size={20} />{/if}
          </button>
          {#each stickerStore.library as image (image)}
            <button class="thumb" onclick={() => placeSticker(image)} title={t('sticker.place')}><img src={imageUrl(image)} alt="" draggable="false" /></button>
          {:else}
            <span class="faint small">{t('sticker.empty')}</span>
          {/each}
        </div>
      </div>
    {/if}

    <div class="book-wrap">
      <div class="book layout-{entry.layout}" class:turning={!!turning} class:decorating>
        <!-- Left page -->
        <section class="leaf left paper-plain">
          {#if entry.layout === 'photo'}
            {@render photoSlot(true)}
            <input class="line-input caption" placeholder={t('diary.captionPlaceholder')} value={entry.line} oninput={(e) => change({ line: e.currentTarget.value })} />
          {:else}
            {@render dateBlock(entry.layout === 'simple')}
            {@render moodPicker(entry.layout === 'simple')}
            <label class="line-field">
              <span class="faint small">{t('diary.lineLabel')}</span>
              <input class="line-input" class:big={entry.layout === 'simple'} placeholder={t('diary.linePlaceholder')} value={entry.line}
                oninput={(e) => change({ line: e.currentTarget.value })} />
            </label>
            {#if entry.layout === 'classic'}{@render photoSlot(false)}{/if}
          {/if}
          <button class="corner prev" onclick={() => step(-1)} aria-label={t('diary.prevDay')} title={t('diary.prevDay')}></button>
        </section>

        <div class="spine" aria-hidden="true"></div>

        <!-- Right page -->
        <section class="leaf right paper-{entry.paper}">
          {#if entry.layout === 'photo'}
            <div class="mini-head">
              {@render dateBlock(false, true)}
              {@render moodPicker(false, true)}
            </div>
          {/if}
          <div class="writing">
            {#key loadKey}
              <DiaryEditor value={entry.body} onchange={(html) => change({ body: html })} />
            {/key}
          </div>
          <button class="corner next" onclick={() => step(1)} aria-label={t('diary.nextDay')} title={t('diary.nextDay')}></button>
        </section>

        <DiaryStickers stickers={entry.stickers} editing={decorating} onchange={(list) => change({ stickers: list })} />

        {#if turning}
          <div class="sheet {turning}" aria-hidden="true"><div class="face front"></div><div class="face back"></div></div>
        {/if}
      </div>
    </div>
  {:else if view === 'week'}
    <div class="toolbar">
      <div class="nav">
        <button class="icon-btn" onclick={() => (day = addDays(day, -7))} aria-label={t('common.previous')}><ChevronLeft size={18} /></button>
        <span class="range strong">{fmtRange(weekDays[0], weekDays[6], { month: 'short', day: 'numeric' })}</span>
        <button class="icon-btn" onclick={() => (day = addDays(day, 7))} aria-label={t('common.next')}><ChevronRight size={18} /></button>
        {#if !weekDays.includes(todayStr)}<button class="btn small" onclick={() => (day = todayStr)}>{t('common.today')}</button>{/if}
      </div>
      <span class="spacer"></span>
      {#if moodCounts.length}
        <span class="mood-sum">
          {#each moodCounts as m (m.id)}<span class="mood-chip" style:--m={m.color} title={t(m.label)}><m.icon size={14} /> {m.n}</span>{/each}
        </span>
      {/if}
    </div>
    <div class="week">
      {#each weekDays as d (d)}
        {@const e = byDate.get(d)}
        {@const m = e ? moodOf(e.mood) : undefined}
        <button class="day-card" class:today={d === todayStr} class:empty={!e} style:--m={m?.color ?? 'var(--border)'} onclick={() => openDay(d)}>
          <span class="dc-head">
            <span class="dc-wd">{fmt(d, { weekday: 'short' })}</span>
            <span class="dc-num">{Number(d.slice(8))}</span>
            {#if m}<span class="dc-mood" title={t(m.label)}><m.icon size={18} /></span>{/if}
          </span>
          {#if e?.photo}<img class="dc-photo" src={imageUrl(e.photo, 'diary-thumb')} alt="" draggable="false" />{/if}
          {#if e}
            {#if e.line}<span class="dc-line">{e.line}</span>{/if}
            <span class="dc-text">{textOf(e.body)}</span>
          {:else}
            <span class="dc-write faint small"><PenLine size={14} /> {t('diary.write')}</span>
          {/if}
        </button>
      {/each}
    </div>
  {:else}
    <div class="toolbar">
      <div class="nav">
        <button class="icon-btn" onclick={() => (year = String(Number(year) - 1))} aria-label={t('common.previous')}><ChevronLeft size={18} /></button>
        <span class="range strong">{t('trpg.year', { year })}</span>
        <button class="icon-btn" onclick={() => (year = String(Number(year) + 1))} aria-label={t('common.next')}><ChevronRight size={18} /></button>
      </div>
      <span class="spacer"></span>
      <span class="muted small">{t('diary.daysWritten', { n: overview.length })}</span>
    </div>
    {#if moodCounts.length}
      <div class="card mood-bar-card">
        <div class="mood-bar">
          {#each moodCounts as m (m.id)}<span style:flex={m.n} style:background={m.color} title="{t(m.label)} {m.n}"></span>{/each}
        </div>
        <div class="legend">
          {#each moodCounts as m (m.id)}<span class="lg" style:--m={m.color}><m.icon size={14} /> {t(m.label)} <b>{m.n}</b></span>{/each}
        </div>
      </div>
    {/if}
    <div class="year">
      {#each months as mo (mo.first)}
        <div class="card month">
          <div class="month-title strong">{fmt(mo.first, { month: 'long' })}</div>
          <div class="dow">{#each weekdayNames('narrow') as w, i (i)}<span>{w}</span>{/each}</div>
          <div class="cells">
            {#each Array(mo.lead) as _, i (i)}<span></span>{/each}
            {#each mo.days as d (d)}
              {@const e = byDate.get(d)}
              {@const m = e ? moodOf(e.mood) : undefined}
              <button
                class="cell"
                class:has={!!e}
                class:today={d === todayStr}
                style:--m={m?.color ?? 'var(--primary)'}
                onclick={() => openDay(d)}
                title={`${fmt(d, { month: 'long', day: 'numeric', weekday: 'short' })}${m ? ` · ${t(m.label)}` : ''}${e?.line ? `\n${e.line}` : ''}`}
              >{Number(d.slice(8))}</button>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

{#snippet dateBlock(big: boolean, compact = false)}
  <div class="date-block" class:big class:compact>
    <span class="d-num">{Number(entry.date.slice(8))}</span>
    <span class="d-rest">
      <span class="d-wd strong">{fmt(entry.date, { weekday: 'long' })}</span>
      <span class="faint small">{fmt(entry.date, { year: 'numeric', month: 'long' })}</span>
    </span>
  </div>
{/snippet}

{#snippet moodPicker(big: boolean, compact = false)}
  <div class="mood-field" class:compact>
    {#if !compact}<span class="faint small">{t('diary.moodLabel')}{#if mood} · <b style:color={mood.color}>{t(mood.label)}</b>{/if}</span>{/if}
    <div class="moods" class:big role="radiogroup" aria-label={t('diary.moodLabel')}>
      {#each MOODS as m (m.id)}
        <button
          type="button"
          class="mood"
          class:on={entry.mood === m.id}
          style:--m={m.color}
          role="radio"
          aria-checked={entry.mood === m.id}
          title={t(m.label)}
          aria-label={t(m.label)}
          onclick={() => change({ mood: (entry.mood === m.id ? '' : m.id) as DiaryMood })}
        ><m.icon size={big ? 26 : compact ? 16 : 20} /></button>
      {/each}
    </div>
  </div>
{/snippet}

{#snippet photoSlot(large: boolean)}
  <div class="photo" class:large>
    {#if photoBusy}
      <span class="photo-empty"><span class="spin"><LoaderCircle size={22} /></span></span>
    {:else if entry.photo}
      <img src={imageUrl(entry.photo, 'diary')} alt="" draggable="false" />
      <span class="photo-actions">
        <button class="btn small" onclick={() => photoInput.click()}><RefreshCw size={13} /> {t('dd.changePhoto')}</button>
        <button class="icon-btn" onclick={() => change({ photo: null })} aria-label={t('dd.removePhoto')} title={t('dd.removePhoto')}><X size={15} /></button>
      </span>
    {:else}
      <button class="photo-empty" onclick={() => photoInput.click()}><ImagePlus size={large ? 30 : 22} /> {t('diary.addPhoto')}</button>
    {/if}
  </div>
{/snippet}

<style>
  .diary {
    /* Paper and ink follow the theme: warm paper in light mode, soft dark paper in dark mode. */
    --paper-bg: color-mix(in srgb, var(--surface) 90%, #f3e3c3);
    --paper-ink: var(--text);
    --paper-rule: color-mix(in srgb, var(--primary) 20%, transparent);
    --paper-line: 30px;
  }
  .strong {
    font-weight: 650;
  }
  .toolbar {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 12px;
  }
  .nav {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .date-pick {
    width: 220px;
  }
  .pick {
    width: 150px;
  }
  .save-state {
    margin-right: 4px;
  }
  .range {
    min-width: 150px;
    text-align: center;
  }

  /* ---- sticker tray */
  .tray {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 14px;
    margin-bottom: 12px;
  }
  .library {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .add,
  .thumb {
    width: 56px;
    height: 56px;
    display: grid;
    place-items: center;
    padding: 4px;
    border: 1.5px dashed var(--border);
    border-radius: 12px;
    background: var(--surface-2);
    color: var(--muted);
    cursor: pointer;
  }
  .thumb {
    border-style: solid;
    background: var(--surface);
  }
  .thumb:hover,
  .add:hover {
    border-color: var(--primary);
  }
  .thumb img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
  }
  .spin {
    display: grid;
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  /* ---- the open book */
  .book-wrap {
    display: flex;
    justify-content: center;
  }
  .book {
    position: relative;
    display: flex;
    width: min(1120px, 100%);
    height: clamp(520px, calc(100vh - 290px), 780px);
    padding: 10px;
    border-radius: 18px;
    background: color-mix(in srgb, var(--primary) 55%, #5b4636);
    box-shadow: var(--shadow-lg);
  }
  .leaf {
    position: relative;
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 28px 30px 30px;
    background: var(--paper-bg);
    color: var(--paper-ink);
    overflow: hidden;
    transition: opacity 0.25s ease;
  }
  .leaf.left {
    border-radius: 10px 2px 2px 10px;
    box-shadow: inset -18px 0 24px -18px rgba(0, 0, 0, 0.22);
  }
  .leaf.right {
    border-radius: 2px 10px 10px 2px;
    box-shadow: inset 18px 0 24px -18px rgba(0, 0, 0, 0.22);
  }
  /* While a page turns, the writing fades so only paper moves. */
  .book.turning .leaf > :not(.corner) {
    opacity: 0;
    transition: opacity 0.12s ease;
  }
  .spine {
    width: 2px;
    flex: none;
    background: linear-gradient(to bottom, rgba(0, 0, 0, 0.2), rgba(0, 0, 0, 0.35), rgba(0, 0, 0, 0.2));
  }

  /* Paper for the writing page; the lines scroll with the text. */
  .writing {
    flex: 1;
    min-height: 0;
  }
  .leaf :global(.diary-editor .body) {
    background-attachment: local;
  }
  .paper-lined :global(.diary-editor .body) {
    background-image: repeating-linear-gradient(to bottom, transparent 0, transparent calc(var(--paper-line) - 1px), var(--paper-rule) calc(var(--paper-line) - 1px), var(--paper-rule) var(--paper-line));
  }
  .paper-grid :global(.diary-editor .body) {
    background-image: linear-gradient(var(--paper-rule) 1px, transparent 1px), linear-gradient(90deg, var(--paper-rule) 1px, transparent 1px);
    background-size: calc(var(--paper-line) / 2) calc(var(--paper-line) / 2);
  }
  .paper-dots :global(.diary-editor .body) {
    background-image: radial-gradient(circle, color-mix(in srgb, var(--paper-ink) 28%, transparent) 1.2px, transparent 1.4px);
    background-size: var(--paper-line) var(--paper-line);
    background-position: calc(var(--paper-line) / -2) calc(var(--paper-line) / 2);
  }

  /* Page corners: click to turn. */
  .corner {
    position: absolute;
    bottom: 0;
    width: 44px;
    height: 44px;
    border: none;
    padding: 0;
    background: none;
    cursor: pointer;
    opacity: 0.55;
    transition: opacity 0.15s, transform 0.15s;
  }
  .corner.prev {
    left: 0;
    background: linear-gradient(45deg, color-mix(in srgb, var(--paper-ink) 14%, var(--paper-bg)) 50%, transparent 50%);
    border-bottom-left-radius: 10px;
  }
  .corner.next {
    right: 0;
    background: linear-gradient(-45deg, color-mix(in srgb, var(--paper-ink) 14%, var(--paper-bg)) 50%, transparent 50%);
    border-bottom-right-radius: 10px;
  }
  .corner:hover {
    opacity: 1;
    transform: scale(1.15);
  }

  /* The sheet that turns over the spine. */
  .sheet {
    position: absolute;
    top: 10px;
    bottom: 10px;
    width: calc(50% - 11px);
    z-index: 7;
    transform-style: preserve-3d;
    pointer-events: none;
  }
  .sheet.next {
    left: 50%;
    transform-origin: left center;
    animation: turn-next 0.65s cubic-bezier(0.45, 0.05, 0.4, 1) forwards;
  }
  .sheet.prev {
    left: 11px;
    transform-origin: right center;
    animation: turn-prev 0.65s cubic-bezier(0.45, 0.05, 0.4, 1) forwards;
  }
  .face {
    position: absolute;
    inset: 0;
    backface-visibility: hidden;
    background: var(--paper-bg);
    border-radius: 4px;
  }
  .sheet.next .front {
    background: linear-gradient(to right, rgba(0, 0, 0, 0.12), transparent 30%), var(--paper-bg);
  }
  .sheet.prev .front {
    background: linear-gradient(to left, rgba(0, 0, 0, 0.12), transparent 30%), var(--paper-bg);
  }
  .back {
    transform: rotateY(180deg);
    background: linear-gradient(to left, rgba(0, 0, 0, 0.1), transparent 35%), var(--paper-bg);
  }
  @keyframes turn-next {
    from {
      transform: perspective(2400px) rotateY(0deg);
      box-shadow: 0 0 0 rgba(0, 0, 0, 0);
    }
    50% {
      box-shadow: -20px 0 40px rgba(0, 0, 0, 0.18);
    }
    to {
      transform: perspective(2400px) rotateY(-180deg);
    }
  }
  @keyframes turn-prev {
    from {
      transform: perspective(2400px) rotateY(0deg);
    }
    50% {
      box-shadow: 20px 0 40px rgba(0, 0, 0, 0.18);
    }
    to {
      transform: perspective(2400px) rotateY(180deg);
    }
  }

  /* ---- page contents */
  .date-block {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .d-num {
    font-size: 54px;
    font-weight: 800;
    line-height: 1;
    letter-spacing: -0.04em;
    color: var(--primary);
  }
  .date-block.big .d-num {
    font-size: 84px;
  }
  .date-block.compact .d-num {
    font-size: 30px;
  }
  .d-rest {
    display: flex;
    flex-direction: column;
  }
  .mood-field {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .moods {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .mood {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border: 1.5px solid transparent;
    border-radius: 50%;
    background: color-mix(in srgb, var(--paper-ink) 5%, transparent);
    color: color-mix(in srgb, var(--paper-ink) 40%, transparent);
    cursor: pointer;
    transition: transform 0.12s, background 0.12s, color 0.12s;
  }
  .moods.big .mood {
    width: 52px;
    height: 52px;
  }
  .mood-field.compact .mood {
    width: 30px;
    height: 30px;
  }
  .mood:hover {
    color: var(--m);
    transform: scale(1.08);
  }
  .mood.on {
    color: var(--m);
    border-color: var(--m);
    background: color-mix(in srgb, var(--m) 16%, transparent);
  }
  .line-field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .line-input {
    width: 100%;
    padding: 6px 2px;
    border: none;
    border-bottom: 1.5px solid var(--paper-rule);
    background: none;
    color: var(--paper-ink);
    font-size: 16px;
    font-weight: 600;
    outline: none;
  }
  .line-input:focus {
    border-bottom-color: var(--primary);
  }
  .line-input.big {
    font-size: 22px;
  }
  .line-input.caption {
    text-align: center;
    font-size: 15px;
  }
  .photo {
    position: relative;
    flex: 1;
    min-height: 120px;
    max-height: 46%;
    display: flex;
  }
  .photo.large {
    max-height: none;
  }
  .photo img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    border: 8px solid #fff;
    border-bottom-width: 22px;
    border-radius: 3px;
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.18);
    transform: rotate(-1.5deg);
  }
  .photo-actions {
    position: absolute;
    right: 12px;
    top: 12px;
    display: flex;
    gap: 4px;
    opacity: 0;
    transition: opacity 0.15s;
  }
  .photo:hover .photo-actions {
    opacity: 1;
  }
  .photo-actions .icon-btn {
    background: var(--surface);
  }
  .photo-empty {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    border: 1.5px dashed color-mix(in srgb, var(--paper-ink) 22%, transparent);
    border-radius: 12px;
    background: none;
    color: color-mix(in srgb, var(--paper-ink) 45%, transparent);
    font-weight: 600;
    cursor: pointer;
  }
  .photo-empty:hover {
    border-color: var(--primary);
    color: var(--primary);
  }
  .mini-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 10px;
  }
  .book.decorating .leaf {
    outline: 2px dashed color-mix(in srgb, var(--primary) 35%, transparent);
    outline-offset: -8px;
  }

  /* ---- week */
  .mood-sum {
    display: flex;
    gap: 6px;
  }
  .mood-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 26px;
    padding: 0 9px;
    border-radius: var(--radius-pill);
    font-size: 12.5px;
    font-weight: 700;
    color: var(--m);
    background: color-mix(in srgb, var(--m) 13%, transparent);
  }
  .week {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    gap: 10px;
  }
  .day-card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
    min-height: 260px;
    padding: 14px;
    border: 1px solid var(--border);
    border-top: 4px solid var(--m);
    border-radius: var(--radius-lg);
    background: var(--paper-bg);
    color: var(--text);
    text-align: left;
    cursor: pointer;
    transition: transform 0.12s, box-shadow 0.12s;
  }
  .day-card:hover {
    transform: translateY(-2px);
    box-shadow: var(--shadow);
  }
  .day-card.empty {
    background: var(--surface);
  }
  .day-card.today .dc-num {
    color: var(--primary);
  }
  .dc-head {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }
  .dc-wd {
    font-size: 12px;
    font-weight: 650;
    color: var(--muted);
  }
  .dc-num {
    font-size: 22px;
    font-weight: 800;
  }
  .dc-mood {
    margin-left: auto;
    color: var(--m);
  }
  .dc-photo {
    width: 100%;
    height: 90px;
    object-fit: cover;
    border-radius: 8px;
  }
  .dc-line {
    font-weight: 700;
    font-size: 13.5px;
  }
  .dc-text {
    font-size: 12.5px;
    color: var(--muted);
    display: -webkit-box;
    -webkit-line-clamp: 6;
    line-clamp: 6;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .dc-write {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    margin-top: auto;
  }

  /* ---- year */
  .mood-bar-card {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-bottom: 12px;
    padding: 16px 18px;
  }
  .mood-bar {
    display: flex;
    height: 12px;
    border-radius: 12px;
    overflow: hidden;
  }
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 16px;
    font-size: 13px;
    color: var(--muted);
  }
  .lg {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .lg :global(svg) {
    color: var(--m);
  }
  .lg b {
    color: var(--text);
  }
  .year {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(220px, 100%), 1fr));
    gap: 12px;
  }
  .month {
    padding: 14px;
  }
  .month-title {
    margin-bottom: 8px;
  }
  .dow,
  .cells {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    gap: 3px;
  }
  .dow span {
    text-align: center;
    font-size: 10.5px;
    color: var(--faint);
  }
  .cell {
    aspect-ratio: 1;
    min-width: 0;
    padding: 0;
    border: none;
    border-radius: 7px;
    background: var(--surface-2);
    color: var(--faint);
    font-size: 10.5px;
    font-weight: 600;
    cursor: pointer;
  }
  .cell.has {
    background: color-mix(in srgb, var(--m) 70%, var(--surface));
    color: #fff;
  }
  .cell.today {
    box-shadow: 0 0 0 2px var(--primary);
  }
  .cell:hover {
    filter: brightness(0.95);
  }

  @container main (max-width: 820px) {
    .book {
      flex-direction: column;
      height: auto;
    }
    .leaf {
      min-height: 360px;
    }
    .spine,
    .sheet {
      display: none;
    }
    .week {
      grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    }
  }
</style>
