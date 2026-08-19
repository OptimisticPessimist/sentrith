<p align="right"><a href="PROVIDER_ADAPTERS.en.md">English</a> ｜ <strong>日本語</strong></p>

# Provider Adapters

## GitHub Copilot / VS Code

Sentrithの最優先測定対象の1つです。

GitHub公式AI Credits APIを利用できる環境では:

```bash
sentrith usage task start --agent copilot --task "..." --github-user USER [--org ORG]
# VS Codeで作業
sentrith usage task stop --success yes
```

で開始/終了snapshot差分を記録します。

### 権限の注意

個人契約とorganization/enterprise課金ではusage endpointと必要権限が異なります。
会社契約の一般ユーザーがorganization billing APIを読めない場合があります。

その場合は:

```bash
--snapshot-credits <cumulative-value>
```

またはprovider export/importを使います。

SentrithはCopilot extensionのprivate APIやnetwork traffic interceptionを標準方式にしません。

## Claude Code

documented hooks/status dataを優先します。
`cost.total_cost_usd` はestimated costであり、正式請求額そのものとは限りません。

## Codex

優先順位:

1. documented machine-readable usage
2. documented hooks
3. documented transcript pathを使ったbest-effort fallback

`codex exec --json` は直接capture可能な経路です。

## Gemini

Geminiはprovider abstractionには含めますが、UIごとにstable usage surfaceが確認できない場合は「完全自動」と表現しません。

documented API / billing export / manual snapshotの順で利用します。

## Unsupported strategy

以下は標準実装にしません:

- OCR
- IDE画面scraping
- undocumented private API
- TLS/network interception
- Agent内部DBの無断解析

壊れやすさ、企業利用時の安全性、provider規約への依存を避けるためです。
