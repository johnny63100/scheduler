// Diary: moods, page layouts, paper styles and highlighter colors.
import type { Component } from 'svelte';
import {
  BatteryLow, FaceAngry, FaceGrinning, FaceNeutral, FaceSlightlyFrowning, FaceSlightlySmiling, Heart,
} from '@lucide/svelte';
import type { DateStr, DiaryEntry, DiaryLayout, DiaryMood, DiaryPaper } from './api';
import type { Key } from './i18n.svelte';

type Icon = Component<{ size?: number }>;

export const MOODS: { id: Exclude<DiaryMood, ''>; label: Key; icon: Icon; color: string }[] = [
  { id: 'great', label: 'diary.mood.great', icon: FaceGrinning, color: '#f5b83d' },
  { id: 'good', label: 'diary.mood.good', icon: FaceSlightlySmiling, color: '#34c38f' },
  { id: 'love', label: 'diary.mood.love', icon: Heart, color: '#f2668b' },
  { id: 'okay', label: 'diary.mood.okay', icon: FaceNeutral, color: '#94a3b8' },
  { id: 'tired', label: 'diary.mood.tired', icon: BatteryLow, color: '#b164e8' },
  { id: 'sad', label: 'diary.mood.sad', icon: FaceSlightlyFrowning, color: '#4aa8ff' },
  { id: 'angry', label: 'diary.mood.angry', icon: FaceAngry, color: '#ef5d6c' },
];

export const moodOf = (id: string) => MOODS.find((m) => m.id === id);

export const LAYOUTS: { id: DiaryLayout; label: Key }[] = [
  { id: 'classic', label: 'diary.layout.classic' },
  { id: 'photo', label: 'diary.layout.photo' },
  { id: 'simple', label: 'diary.layout.simple' },
];

export const PAPERS: { id: DiaryPaper; label: Key }[] = [
  { id: 'lined', label: 'diary.paper.lined' },
  { id: 'grid', label: 'diary.paper.grid' },
  { id: 'dots', label: 'diary.paper.dots' },
  { id: 'plain', label: 'diary.paper.plain' },
];

/** Highlighter pens: soft colors that keep dark text readable in both themes. */
export const HIGHLIGHTS: { color: string; label: Key }[] = [
  { color: '#fde68a', label: 'diary.hl.yellow' },
  { color: '#fbcfe8', label: 'diary.hl.pink' },
  { color: '#bbf7d0', label: 'diary.hl.green' },
  { color: '#bfdbfe', label: 'diary.hl.blue' },
];

export function blankEntry(date: DateStr, like?: DiaryEntry | null): DiaryEntry {
  // A new page keeps the layout and paper of the last one, so a chosen style sticks.
  return {
    date, mood: '', layout: like?.layout ?? 'classic', paper: like?.paper ?? 'lined', line: '', body: '', photo: null,
    stickers: [], updated_at: 0,
  };
}

/** Plain text of the writing, for previews. */
export function textOf(html: string): string {
  if (!html) return '';
  const doc = new DOMParser().parseFromString(html, 'text/html');
  return (doc.body.textContent ?? '').replace(/\s+/g, ' ').trim();
}

/** Nothing written or chosen (such a page is not kept). */
export const isBlank = (e: DiaryEntry) => !e.mood && !e.line.trim() && !textOf(e.body) && !e.photo && !e.stickers.length;
