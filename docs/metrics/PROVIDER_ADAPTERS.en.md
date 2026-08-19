<p align="right"><strong>English</strong> ｜ <a href="PROVIDER_ADAPTERS.ja.md">日本語</a></p>

# Provider Adapters

Provider adapters connect Sentrith's Task Ledger to documented provider usage surfaces.

## GitHub Copilot / VS Code

Copilot in VS Code is a first-class measurement target.

When GitHub AI Credits API access is available:

```bash
sentrith usage task start \
  --agent copilot \
  --task "..." \
  --github-user USER \
  [--org ORG]

# work normally in VS Code

sentrith usage task stop --success yes
```

Sentrith records the start/end cumulative snapshot difference.

### Permission caveat

Personal and organization/enterprise-billed usage can require different endpoints and permissions.

A normal employee may not have organization billing access.

Fallback:

```bash
--snapshot-credits <cumulative-value>
```

or import/export a provider report into the same Task Ledger model.

Sentrith should not depend on Copilot extension private APIs or network interception.

## Claude Code

Prefer documented hooks/status information.

Estimated cost fields are useful for comparison but are not necessarily the provider's final invoice.

## Codex

Preferred order:

1. documented machine-readable usage
2. documented hooks
3. documented transcript location as an explicitly best-effort fallback

`codex exec --json` is a strong direct-capture path for non-interactive use.

## Gemini

Gemini belongs in the provider abstraction, but Sentrith should not claim complete automatic measurement for a UI when no stable documented usage surface exists.

Preferred fallback order:

1. documented usage API
2. documented billing/export data
3. manual cumulative snapshot

## Unsupported default strategies

Do not make these standard adapters:

- OCR
- IDE screen scraping
- undocumented private APIs
- TLS/network interception
- arbitrary parsing of internal agent databases

These are brittle and problematic in enterprise environments.
