import { TOOLS } from '../lib/registry';
import {
  ALL_LOCALES,
  homeAlternates,
  localizedPath,
  localizedToolPath,
  toolAlternates,
} from '../lib/locales';

const SITE = 'https://unfleece.com';

function absolute(path: string): string {
  return `${SITE}${path}`;
}

/** A <url> entry with xhtml:link hreflang alternates (no lastmod — we don't fake it). */
function urlEntry(path: string, alternates: { lang: string; path: string }[]): string {
  const alts = alternates
    .map((alt) => `    <xhtml:link rel="alternate" hreflang="${alt.lang}" href="${absolute(alt.path)}" />`)
    .join('\n');
  return `  <url>
    <loc>${absolute(path)}</loc>
${alts}
  </url>`;
}

export function GET() {
  // Home pages: one entry per locale, each carrying the full reciprocal hreflang set.
  const homeUrls = ALL_LOCALES.map((locale) => urlEntry(localizedPath(locale, '/'), homeAlternates()));

  // Tool pages: every tool in every locale (all are built and fully translated).
  const toolUrls = TOOLS.flatMap((tool) =>
    ALL_LOCALES.map((locale) => urlEntry(localizedToolPath(locale, tool.slug), toolAlternates(tool.slug))),
  );

  // About is English-only for now (no localized variant), so no hreflang alternates.
  const aboutUrl = `  <url><loc>${absolute('/about')}</loc></url>`;

  const body = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">
${homeUrls.join('\n')}
${aboutUrl}
${toolUrls.join('\n')}
</urlset>
`;
  return new Response(body, { headers: { 'Content-Type': 'application/xml' } });
}
