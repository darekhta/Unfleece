# 00 — Vision & Positioning

> **Status: shipped.** Unfleece is live at <https://unfleece.com> — 22 client-side
> tools, $0 infrastructure, AGPL-3.0. This document records the positioning the product
> was built to; everything below is in force, not aspiration.

## The problem

A whole genre of "PDF utility" websites runs on two practices worth opposing:

- **Fleeceware billing.** You search "merge pdf", land on a polished site, do the task,
  then hit a paywall: a $0.99–$1.95 "trial" that silently converts to **~$39–$59 per
  week or month**. Cancellation is buried. The tool's actual value is near-zero; the
  business *is* the dark-pattern funnel. (pdfSimpli is the archetype; FormSwift, Soda
  PDF, PDF Pro and others share the pattern.)
- **Mandatory upload.** Even the honest freemium players (iLovePDF, Smallpdf) upload
  your document to their servers to process it — then throttle the free tier (Smallpdf
  ~2 tasks/day; iLovePDF ~25 MB caps) to push subscriptions. For confidential documents
  (legal, medical, financial) that upload is itself a problem.

The uncomfortable truth they don't advertise: **the overwhelming majority of PDF
operations can run entirely on the user's own device.** No server required.

## The solution

**Unfleece** is a free, open-source PDF toolkit where every operation runs **100% in the
browser** via WebAssembly. Files are never uploaded. There is no backend to bill for, so
there's nothing to monetize via a paywall — which is exactly the point.

## Principles

1. **Free forever** — core tools are never gated behind a trial or subscription.
2. **Private by architecture** — files never leave the device; verifiable in the Network
   tab, not just promised.
3. **Fast & light** — near-zero JS on landing; heavy engines lazy-load per tool.
4. **Honest** — labels match reality; no fake fidelity, no false "secure".
5. **Open source** — the running code is auditable. Trust is earned by inspection.

## Who it's for

- **Privacy-sensitive professionals** — lawyers, healthcare, finance, journalists — who
  *cannot* upload documents to a third party. "Files never leave your browser" is a
  cleaner compliance posture than any server-side competitor.
- **Everyday users** burned by fleeceware who just want to merge/split/compress without a
  surprise weekly bill.
- **Developers & the privacy community** — who value (and will audit + share) an
  open-source, no-upload tool.

## Differentiators vs. incumbents

| | Fleeceware (pdfSimpli…) | Honest freemium (Smallpdf/iLovePDF) | **Unfleece** |
|---|---|---|---|
| Price | "$1 trial" → ~$50/wk | Free tier, throttled → sub | **Free, unthrottled** |
| Your files | Uploaded | Uploaded | **Never leave device** |
| Account | Often required | Pushed | **None** |
| Source | Closed | Closed | **Open, auditable** |
| Compliance | Poor | OK (but uploads) | **Strong (no processor)** |

Three differentiators we lead with:
1. **Privacy by architecture** — "watch the Network tab; nothing uploads."
2. **No fleeceware** — unlimited, free, no account, no file-size paywall.
3. **Premium UX, free** — native-quality previews and a glass UI that match paid tools.

## Brand voice

Confident, a little cheeky, never smug. We're *for* the user and mildly *against* the
subscription-trap status quo — but the tone stays warm and helpful (it's "Unfleece", not
"Unfleece"). Plain language. We explain limits honestly instead of overpromising.

## Non-goals

- **Not** a server-side conversion farm. We will *not* add a backend that processes files
  to chase fidelity — that breaks the entire premise. (See the office-conversion call in
  `docs/02-features.md`.)
- **Not** an AI document product (summarize/chat) in v1 — those need an LLM and would
  break the no-upload promise.
- **Not** chasing the single highest-volume keyword ("pdf to word") on fidelity, which is
  structurally a server's game. We win where client-side genuinely competes.
