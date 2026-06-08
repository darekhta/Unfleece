// One self-contained translation bundle per locale. Adding a language = adding
// one file that satisfies this interface; the registry (../locales.ts) wires it
// up and falls back to English for any bundle that is missing or partial.
import type { Category } from '../registry.js';

export type Dir = 'ltr' | 'rtl';

export interface LocaleMeta {
  /** BCP-47 base code, also the URL path segment (en, es, zh, ar, …). */
  code: string;
  /** Value for <html lang>. */
  htmlLang: string;
  /** The language's own name in its own script — shown in the switcher. */
  autonym: string;
  /** English name — for aria-labels / tooltips. */
  englishName: string;
  /** Text direction; 'rtl' for Arabic, Hebrew, … */
  dir: Dir;
}

export interface CategoryText {
  label: string;
  descriptor: string;
}

export interface LayoutText {
  skip: string;
  homeAria: string;
  navAria: string;
  allTools: string;
  howItWorks: string;
  privacy: string;
  footerBlurb: string;
  tools: string;
  project: string;
  trust: string;
  github: string;
  license: string;
  reportIssue: string;
  noUpload: string;
  noAccount: string;
  noTracking: string;
  /** Accessible label for the language switcher button/menu. */
  language: string;
  /** "Open" affordance on tool cards (optional; falls back to English). */
  open?: string;
}

export interface ToolPageText {
  breadcrumbHome: string;
  breadcrumbTools: string;
  privateWorkspace: string;
  nothingUploaded: string;
  howEyebrow: string;
  howHeading: (toolName: string) => string;
  steps: { title: string; body: string }[];
  differenceEyebrow: string;
  differenceHeading: string;
  usBullets: string[];
  themHeading: string;
  themBullets: string[];
  questionsEyebrow: string;
  questionsHeading: string;
  relatedEyebrow: string;
  relatedHeading: string;
  faqs: (toolName: string) => { q: string; a: string }[];
}

export interface HomeText {
  heroLine1: string;
  heroLine2: string;
  subhead: string;
  browseCta: string;
  howCta: string;
  proof: { title: string; body: string; linkLabel?: string }[];
  toolsEyebrow: string;
  pickHeading: string;
  pickBody: string;
  proveEyebrow: string;
  proveHeading: string;
  proveBody: string;
  /** Each numbered step: a bold lead + the rest of the sentence. */
  proveSteps: { strong: string; rest: string }[];
}

/** Strings for the interactive tool runner island (ToolRunner + Dropzone).
 * Plain strings; `{tool}`/`{n}`/`{kind}` placeholders are filled in the component. */
export interface RunnerText {
  stepAdd: string;
  stepOptions: string;
  stepRun: string;
  stepDownload: string;
  working: string;
  cancel: string;
  startOver: string;
  addFileFirst: string;
  fixOptions: string;
  progressFallback: string;
  setupErrorTitle: string;
  setupErrorBody: string; // "{tool} needs its dedicated editor."
  largeTitle: string;
  largeBody: string;
  doneOne: string;
  doneMany: string;
  reviewText: string;
  extractedText: string; // "Extracted text · {n} characters"
  copy: string;
  madeHere: string;
  // Dropzone
  kindImage: string;
  kindFile: string;
  dropOne: string; // "Drop a {kind} here, or click to choose"
  dropMany: string; // "Drop {kind}s here, or click to choose"
  dzTypesOne: string; // "{kind} · one file"
  dzTypesMany: string; // "{kind} · single or multiple"
  dzPrivacy: string;
  dzAria: string;
  addedFiles: string;
  rejectedOne: string;
  rejectedMany: string; // "{n} files do not match this tool."
  rejectedSingle: string;
}

export interface ToolText {
  name: string;
  tagline: string;
  description: string;
}

/** id → translated tool text. Missing ids fall back to the English registry. */
export type ToolTextMap = Record<string, ToolText>;

export interface LocaleBundle {
  meta: LocaleMeta;
  categories: Record<Category, CategoryText>;
  layout: LayoutText;
  toolPage: ToolPageText;
  home: HomeText;
  /** Interactive runner island strings (optional; falls back to English). */
  runner?: RunnerText;
  /** Empty for English (the registry already holds the source strings). */
  tools: ToolTextMap;
}
