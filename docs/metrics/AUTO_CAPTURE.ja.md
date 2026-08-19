<p align="right"><a href="AUTO_CAPTURE.en.md">English</a> ｜ <strong>日本語</strong></p>

# 自動Usage Capture

Sentrithでは `sentrith` がAgentからusageを自動取得するadapterを持ちます。

**追加LLM呼び出しはありません。**

ただし各vendorが公開している機械可読情報が異なるため、精度も異なります。

## 対応状況

| Agent | 対話利用 | 非対話利用 | 自動取得 |
|---|---|---|---|
| Codex | Hooks + transcript | `codex exec --json` | Token usage |
| Claude Code | statusLine + Hooks | 同じ仕組み | Estimated USD cost / duration |
| Copilot CLI | stable machine APIなし | `copilot -p` wrapper | AI Credits best-effort |

---

## Codex: 推奨は `codex exec --json`

OpenAI公式の `turn.completed.usage` を直接読みます。

```bash
sentrith usage run codex \
  --task "fix login 500" \
  --phase standard \
  -- \
  "Fix the intermittent login 500 error."
```

内部:

```text
sentrith
→ codex exec --json
→ turn.completed.usage
→ usage.csv
```

取得:

- input tokens
- cached input tokens
- output tokens
- session/thread id

### Codex対話CLI

`.codex/hooks.example.json` に:

```text
UserPromptSubmit
Stop
```

hookを追加しています。

Codex hookは `transcript_path` を提供するため、開始/終了時のtoken snapshot差分を取ります。

ただしOpenAI公式は**transcript formatはstable interfaceではない**と明記しています。

そのため:

```text
codex exec --json
= stable / recommended

interactive transcript capture
= best effort
```

です。

---

## Claude Code: 対話利用を自動測定

Claude Codeはstatus line commandへJSONをstdinで渡します。

公式JSONには:

```text
cost.total_cost_usd
cost.total_duration_ms
context_window.*
session_id
model
```

があります。

Sentrithでは:

```text
statusLine
→ sentrith usage claude-status
→ latest snapshot
```

さらに:

```text
UserPromptSubmit
→ start snapshot

Stop
→ end snapshot
→ 差分をusage.csvへ保存
```

します。

### 有効化

`.claude/settings.hooks.example.json` を既存settingsへmergeしてください。

標準では:

```json
"statusLine": {
  "type": "command",
  "command": "./bin/sentrith usage claude-status"
}
```

を使用します。

取得:

- session estimated USD cost delta
- duration
- model/session id

注意:

Claude Code公式は `total_cost_usd` を推定costとして提供します。
請求の正式値はprovider側billingがsource of truthです。

---

## Copilot CLI

GitHub公式では:

```bash
copilot -p "..."
```

の通常出力にはusage情報が含まれ、
`-s` を付けるとusage情報を省略します。

Sentrith Sentrith pre-release prototype:

```bash
sentrith usage run copilot \
  --task "add CSV export" \
  --phase standard \
  -- \
  -p "Add CSV export."
```

`sentrith` は通常outputをそのまま表示しつつ、
usage footerからAI Creditsをbest-effortで抽出します。

### 制約

現行公式documentでは `/usage` は対話画面にstatisticsを表示しますが、
stableなJSON schemaは確認できません。

そのためinteractive Copilotの画面scrapingは標準実装しません。

---

# `usage record` はどうなる？

残します。

自動captureできない値や、provider billingから正式値を補正したい場合に使います。

```bash
sentrith usage record ...
```

つまり:

```text
usage run / hooks
= automatic capture

usage record
= manual/import fallback
```

です。

---

# Success / Reworkについて

消費量は自動取得できますが、

```text
本当に要求を満たしたか
```

は単純なCLI終了コードだけでは判定できません。

そのためinteractive hook captureでは `success` を自動で `yes` にしません。

README benchmarkで品質込み比較する場合は、
Acceptance Criteria確認後にsuccess/reworkを補完するのが最も正確です。

非対話wrapperではprocess exit codeを暫定successとして記録しますが、
これはengineering successの完全な代理ではありません。
