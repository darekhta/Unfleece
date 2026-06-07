# 09 — Branding

## Name: **Unfleece**

A friendly coinage on "fleece" (to fleece = to swindle). **Un**-fleecy = the un-swindling
PDF tool — free, no trap, no surprise weekly bill. Approachable sibling of the blunter
"Unfleece"; the softer form reads as a real, warm brand rather than a pure jab.

**Why it won** (from a two-round brainstorm + judge panel + availability sweep):
- On-mission: literally names the anti-fleeceware stance.
- Distinctive + low trademark risk; unique term → easy SEO.
- **Full domain availability** (rare): every relevant TLD was free.

## Domains

| Domain | Status | Use |
|---|---|---|
| **`unfleece.com`** | **purchased + LIVE** (Cloudflare Registrar) | **primary brand domain** — canonical URLs, sitemap, OG all point here |
| `www.unfleece.com` | live | attached to the same Pages project |
| `unfleece.pages.dev` | live | deploy alias ($0 forever) |
| `unfleece.app` / `.io` / `.dev` | still available | optional defensive grabs |
| `unfleece.js.org` | claim later | free OSS subdomain (PR once repo is public) |

## Tagline

Primary: **"Free, private PDF tools — your files never leave your browser."**

Alternates:
- "PDF tools without the fleece."
- "Every PDF tool. Free. Private. In your browser."
- "The PDF toolkit the subscription traps don't want you to have." (launch/HN energy)

The name doesn't state the category, so the tagline always carries "PDF tools" + the
"never leaves your browser" wedge (the Notion/Linear/Vercel pattern).

## Voice & tone
Confident, warm, a little cheeky — **for** the user, mildly **against** the
subscription-trap status quo, never smug. Plain language. We state limits honestly instead
of overpromising (the honesty *is* the brand). See `docs/00-vision.md`.

## Visual identity (as shipped)
- **"Quiet, engineered, reassuring — one light, one accent":** light-first design system
  (dark theme via persisted toggle), emerald `#2ECC8F` accent ramp, cool surface
  elevation ladder, restrained glass cards.
- **Typography:** Inter (UI) + JetBrains Mono (numbers/meta).
- **Bento-grid** tool launcher — one card per tool (also internal SEO links).
- **Logo:** the cheerful emerald **un-sheared sheep** mark (transparent PNG, theme-adaptive)
  — the file you *keep*. Used as favicon, header mark and OG image.
- **Icons:** custom 24 px monoline set (`src/lib/icons.ts`), one visual voice across all
  37 tools.
- Effects kept light; Core Web Vitals stay green.

## Naming conventions (code)
- Repo / app: `unfleece` (GitHub: `darekhta/Unfleece`)
- Permissive Rust core crate: `unfleece-core` (MIT/Apache) — shipped
- AGPL engine-wrap crate: `unfleece-wrap` — future, created with the first AGPL engine
- npm scope (if published): `@unfleece/*`
