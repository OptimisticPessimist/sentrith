<p align="right"><a href="README.en.md">English</a> ｜ <strong>日本語</strong></p>

# Sentrith Usage Measurement

Sentrithは「usageが減るはず」ではなく、実測することを前提にします。

## まず動かす

読む前に、この3コマンドで始められます。

```bash
sentrith hooks install
sentrith usage baseline start
```

以降は**普通に開発するだけ**です。手動での記録は不要です。

baselineが十分たまったら戻します。

```bash
sentrith usage baseline stop
```

今どこにいて次に何をすればいいかは、常にこれで分かります。

```bash
sentrith usage status
```

比較可能になると `status` が次のコマンドを案内します。

---

## 3コマンドが何をするか

| コマンド | 内容 |
|---|---|
| `hooks install` | `.claude/settings.json` / `.codex/hooks.json` へSentrithのhookだけを冪等にmerge。既存の設定・他のhook・独自statusLineは保持します。手動のJSON編集は不要です。 |
| `usage baseline start` | Agent instruction file(`AGENTS.md` 等)を `.sentrith-private/baseline-stash/` へ退避し、以降の記録を `baseline` にします。計測hookとデータは動いたままです。 |
| `usage baseline stop` | 退避したfileを元に戻し、記録を `standard` へ戻します。 |

baselineは「Sentrithが無い状態」を測る必要があるため、この部分だけは明示的な操作が要ります。

`baseline start` / `stop` の後は、**新しいAgent sessionを開始してください**。退避前の指示が古いsessionのcontextに残っているためです。

---

## 記録されるもの

hookが1 turnにつき1行を `.ai-usage/usage.csv` へ追記します。

- token / model — transcriptから取得
- cost / duration — statusLineから取得(fallback)
- success — commit到達とtest結果から機械的に判定。判定不能は `unknown`
- head_sha — task境界の判定に使用

Agentへの追加のモデル呼び出しはありません。

---

## 読む順

仕組みを詳しく知りたい場合:

1. [`MEASUREMENT_ARCHITECTURE.ja.md`](MEASUREMENT_ARCHITECTURE.ja.md) — task境界とsuccessの定義
2. [`AUTO_CAPTURE.ja.md`](AUTO_CAPTURE.ja.md) — Agent別の取得方法
3. [`PROVIDER_ADAPTERS.ja.md`](PROVIDER_ADAPTERS.ja.md) — Provider差
4. [`BENCHMARK_GUIDE.ja.md`](BENCHMARK_GUIDE.ja.md) — 比較設計とサンプル数
5. [`COMMUNITY_BENCHMARK.ja.md`](COMMUNITY_BENCHMARK.ja.md) — 匿名contribution
6. [`AUTO_PUBLISH.ja.md`](AUTO_PUBLISH.ja.md) — READMEへの反映

---

## データの置き場所

Private raw usage:

```text
.ai-usage/
```

Public community data:

```text
docs/metrics/contributions/
```

raw dataは標準ではrepositoryへcommitしません。

公開benchmark datasetへ入るのは匿名集約済みのcontribution fileだけです。
