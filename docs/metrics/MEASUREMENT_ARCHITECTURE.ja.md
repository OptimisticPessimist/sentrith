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
