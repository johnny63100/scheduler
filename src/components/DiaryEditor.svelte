<script lang="ts">
  // The writing area of a diary page: a small rich text editor with highlighter pens.
  // Content goes in and comes out as HTML, which keeps each highlighter color.
  import { onDestroy, onMount } from 'svelte';
  import { Editor } from '@tiptap/core';
  import StarterKit from '@tiptap/starter-kit';
  import Highlight from '@tiptap/extension-highlight';
  import { TaskItem, TaskList } from '@tiptap/extension-list';
  import { Placeholder } from '@tiptap/extensions';
  import { Bold, Eraser, Highlighter, Italic, List, ListChecks, Strikethrough, Underline } from '@lucide/svelte';
  import { HIGHLIGHTS } from '../lib/diary';
  import { t } from '../lib/i18n.svelte';
  import { fixTaskToggles } from '../lib/taskToggle';

  let { value, onchange, placeholder }: { value: string; onchange: (html: string) => void; placeholder?: string } = $props();

  let element: HTMLDivElement;
  let stopTaskFix: (() => void) | undefined;
  let editor = $state<Editor | null>(null);
  /** Bumped on every editor transaction so toolbar states re-evaluate. */
  let version = $state(0);
  /** The pen used by the highlighter button; the arrow next to it picks another color. */
  let pen = $state(HIGHLIGHTS[0].color);

  onMount(() => {
    editor = new Editor({
      element,
      extensions: [
        StarterKit.configure({ link: false, heading: { levels: [2, 3] }, codeBlock: false, code: false }),
        Highlight.configure({ multicolor: true }),
        TaskList,
        TaskItem.configure({ nested: true }),
        Placeholder.configure({ placeholder: () => placeholder ?? t('diary.placeholder') }),
      ],
      content: value,
      editorProps: { attributes: { class: 'diary-text', spellcheck: 'false' } },
      onUpdate: ({ editor }) => onchange(editor.isEmpty ? '' : editor.getHTML()),
      onTransaction: () => version++,
    });
    stopTaskFix = fixTaskToggles(element, () => editor);
  });

  onDestroy(() => {
    stopTaskFix?.();
    editor?.destroy();
  });

  function active(name: string, attrs?: Record<string, unknown>) {
    version; // re-run when the editor changes
    return editor?.isActive(name, attrs) ?? false;
  }

  const chain = () => editor!.chain().focus();

  function highlight(color: string) {
    pen = color;
    chain().toggleHighlight({ color }).run();
  }
</script>

<div class="diary-editor">
  <div class="tools" role="toolbar" aria-label={t('diary.tools')}>
    <button type="button" class:on={active('bold')} onclick={() => chain().toggleBold().run()} title={t('rt.bold')} aria-label={t('rt.bold')}><Bold size={15} /></button>
    <button type="button" class:on={active('italic')} onclick={() => chain().toggleItalic().run()} title={t('rt.italic')} aria-label={t('rt.italic')}><Italic size={15} /></button>
    <button type="button" class:on={active('underline')} onclick={() => chain().toggleUnderline().run()} title={t('rt.underline')} aria-label={t('rt.underline')}><Underline size={15} /></button>
    <button type="button" class:on={active('strike')} onclick={() => chain().toggleStrike().run()} title={t('rt.strike')} aria-label={t('rt.strike')}><Strikethrough size={15} /></button>
    <span class="sep"></span>
    <button type="button" class="pen" style:--pen={pen} class:on={active('highlight', { color: pen })} onclick={() => highlight(pen)} title={t('diary.highlighter')} aria-label={t('diary.highlighter')}><Highlighter size={15} /></button>
    {#each HIGHLIGHTS as h (h.color)}
      <button type="button" class="swatch" class:picked={pen === h.color} style:--pen={h.color} onclick={() => highlight(h.color)} title={t(h.label)} aria-label={t(h.label)}></button>
    {/each}
    <button type="button" onclick={() => chain().unsetHighlight().run()} title={t('diary.unhighlight')} aria-label={t('diary.unhighlight')}><Eraser size={15} /></button>
    <span class="sep"></span>
    <button type="button" class:on={active('bulletList')} onclick={() => chain().toggleBulletList().run()} title={t('rt.bullet')} aria-label={t('rt.bullet')}><List size={15} /></button>
    <button type="button" class:on={active('taskList')} onclick={() => chain().toggleTaskList().run()} title={t('rt.checklist')} aria-label={t('rt.checklist')}><ListChecks size={15} /></button>
  </div>
  <div class="body" bind:this={element}></div>
</div>

<style>
  .diary-editor {
    display: flex;
    flex-direction: column;
    min-height: 0;
    height: 100%;
  }
  .tools {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 2px;
    padding: 3px;
    margin-bottom: 8px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--paper-ink) 6%, transparent);
  }
  .tools button {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border: none;
    border-radius: 7px;
    background: none;
    color: color-mix(in srgb, var(--paper-ink) 70%, transparent);
    cursor: pointer;
  }
  .tools button:hover {
    background: color-mix(in srgb, var(--paper-ink) 8%, transparent);
  }
  .tools button.on {
    background: color-mix(in srgb, var(--paper-ink) 14%, transparent);
    color: var(--paper-ink);
  }
  .pen {
    box-shadow: inset 0 -4px 0 var(--pen);
  }
  .swatch {
    width: 18px !important;
    height: 18px !important;
    margin: 0 2px;
    border-radius: 50% !important;
    background: var(--pen) !important;
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--paper-ink) 20%, transparent);
  }
  .swatch.picked {
    box-shadow: 0 0 0 2px var(--paper-bg), 0 0 0 4px color-mix(in srgb, var(--paper-ink) 45%, transparent);
  }
  .sep {
    width: 1px;
    height: 18px;
    margin: 0 4px;
    background: color-mix(in srgb, var(--paper-ink) 15%, transparent);
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  .body :global(.diary-text) {
    min-height: 100%;
    outline: none;
    color: var(--paper-ink);
    font-size: 15px;
    /* Matches the ruled lines of the paper. */
    line-height: var(--paper-line);
    word-wrap: break-word;
  }
  .body :global(.diary-text p) {
    margin: 0;
  }
  .body :global(.diary-text h2),
  .body :global(.diary-text h3) {
    margin: 0;
    line-height: var(--paper-line);
  }
  .body :global(.diary-text ul),
  .body :global(.diary-text ol) {
    margin: 0;
    padding-left: 1.3em;
  }
  .body :global(.diary-text mark) {
    padding: 0 2px;
    border-radius: 3px;
    /* TipTap writes "color: inherit" inline; highlighter colors are light, so keep dark text on them. */
    color: #1e2232 !important;
  }
  .body :global(.diary-text ul[data-type='taskList']) {
    list-style: none;
    padding-left: 0.1em;
  }
  .body :global(.diary-text li[data-checked]) {
    display: flex;
    align-items: flex-start;
    gap: 8px;
  }
  .body :global(.diary-text li[data-checked] > label) {
    display: flex;
    align-items: center;
    height: var(--paper-line);
    flex: none;
  }
  .body :global(.diary-text li[data-checked] > div) {
    flex: 1;
    min-width: 0;
  }
  .body :global(.diary-text li[data-checked='true'] > div) {
    text-decoration: line-through;
    opacity: 0.55;
  }
  .body :global(.diary-text input[type='checkbox']) {
    margin: 0;
    accent-color: var(--primary);
  }
  .body :global(.diary-text p.is-editor-empty:first-child::before) {
    content: attr(data-placeholder);
    float: left;
    height: 0;
    pointer-events: none;
    color: color-mix(in srgb, var(--paper-ink) 35%, transparent);
  }
</style>
