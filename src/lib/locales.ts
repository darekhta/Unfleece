// Localization registry. Each language is a self-contained bundle under
// ./i18n/<code>.ts; this file wires them together, derives the legacy
// per-namespace maps the pages still import, and provides path/detection
// helpers. Missing or partial bundles fall back to English automatically.
import type { Category, Tool } from './registry.js';
import type {
  CategoryText,
  Dir,
  HomeText,
  LayoutText,
  LocaleBundle,
  LocaleMeta,
  ToolPageText,
  ToolText,
} from './i18n/types.js';

import { en } from './i18n/en.js';
import { zh } from './i18n/zh.js';
import { hi } from './i18n/hi.js';
import { es } from './i18n/es.js';
import { fr } from './i18n/fr.js';
import { ar } from './i18n/ar.js';
import { pt } from './i18n/pt.js';
import { ru } from './i18n/ru.js';
import { de } from './i18n/de.js';
import { ja } from './i18n/ja.js';
import { it } from './i18n/it.js';
import { uk } from './i18n/uk.js';

export type { LayoutText, ToolPageText, HomeText, CategoryText, LocaleMeta, ToolText, Dir };

export type Locale = 'en' | 'zh' | 'hi' | 'es' | 'fr' | 'ar' | 'pt' | 'ru' | 'de' | 'ja' | 'it' | 'uk';
export type NonDefaultLocale = Exclude<Locale, 'en'>;

export const DEFAULT_LOCALE: Locale = 'en';

/** Source of truth: ordered roughly by global reach, English first. */
const BUNDLES: Record<Locale, LocaleBundle> = { en, zh, hi, es, fr, ar, pt, ru, de, ja, it, uk };

export const ALL_LOCALES: Locale[] = Object.keys(BUNDLES) as Locale[];
export const NON_DEFAULT_LOCALES: NonDefaultLocale[] = ALL_LOCALES.filter((l) => l !== 'en') as NonDefaultLocale[];

/** URL prefix for a locale ('' for the default English at the root). */
export function pathPrefix(locale: Locale): string {
  return locale === DEFAULT_LOCALE ? '' : `/${locale}`;
}

export const LOCALES: Record<Locale, LocaleMeta & { pathPrefix: string; label: string }> = Object.fromEntries(
  ALL_LOCALES.map((l) => [l, { ...BUNDLES[l].meta, pathPrefix: pathPrefix(l), label: BUNDLES[l].meta.autonym }]),
) as Record<Locale, LocaleMeta & { pathPrefix: string; label: string }>;

export function bundle(locale: Locale): LocaleBundle {
  return BUNDLES[locale] ?? en;
}

// ---------------------------------------------------------------------------
// Legacy per-namespace maps (pages import these directly).
// ---------------------------------------------------------------------------
export const CATEGORY_TEXT = Object.fromEntries(
  ALL_LOCALES.map((l) => [l, BUNDLES[l].categories]),
) as Record<Locale, Record<Category, CategoryText>>;

export const LAYOUT_TEXT = Object.fromEntries(
  ALL_LOCALES.map((l) => [l, BUNDLES[l].layout]),
) as Record<Locale, LayoutText>;

export const TOOL_PAGE_TEXT = Object.fromEntries(
  ALL_LOCALES.map((l) => [l, BUNDLES[l].toolPage]),
) as Record<Locale, ToolPageText>;

export const HOME_TEXT = Object.fromEntries(
  ALL_LOCALES.map((l) => [l, BUNDLES[l].home]),
) as Record<Locale, HomeText>;

/** Apply a locale's tool name/tagline/description, falling back to English. */
export function localizeTool(tool: Tool, locale: Locale): Tool {
  if (locale === DEFAULT_LOCALE) return tool;
  const text = BUNDLES[locale].tools[tool.id];
  return text ? { ...tool, ...text } : tool;
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------
export function localizedToolPath(locale: Locale, slug: string): string {
  return `${pathPrefix(locale)}/tools/${slug}`;
}

/** Localized path for the home page (or any root-relative page). */
export function localizedPath(locale: Locale, path = '/'): string {
  const clean = path.startsWith('/') ? path : `/${path}`;
  if (clean === '/') return pathPrefix(locale) || '/';
  return `${pathPrefix(locale)}${clean}`;
}

/** hreflang alternates for a tool page (all locales + x-default → English). */
export function toolAlternates(slug: string): { lang: string; path: string }[] {
  return [
    ...ALL_LOCALES.map((locale) => ({ lang: BUNDLES[locale].meta.htmlLang, path: localizedToolPath(locale, slug) })),
    { lang: 'x-default', path: localizedToolPath(DEFAULT_LOCALE, slug) },
  ];
}

/** hreflang alternates for the home page. */
export function homeAlternates(): { lang: string; path: string }[] {
  return [
    ...ALL_LOCALES.map((locale) => ({ lang: BUNDLES[locale].meta.htmlLang, path: localizedPath(locale, '/') })),
    { lang: 'x-default', path: localizedPath(DEFAULT_LOCALE, '/') },
  ];
}

/** Strip a leading locale segment from a path, returning the locale + remainder. */
export function splitLocale(path: string): { locale: Locale; rest: string } {
  const m = path.match(/^\/([a-z]{2})(?:\/|$)/);
  if (m && ALL_LOCALES.includes(m[1] as Locale) && m[1] !== 'en') {
    const locale = m[1] as Locale;
    const rest = path.slice(`/${locale}`.length) || '/';
    return { locale, rest };
  }
  return { locale: 'en', rest: path || '/' };
}

/** Given the current path, return the equivalent path in `target` (for the switcher). */
export function switchLocalePath(currentPath: string, target: Locale): string {
  const { rest } = splitLocale(currentPath);
  return localizedPath(target, rest);
}

/** Best supported locale for an ordered list of BCP-47 tags (server-side helper). */
export function matchLocale(preferred: readonly string[]): Locale | null {
  for (const tag of preferred) {
    const base = tag.toLowerCase().split('-')[0];
    const hit = ALL_LOCALES.find((l) => l === base);
    if (hit) return hit;
  }
  return null;
}
