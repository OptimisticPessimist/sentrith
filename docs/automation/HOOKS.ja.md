<p align="right"><a href="HOOKS.en.md">English</a> ｜ <strong>日本語</strong></p>

# Optional Hooks — クレジットを増やさない自動化

Sentrithの標準フローはAgent instructionsだけで自動ルーティングします。

Hooksは**必須ではありません**。

ここではLLMを追加で呼ばないローカルcommand hookだけを対象にします。

## なぜLLM hookを標準にしないのか

以下は避けます。

```text
Stop
→ AI reviewerを起動
→ AI memory writerを起動
→ AI summaryを起動
```

モデル呼び出しが増えるため、クレジット節約と逆方向です。

---

## 同梱スクリプト

### `sentrith preflight`

機械的に確認:

- Project Memoryがbootstrap済みか
- `PROJECT.md` が400行を大きく超えていないか
- `STATE.md` が120行を超えていないか

AIを呼びません。

### `sentrith closeout-check`

Git差分を見て:

- 変更ファイル数
- migration/auth/security/infra等の高リスク領域

を短く警告します。

AIを呼びません。

---

# Codex

Codexはrepo-local `.codex/hooks.json` をサポートします。

Sentrithには `.codex/hooks.example.json` を同梱しています。

使う場合:

```bash
sentrith hooks install --agent codex
```

で `.codex/hooks.json` へmergeできます(手動コピーでも構いません)。

初回はCodexの `/hooks` でhookを確認・trustする必要があります。

command hook自体はローカル `sentrith` バイナリなので追加のAI呼び出しはありません。

ただしhook stdoutがcontextに追加される場合、その短い出力分のinput tokenは増えます。

---

# Claude Code

Claude Codeはsettings内のhooksでcommandを実行できます。

次のコマンドで、既存の `.claude/settings.json` へSentrithのhookだけを冪等にmergeできます。

```bash
sentrith hooks install --agent claude
```

既存の設定・他のhook・独自のstatusLineは保持され、何度実行しても重複しません。
Windowsでは `bin\sentrith.exe` 形式へ自動で書き換えます。

事前確認は `--dry-run`、状態確認は `sentrith hooks status` です。

手動でmergeする場合は `.claude/settings.hooks.example.json` を参考にしてください。

Claudeのprompt-based / agent-based hooksはモデルを使えるため、Sentrithでは標準採用しません。

---

# GitHub Copilot CLI

Copilot CLIはrepository hookを `.github/hooks/*.json` から読み込めます。

`.github/hooks/ai-project.example.json` を必要なら `ai-project.json` にrenameしてください。

shell hookだけなので、それ自体はAI creditを消費しません。

---

# 推奨

まずHooksなしで使ってください。

```text
AGENTS instructions
+ dev workflow
```

だけで十分です。

次の場合だけHooksを有効化:

- bootstrap忘れを機械的に検知したい
- Memory肥大化を警告したい
- high-risk変更を必ず警告したい

**クレジット削減を最優先するなら、LLMベースhookや自動subagent reviewはOFFが基本です。**


# Runtime要件

Python runtimeへの依存は廃止しました。

Hooksは `bin/sentrith` / `bin/sentrith.exe` を実行します。

バイナリはGitHub Actionsで各OS向けにbuildしてRelease配布できます。
`docs/automation/GITHUB_ACTIONS.ja.md` を参照してください。

## Windowsでの注意

Windows nativeのClaude Codeではhook commandがcmd.exe経由で実行されるため、`./bin/sentrith` 形式のパスは解決されないことがあります。

その場合は次のいずれかにしてください。

```text
bin\sentrith.exe preflight
```

または絶対パス指定。バイナリ名は `sentrith.exe` です(`bin/README.md` 参照)。

WSL / Git Bash経由で使う場合は `./bin/sentrith` のままで動作します。


# Usage Capture

Claude Code:

- statusLine → `sentrith usage claude-status`
- UserPromptSubmit → `sentrith usage hook claude`
- Stop → `sentrith usage hook claude`

Codex:

- UserPromptSubmit → `sentrith usage hook codex`
- Stop → `sentrith usage hook codex`

これらはすべてローカルcommand hookで、追加LLMを起動しません。

token・model・test実行結果はStop時にtranscriptを1回走査して取得します。costはstatusLineのfallbackです。

phaseは `sentrith usage baseline start` / `stop` で切り替えます(既定は `standard`)。

詳細: [`../metrics/AUTO_CAPTURE.ja.md`](../metrics/AUTO_CAPTURE.ja.md) / [`../metrics/MEASUREMENT_ARCHITECTURE.ja.md`](../metrics/MEASUREMENT_ARCHITECTURE.ja.md)
