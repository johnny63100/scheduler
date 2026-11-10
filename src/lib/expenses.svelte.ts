// Money book categories (spending and income) and currency formatting.
// The built-in categories can be renamed, re-iconed, recolored or hidden; the user can add their own.
// The list is stored in the setting `expense.categories` (see `saveCategories`).
import type { Component } from 'svelte';
import {
  Baby, Beer, BookOpen, Briefcase, Brush, Bus, Cake, Car, Cat, CircleEllipsis, Clapperboard, Coffee, Coins, Dices, Dog, Dumbbell, Fuel,
  GraduationCap, HeartHandshake, HeartPulse, House, Gamepad2, Gift, HandCoins, Undo2, Landmark, Laptop, Music, Palette, PiggyBank, Pill, Pizza, Plane,
  Presentation, Receipt, Scissors, Shirt, ShoppingBag, Smartphone, Sparkles, Store, Ticket, Tv, Utensils, Wifi, Wrench,
} from '@lucide/svelte';
import { api, type ExpenseKind } from './api';
import { intlLocale, i18n, t, type Key } from './i18n.svelte';

type IconComp = Component<{ size?: number }>;

/** Icons a category can use; `emoji` is what the phone page shows. */
export const CATEGORY_ICONS: Record<string, { icon: IconComp; emoji: string }> = {
  utensils: { icon: Utensils, emoji: '🍚' },
  coffee: { icon: Coffee, emoji: '☕' },
  pizza: { icon: Pizza, emoji: '🍕' },
  cake: { icon: Cake, emoji: '🍰' },
  beer: { icon: Beer, emoji: '🍺' },
  bus: { icon: Bus, emoji: '🚌' },
  car: { icon: Car, emoji: '🚗' },
  fuel: { icon: Fuel, emoji: '⛽' },
  plane: { icon: Plane, emoji: '✈️' },
  bag: { icon: ShoppingBag, emoji: '🛍️' },
  shirt: { icon: Shirt, emoji: '👕' },
  scissors: { icon: Scissors, emoji: '💇' },
  house: { icon: House, emoji: '🏠' },
  receipt: { icon: Receipt, emoji: '🧾' },
  phone: { icon: Smartphone, emoji: '📱' },
  wifi: { icon: Wifi, emoji: '📶' },
  tv: { icon: Tv, emoji: '📺' },
  health: { icon: HeartPulse, emoji: '💊' },
  pill: { icon: Pill, emoji: '💉' },
  dumbbell: { icon: Dumbbell, emoji: '🏋️' },
  game: { icon: Gamepad2, emoji: '🎮' },
  dice: { icon: Dices, emoji: '🎲' },
  music: { icon: Music, emoji: '🎵' },
  film: { icon: Clapperboard, emoji: '🎬' },
  ticket: { icon: Ticket, emoji: '🎟️' },
  book: { icon: BookOpen, emoji: '📖' },
  school: { icon: GraduationCap, emoji: '📚' },
  palette: { icon: Palette, emoji: '🎨' },
  gift: { icon: Gift, emoji: '🎁' },
  cat: { icon: Cat, emoji: '🐱' },
  dog: { icon: Dog, emoji: '🐶' },
  baby: { icon: Baby, emoji: '🍼' },
  wrench: { icon: Wrench, emoji: '🔧' },
  sparkles: { icon: Sparkles, emoji: '✨' },
  piggy: { icon: PiggyBank, emoji: '🐷' },
  briefcase: { icon: Briefcase, emoji: '💼' },
  laptop: { icon: Laptop, emoji: '💻' },
  brush: { icon: Brush, emoji: '🖌️' },
  lesson: { icon: Presentation, emoji: '🧑‍🏫' },
  store: { icon: Store, emoji: '🏪' },
  support: { icon: HeartHandshake, emoji: '💜' },
  coins: { icon: Coins, emoji: '🪙' },
  bank: { icon: Landmark, emoji: '🏦' },
  handcoins: { icon: HandCoins, emoji: '💸' },
  undo: { icon: Undo2, emoji: '↩️' },
  more: { icon: CircleEllipsis, emoji: '📎' },
};

/** Colors offered for categories. */
export const CATEGORY_COLORS = [
  '#ff8a5c', '#b0773f', '#f5b83d', '#34c38f', '#22b5bf', '#4aa8ff', '#7c74ff', '#b164e8', '#f2668b', '#ef5d6c', '#7d8597', '#94a3b8',
];

/** A stored category. Built-ins have `label` (translated name) until renamed; custom ones have `custom`. */
export type CategoryDef = {
  id: string;
  kind: ExpenseKind;
  label?: Key;
  name?: string;
  icon: string;
  color: string;
  hidden?: boolean;
  /** Left out of totals, charts and budgets (e.g. paying for friends and being paid back). */
  excluded?: boolean;
  custom?: boolean;
};

const BUILT_IN: CategoryDef[] = [
  { id: 'food', kind: 'expense', label: 'ex.cat.food', icon: 'utensils', color: '#ff8a5c' },
  { id: 'cafe', kind: 'expense', label: 'ex.cat.cafe', icon: 'coffee', color: '#b0773f' },
  { id: 'transport', kind: 'expense', label: 'ex.cat.transport', icon: 'bus', color: '#4aa8ff' },
  { id: 'shopping', kind: 'expense', label: 'ex.cat.shopping', icon: 'bag', color: '#f2668b' },
  { id: 'housing', kind: 'expense', label: 'ex.cat.housing', icon: 'house', color: '#7c74ff' },
  { id: 'bills', kind: 'expense', label: 'ex.cat.bills', icon: 'receipt', color: '#7d8597' },
  { id: 'health', kind: 'expense', label: 'ex.cat.health', icon: 'health', color: '#ef5d6c' },
  { id: 'fun', kind: 'expense', label: 'ex.cat.fun', icon: 'game', color: '#b164e8' },
  { id: 'education', kind: 'expense', label: 'ex.cat.education', icon: 'school', color: '#22b5bf' },
  { id: 'travel', kind: 'expense', label: 'ex.cat.travel', icon: 'plane', color: '#34c38f' },
  { id: 'gifts', kind: 'expense', label: 'ex.cat.gifts', icon: 'gift', color: '#f5b83d' },
  { id: 'settle', kind: 'expense', label: 'ex.cat.settle', icon: 'handcoins', color: '#94a3b8', excluded: true },
  { id: 'other', kind: 'expense', label: 'ex.cat.other', icon: 'more', color: '#94a3b8' },
  // Where income comes from; broad enough for jobs, side work and creative income alike.
  { id: 'salary', kind: 'income', label: 'ex.in.salary', icon: 'briefcase', color: '#4aa8ff' },
  { id: 'sidejob', kind: 'income', label: 'ex.in.sidejob', icon: 'laptop', color: '#7c74ff' },
  { id: 'commission', kind: 'income', label: 'ex.in.commission', icon: 'brush', color: '#f2668b' },
  { id: 'lesson', kind: 'income', label: 'ex.in.lesson', icon: 'lesson', color: '#22b5bf' },
  { id: 'sales', kind: 'income', label: 'ex.in.sales', icon: 'store', color: '#ff8a5c' },
  { id: 'creator', kind: 'income', label: 'ex.in.creator', icon: 'support', color: '#b164e8' },
  { id: 'allowance', kind: 'income', label: 'ex.in.allowance', icon: 'coins', color: '#f5b83d' },
  { id: 'investment', kind: 'income', label: 'ex.in.investment', icon: 'bank', color: '#34c38f' },
  { id: 'settle_income', kind: 'income', label: 'ex.in.settle', icon: 'undo', color: '#94a3b8', excluded: true },
  { id: 'other_income', kind: 'income', label: 'ex.in.other', icon: 'more', color: '#94a3b8' },
];

/** Categories every list falls back to; they can be renamed but not hidden. */
export const FALLBACK = { expense: 'other', income: 'other_income' } as const;

export const categoryStore = $state({ list: BUILT_IN.map((c) => ({ ...c })) as CategoryDef[] });

/** A category ready to show. */
export type Category = {
  id: string;
  kind: ExpenseKind;
  name: string;
  icon: IconComp;
  iconKey: string;
  color: string;
  hidden: boolean;
  excluded: boolean;
  custom: boolean;
};

export function resolveCategory(c: CategoryDef): Category {
  const icon = CATEGORY_ICONS[c.icon] ?? CATEGORY_ICONS.more;
  return {
    id: c.id,
    kind: c.kind,
    name: c.name?.trim() || (c.label ? t(c.label) : c.id),
    icon: icon.icon,
    iconKey: c.icon in CATEGORY_ICONS ? c.icon : 'more',
    color: c.color,
    hidden: !!c.hidden,
    excluded: !!c.excluded,
    custom: !!c.custom,
  };
}

/** Categories of one kind in order; hidden ones only when asked (e.g. for old records). */
export function categoriesFor(kind: ExpenseKind, includeHidden = false): Category[] {
  return categoryStore.list.filter((c) => c.kind === kind && (includeHidden || !c.hidden)).map(resolveCategory);
}

export const categoryOf = (id: string): Category => {
  const found = categoryStore.list.find((c) => c.id === id);
  if (found) return resolveCategory(found);
  const fallback = categoryStore.list.find((c) => c.id === FALLBACK.expense) ?? BUILT_IN.find((c) => c.id === FALLBACK.expense)!;
  return resolveCategory(fallback);
};

/** Whether entries in this category count in totals, charts and budgets. */
export const countsInStats = (categoryId: string) => !categoryStore.list.find((c) => c.id === categoryId)?.excluded;

/** Saved list merged with the built-ins (new built-ins in a later version still appear). */
function mergeCategories(raw: string | null): CategoryDef[] {
  let saved: CategoryDef[] = [];
  try {
    const parsed = JSON.parse(raw || '[]');
    if (Array.isArray(parsed)) saved = parsed.filter((c) => c && typeof c.id === 'string' && (c.kind === 'expense' || c.kind === 'income'));
  } catch {
    saved = [];
  }
  if (!saved.length) return BUILT_IN.map((c) => ({ ...c }));
  const out: CategoryDef[] = saved.map((c) => {
    const base = BUILT_IN.find((b) => b.id === c.id);
    return base
      ? { ...base, name: c.name || undefined, icon: c.icon || base.icon, color: c.color || base.color, hidden: !!c.hidden, excluded: c.excluded ?? base.excluded }
      : { id: c.id, kind: c.kind, name: c.name || c.id, icon: c.icon || 'more', color: c.color || '#94a3b8', hidden: !!c.hidden, excluded: !!c.excluded, custom: true };
  });
  // New built-ins from a later version go just before that kind's fallback ("Other").
  for (const b of BUILT_IN) {
    if (out.some((c) => c.id === b.id)) continue;
    const at = out.findIndex((c) => c.id === FALLBACK[b.kind]);
    out.splice(at < 0 ? out.length : at, 0, { ...b });
  }
  return out;
}

export async function saveCategories(list: CategoryDef[]) {
  categoryStore.list = list.map((c) => ({ ...c, hidden: Object.values(FALLBACK).includes(c.id as never) ? false : c.hidden }));
  const stored = categoryStore.list.map(({ id, kind, name, icon, color, hidden, excluded, custom }) => ({ id, kind, name, icon, color, hidden, excluded: !!excluded, custom }));
  await api.setSetting('expense.categories', JSON.stringify(stored)).catch(() => {});
}

/** A new id for a category the user adds. */
export const newCategoryId = () => `c${Date.now().toString(36)}`;

export const CURRENCIES = ['KRW', 'USD', 'EUR', 'JPY', 'GBP', 'CNY', 'CAD', 'AUD'] as const;

/** Currencies normally written without decimals. */
const NO_DECIMALS = new Set(['KRW', 'JPY']);

export const expenseSettings = $state({
  currency: '' as string,
  budget: null as number | null,
  /** Show payments from the commission manager (after fees) as income. */
  linkCommissions: true,
  /** Monthly spending target per category id (the same every month). */
  categoryBudgets: {} as Record<string, number>,
  /** Payments due every month (see src-tauri/src/fixed.rs). */
  fixed: [] as FixedExpense[],
  /** month "YYYY-MM" -> fixed id -> id of the recorded expense */
  fixedPaid: {} as Record<string, Record<string, number>>,
  /** month "YYYY-MM" -> ids of fixed expenses skipped that month (not due, not counted) */
  fixedSkipped: {} as Record<string, string[]>,
  /** How the fixed list is ordered: by category (then the dragged order) or by due day. */
  fixedSort: 'category' as 'category' | 'date',
  loaded: false,
});

export async function loadExpenseSettings() {
  const [currency, budget, link, perCategory, cats] = await Promise.all([
    api.getSetting('expense.currency').catch(() => null),
    api.getSetting('expense.budget').catch(() => null),
    api.getSetting('expense.linkCommissions').catch(() => null),
    api.getSetting('expense.categoryBudgets').catch(() => null),
    api.getSetting('expense.categories').catch(() => null),
  ]);
  await loadFixed();
  categoryStore.list = mergeCategories(cats);
  expenseSettings.categoryBudgets = parseBudgets(perCategory);
  expenseSettings.linkCommissions = link !== 'false';
  expenseSettings.currency = currency ?? ({ ko: 'KRW', ja: 'JPY', en: 'USD' } as const)[i18n.locale];
  expenseSettings.budget = budget && Number(budget) > 0 ? Number(budget) : null;
  expenseSettings.loaded = true;
}

export async function saveExpenseSettings(currency: string, budget: number | null, linkCommissions = expenseSettings.linkCommissions) {
  expenseSettings.currency = currency;
  expenseSettings.budget = budget && budget > 0 ? budget : null;
  expenseSettings.linkCommissions = linkCommissions;
  await api.setSetting('expense.linkCommissions', String(linkCommissions)).catch(() => {});
  await api.setSetting('expense.currency', currency).catch(() => {});
  await api.setSetting('expense.budget', expenseSettings.budget ? String(expenseSettings.budget) : '').catch(() => {});
}

function parseBudgets(raw: string | null): Record<string, number> {
  try {
    const obj = JSON.parse(raw || '{}') as Record<string, unknown>;
    return Object.fromEntries(Object.entries(obj).filter(([, v]) => typeof v === 'number' && v > 0)) as Record<string, number>;
  } catch {
    return {};
  }
}

/** A payment due every month, or once a year when `month` (1–12) is set. */
export type FixedExpense = { id: string; name: string; amount: number; category: string; day: number; month?: number };

/** Whether a fixed expense is due in `month` ("YYYY-MM-DD" or "YYYY-MM"). */
export const fixedDueIn = (f: FixedExpense, month: string) => !f.month || f.month === Number(month.slice(5, 7));

function parseJson<T>(raw: string | null, fallback: T): T {
  try {
    return raw ? (JSON.parse(raw) as T) : fallback;
  } catch {
    return fallback;
  }
}

/** Reads the fixed list and paid marks (the paid marks also change when ticking from the phone). */
export async function loadFixed() {
  const [list, paid, sort, skipped] = await Promise.all([
    api.getSetting('expense.fixed').catch(() => null),
    api.getSetting('expense.fixedPaid').catch(() => null),
    api.getSetting('expense.fixedSort').catch(() => null),
    api.getSetting('expense.fixedSkipped').catch(() => null),
  ]);
  expenseSettings.fixedSkipped = parseJson<Record<string, string[]>>(skipped, {});
  expenseSettings.fixedSort = sort === 'date' ? 'date' : 'category';
  const items = parseJson<FixedExpense[]>(list, []);
  expenseSettings.fixed = Array.isArray(items) ? items : [];
  expenseSettings.fixedPaid = parseJson<Record<string, Record<string, number>>>(paid, {});
}

/** Skips (or un-skips) a fixed expense for one month ("YYYY-MM"). */
export async function setFixedSkipped(id: string, month: string, skip: boolean) {
  const all = { ...expenseSettings.fixedSkipped };
  const list = new Set(all[month] ?? []);
  if (skip) list.add(id);
  else list.delete(id);
  if (list.size) all[month] = [...list];
  else delete all[month];
  expenseSettings.fixedSkipped = all;
  await api.setSetting('expense.fixedSkipped', JSON.stringify(all)).catch(() => {});
}

export async function saveFixedSort(sort: 'category' | 'date') {
  expenseSettings.fixedSort = sort;
  await api.setSetting('expense.fixedSort', sort).catch(() => {});
}

export async function saveFixed(list: FixedExpense[]) {
  expenseSettings.fixed = list;
  await api.setSetting('expense.fixed', JSON.stringify(list)).catch(() => {});
}

export const newFixedId = () => `f${Date.now().toString(36)}`;

export async function saveCategoryBudgets(budgets: Record<string, number>) {
  expenseSettings.categoryBudgets = Object.fromEntries(Object.entries(budgets).filter(([, v]) => v > 0));
  await api.setSetting('expense.categoryBudgets', JSON.stringify(expenseSettings.categoryBudgets)).catch(() => {});
}

/** Sum of the category targets, if any are set. */
export const categoryBudgetTotal = () => Object.values(expenseSettings.categoryBudgets).reduce((s, v) => s + v, 0);

/** The month's spending budget: the category targets added up, or the single monthly budget. */
export const monthlyBudget = () => categoryBudgetTotal() || expenseSettings.budget;

export function fmtMoney(amount: number, currency = expenseSettings.currency || 'USD'): string {
  const digits = NO_DECIMALS.has(currency) ? 0 : 2;
  return new Intl.NumberFormat(intlLocale(), {
    style: 'currency',
    currency,
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  }).format(amount);
}

/** Short form for chart labels, e.g. "₩12k". */
export function fmtMoneyShort(amount: number, currency = expenseSettings.currency || 'USD'): string {
  return new Intl.NumberFormat(intlLocale(), { style: 'currency', currency, notation: 'compact', maximumFractionDigits: 1 }).format(amount);
}

export const currencySymbol = (currency = expenseSettings.currency || 'USD') =>
  new Intl.NumberFormat(intlLocale(), { style: 'currency', currency }).formatToParts(0).find((p) => p.type === 'currency')?.value ?? currency;
