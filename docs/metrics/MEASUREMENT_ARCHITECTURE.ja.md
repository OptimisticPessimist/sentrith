<p align="right"><a href="MEASUREMENT_ARCHITECTURE.en.md">English</a> ｜ <strong>日本語</strong></p>

# Measurement Architecture

Sentrithではusage測定をCLI wrapper中心ではなく、**UI非依存の測定モデル**として扱います。

## 原則

```text
Task Ledger
+
Provider Usage
+
Success / Rework
=
Usage per successful task
```

IDE、Desktop app、CLI、Webのどれを使ったかは本質ではありません。

## Task Ledger

Sentrithが持つもの:

- task start / stop
- baseline / standard
- agent / model
- task category
- success
- rework
- duration

Provider usageと結合する前の作業境界です。

`usage task start` / `stop` による明示的なledgerに加えて、hook計測では作業境界を**自動導出**します。

```text
1 turn   = usage.csv 1行
1 task   = 同一session内で head_sha が変わるまでのturn群
```

`sentrith usage report --tasks` はこの規則でturnをtaskへ集約して集計します。

## Successの定義(自動計測)

hookで自動記録された行のsuccessは、**人間の主観ではなくrepository evidence**から導きます。

```text
commit到達 + 直近のtest実行がgreen  -> yes
commit到達 + 直近のtest実行がred    -> no
それ以外(判定材料なし)              -> unknown
```

判定材料:

- **commit到達**: `UserPromptSubmit` 時点のHEADと `Stop` 時点のHEADが異なる
- **test実行結果**: turn内で実行されたtest系コマンド(`cargo test`、`pytest`、`npm test` 等)の成否

`unknown` は失敗ではありません。**success rateの分母から除外**します。

```text
success rate = yes / (yes + no)
```

これにより、自動計測が成功率を不当に押し下げることを防ぎます。unknownの件数は必ず併記します。

この定義は再現可能である代わりに、「人間が成功と感じたか」とは一致しません。commitせずに終わった正しい作業はunknownになります。

## Rework(churn proxy)

reworkは記録時に仕込まず、**git履歴から事後計算**します。

```bash
sentrith usage report --churn --days 14
```

記録されたcommitについて「そのcommitが触れたfileのうち、N日以内に再度変更されたfileの割合」を出します。

file単位のproxyであり、line単位の帰属ではありません。

## Phase(baseline / standard)

baseline測定は専用コマンドで切り替えます。

```bash
sentrith usage baseline start
sentrith usage baseline stop
```

優先順位:

```text
--phase > .ai-usage/phase(marker) > SENTRITH_PHASE > standard
```

markerが環境変数より上位なのは、hookがAgent processから起動されるためです。
Agent起動後にexportした変数はhookへ届かず、markerなら必ず読めます。

baseline測定はSentrithのcontractを外した状態で行う必要があるため、切り替えだけは明示的な操作です。

## Provider Usage

source of truthは可能な限りprovider側です。

- GitHub Copilot: AI Credits API / export / manual snapshot
- Claude Code: documented status/hook cost data
- Codex: documented hooks / JSON usage surface
- Gemini: documented usage/billing/import surface

private API、通信傍受、UI OCR/scrapingを標準方式にしません。

## Native metric

provider単位は保持します。

```text
Copilot -> AI Credits
Claude  -> estimated USD / tokens
Codex   -> tokens / provider usage
Gemini  -> tokens / provider cost
```

## Cross-provider metric

異なる単位を直接足しません。

各環境で:

```text
baseline usage / successful task = 100
```

としてSentrith導入後の相対変化を計算します。

Community Benchmarkでは、このbaseline-relative changeの中央値を使います。

## なぜsuccessful taskか

単純なusage/task削減だけでは:

```text
安くなった
+
失敗率が上がった
```

ケースを改善と誤認します。

そのため主指標は:

> usage / successful task

です。

success rate / rework / durationも必ず別に確認します。
