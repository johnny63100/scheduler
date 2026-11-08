<script lang="ts">
  // A commission deadline on the calendar: click to open it on the Commissions page.
  import { Brush } from '@lucide/svelte';
  import type { Commission } from '../lib/api';
  import { commissionFocus, stageOf } from '../lib/commissions.svelte';
  import { t } from '../lib/i18n.svelte';
  import { ui } from '../lib/state.svelte';

  let { commission, compact = false, style = '' }: { commission: Commission; compact?: boolean; style?: string } = $props();
  const stage = $derived(stageOf(commission.status));

  function open(e: MouseEvent) {
    e.stopPropagation();
    commissionFocus.id = commission.id;
    ui.page = 'commissions';
  }
</script>

<button
  class="deadline-chip"
  class:compact
  {style}
  style:--c={stage.color}
  title={`${commission.title} · ${t(stage.label)}\n${t('cal.deadlineHint')}`}
  onclick={open}
  ondblclick={(e) => e.stopPropagation()}
  onpointerdown={(e) => e.stopPropagation()}
>
  <span class="icon"><Brush size={12} /></span>
  <span class="truncate">{commission.title}</span>
</button>

<style>
  .deadline-chip {
    display: flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
    height: 21px;
    margin: 0 5px;
    padding: 0 7px;
    border: none;
    border-radius: 6px;
    background: color-mix(in srgb, var(--c) 16%, var(--surface));
    color: var(--text);
    font-size: 12px;
    font-weight: 600;
    text-align: left;
    cursor: pointer;
    pointer-events: auto;
  }
  .deadline-chip:hover {
    background: color-mix(in srgb, var(--c) 26%, var(--surface));
  }
  .icon {
    display: grid;
    flex: none;
    color: var(--c);
  }
  .compact {
    margin: 0 3px;
  }
</style>
