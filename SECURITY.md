# Security Policy

## The model

Unfleece processes every file **entirely in your browser** — there is no server that
receives, stores, or can leak your documents. The static site is served from Cloudflare
Pages with a strict Content-Security-Policy, and you can verify the no-upload claim
yourself: open your browser's Network tab while using any tool.

What that means for severity: a typical "server breach" class of issue does not exist
here. The interesting surface is the client itself — XSS, malicious-PDF handling in the
parsing/rendering engines, supply-chain issues in dependencies, and anything that could
make file contents leave the device.

## Reporting a vulnerability

Please report security issues privately via
**[GitHub Security Advisories](https://github.com/darekhta/Unfleece/security/advisories/new)**
rather than a public issue. Include reproduction steps and, for malicious-file issues,
a sample file if you can share one.

You can expect an acknowledgement within a few days. There is no bug bounty — this is a
free, open-source project — but reports are credited in release notes unless you prefer
otherwise.

## Scope notes

- The **Sign** tool produces a *visual* signature mark, not a cryptographic signature —
  this is stated in the UI and is not a vulnerability.
- **Redact** rebuilds pages as images and is labeled accordingly; reports that flattened
  output is no longer selectable text are expected behavior.
- Dependency advisories are tracked by Dependabot and `npm audit` in CI.
