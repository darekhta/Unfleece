# 07 — Go-to-Market & SEO

> **Status: playbook.** The product is finished and live at <https://unfleece.com>
> (canonical URLs, localized hreflang sitemap, robots, FAQ schema all point there).
> The launch spikes and the SEO grind below are the *next* phase — they start when the
> repo goes public.

## The honest truth up front
This category has **no viral loop**. Incumbents (Smallpdf ~33M visits/mo, DA 82; iLovePDF;
Sejda ~19M; Canva's templated pages 38M+ organic) win on **years of tool-per-URL structure
+ localization + domain authority**. SEO here is a **6–18 month grind, not a launch
tactic.** Plan accordingly: launch spikes get the first users and backlinks; SEO is the
durable engine that compounds slowly.

## Growth engine: programmatic SEO (tool-per-URL)
- **One indexable static page per tool and locale** (`/tools/merge-pdf`,
  `/uk/tools/merge-pdf`, `/it/tools/merge-pdf`, …) — exactly how incumbents rank. Astro
  makes each a static route.
- **Clear Google's 2025 thin-content bar:** every page needs 500+ genuinely unique words,
  a working tool (unique utility), how-to content, and FAQ schema. No boilerplate clones.
- First localization wave shipped: English, Ukrainian and Italian tool pages with
  canonical/hreflang alternates and a language switcher.
- Internal linking via the bento-grid homepage (each card → a tool page).
- Mind Cloudflare's 20,000-file limit when multiplying tools × locales (`docs/01`).

## Don't fight head terms first
- The highest-volume term, "pdf to word", is structurally a **server's** game on fidelity
  — we'll lose it and that's fine (`docs/02-features.md`). Don't anchor on it.
- Target **long-tail / intent terms** we can actually win and that match our wedge:
  - "merge pdf **without uploading**", "compress pdf **offline / in browser**",
    "**private** pdf editor", "edit pdf **no upload**", "free pdf tools **no account**".
  - Locale-specific variants.

## Launch playbook (the spikes)
- **Show HN** — lead with the architecture story: "Unfleece — free PDF tools that run
  100% in your browser; watch the Network tab, nothing uploads. Open source."
- **Reddit** — r/privacy, r/selfhosted, r/opensource, r/pdf, r/degoogle.
- **Product Hunt** — the privacy + anti-fleeceware angle + premium UI.
- **Ride privacy controversies** — there's recurring news about PDF sites' tracking/upload
  practices; a well-timed "here's a tool that can't do that" post converts.

## Differentiation that converts (once discovered)
The niche is already contested (BentoPDF, RaptorPDF, HonestPDF, SimplePDF, EmbedPDF,
Stirling-PDF at 80k stars but server/self-hosted). Our wedges:
1. **Verifiable privacy** — open-source + "watch the Network tab"; a cleaner GDPR/HIPAA
   posture than Smallpdf/iLovePDF, which upload files.
2. **No fleeceware** — unlimited, free, no account, no file-size paywall.
3. **Premium UX for free** — native-quality PDFium previews + glass UI matching paid tools.

## Trust signals to build in
- Prominent, honest "Files never leave your device — [how it works]" with a link to the
  open-source repo and a literal "open your Network tab" invitation.
- No account wall, no email capture before use.
- Clear, non-deceptive labels everywhere (the anti-fleeceware brand depends on it).

## Measurement (content-blind)
Analytics must never capture document data. Prefer privacy-respecting, aggregate-only
metrics (e.g. Cloudflare Web Analytics) — using a creepy tracker would undercut the entire
pitch.
