import { TOOLS } from '../lib/registry';
import { localizedToolPath, toolAlternates, type Locale } from '../lib/locales';

const SITE = 'https://unfleece.com';
const staticPaths = ['/', '/about'];

function absolute(path: string): string {
  return `${SITE}${path}`;
}

function toolUrl(slug: string, locale: Locale): string {
  const path = localizedToolPath(locale, slug);
  const alternates = toolAlternates(slug)
    .map((alt) => `    <xhtml:link rel="alternate" hreflang="${alt.lang}" href="${absolute(alt.path)}" />`)
    .join('\n');
  return `  <url>
    <loc>${absolute(path)}</loc>
${alternates}
  </url>`;
}

export function GET() {
  const body = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">
${staticPaths.map((p) => `  <url><loc>${absolute(p)}</loc></url>`).join('\n')}
${TOOLS.flatMap((t) => [
  toolUrl(t.slug, 'en'),
  toolUrl(t.slug, 'uk'),
  toolUrl(t.slug, 'it'),
]).join('\n')}
</urlset>
`;
  return new Response(body, { headers: { 'Content-Type': 'application/xml' } });
}
