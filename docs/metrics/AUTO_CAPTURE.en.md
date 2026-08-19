<p align="right"><strong>English</strong> ｜ <a href="AUTO_CAPTURE.ja.md">日本語</a></p>

# Automatic Usage Capture

Sentrith Sentrith pre-release prototype adds vendor adapters that capture usage without extra model calls.

- **Codex:** stable automatic token capture from `codex exec --json`; interactive hooks use `transcript_path` as a best-effort fallback because OpenAI explicitly warns that transcript format is not a stable hook interface.
- **Claude Code:** automatic per-turn estimated USD cost/duration using official statusLine JSON plus `UserPromptSubmit`/`Stop` hooks.
- **Copilot CLI:** programmatic `copilot -p` wrapper parses the normal usage footer on a best-effort basis. GitHub documents `/usage` for interactive statistics but does not currently document a stable JSON schema for it.

Examples:

```bash
sentrith usage run codex --task "fix login" -- "Fix login."
sentrith usage run copilot --task "add export" -- -p "Add CSV export."
```

Claude Code uses the provided statusLine/hooks configuration and does not require a wrapper.

Manual `sentrith usage record` remains available as an import/fallback path.
