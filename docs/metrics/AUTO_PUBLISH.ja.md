<p align="right"><a href="AUTO_PUBLISH.en.md">English</a> ｜ <strong>日本語</strong></p>

# 実測集計・README自動更新

Sentrithでは `sentrith` がusage CSVの集計からREADME更新まで行えます。

AI/APIを追加で呼びません。

---

## 1. 日々の記録

```bash
sentrith usage record \
  --agent codex \
  --model "<model>" \
  --phase baseline \
  --task "fix login A" \
  --credits 12.4 \
  --tool-calls 18 \
  --success yes \
  --rework 0
```

standard導入後:

```bash
sentrith usage record \
  --agent codex \
  --model "<model>" \
  --phase standard \
  --task "fix login B" \
  --credits 8.1 \
  --tool-calls 11 \
  --success yes \
  --rework 0
```

---

## 2. README更新前の確認

```bash
sentrith usage publish \
  --agent codex \
  --model "<model>" \
  --task-mix "bugfix + small feature" \
  --date YYYY-MM-DD \
  --dry-run
```

READMEは変更せず、生成予定の日本語・英語benchmark sectionだけ表示します。

---

## 3. README自動更新

```bash
sentrith usage publish \
  --agent codex \
  --model "<model>" \
  --task-mix "bugfix + small feature" \
  --date YYYY-MM-DD
```

自動更新対象:

```text
README.md
README.ja.md
```

それぞれの:

```text
<!-- SENTRITH-USAGE-BENCHMARK:BEGIN -->
...
<!-- SENTRITH-USAGE-BENCHMARK:END -->
```

の間だけを書き換えます。

他のREADME本文には触れません。

---

## 4. サンプル不足時は失敗する

デフォルトでは:

```text
baseline >= 5
standard >= 5
```

が必要です。

足りない場合:

```text
refusing README publication
```

として更新しません。

変更:

```bash
--min-samples 10
```

で10+10を要求できます。

---

## 5. `--force`

```bash
sentrith usage publish ... --force
```

で少数サンプルでも掲載できますが、
READMEには小規模sampleである警告を自動表示します。

通常公開には非推奨です。

---

## 6. 自動計算する指標

- Credits / task
- Credits / successful task
- Input tokens / task
- Cached input / task
- Tool calls / task
- Rework / task
- Success rate
- baseline → standard change

欠損項目は `-` として掲載します。

---

## 7. 完全自動化できない部分

`sentrith` はvendor-neutralなので、Codex / Claude / Copilotの
private usage画面やbilling APIへ勝手に接続しません。

したがって:

```text
credits
token counts
```

の取得は各Agent側から値が得られる場合に `usage record` へ渡す必要があります。

一方、

```text
集計
publication threshold
README table生成
日英同期
README書き換え
```

は完全自動です。

---

## 8. GitHub Actionsで定期更新する場合

`.ai-usage/usage.csv` は標準ではgitignoreされています。

そのため公開benchmarkをCIで自動更新したい場合は、

- anonymized benchmark CSVだけ別途commitする
- artifact/storageからworkflow内で取得する

などの運用が必要です。

private task名を含む生データをpublic repositoryへcommitしないでください。
