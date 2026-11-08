// Ticking a checklist box in a TipTap editor. TipTap's own handler focuses the editor in the same
// command chain, and in WebKit that focus changes the state first, so the tick was rejected
// ("mismatched transaction") and never saved. This applies the tick to the current document.
import type { Editor } from '@tiptap/core';

/** Listens on `element` (capture phase, before TipTap's handler); returns a function that stops listening. */
export function fixTaskToggles(element: HTMLElement, getEditor: () => Editor | null): () => void {
  const onChange = (event: Event) => {
    const box = event.target;
    const editor = getEditor();
    if (!(box instanceof HTMLInputElement) || box.type !== 'checkbox' || !editor) return;
    const item = box.closest('li[data-checked]');
    if (!item) return;
    event.stopPropagation();
    const view = editor.view;
    const at = view.state.doc.resolve(view.posAtDOM(item, 0));
    for (let depth = at.depth; depth > 0; depth--) {
      const node = at.node(depth);
      if (node.type.name !== 'taskItem') continue;
      view.dispatch(view.state.tr.setNodeMarkup(at.before(depth), undefined, { ...node.attrs, checked: box.checked }));
      return;
    }
  };
  element.addEventListener('change', onChange, true);
  return () => element.removeEventListener('change', onChange, true);
}
