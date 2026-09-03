<p align="right"><a href="BENCHMARK_GUIDE.en.md">English</a> ｜ <strong>日本語</strong></p>

# 実測ベンチマーク手順

この文書は、Sentrith導入前後で
**本当にAI usage / credits / tool calls / reworkが改善したか**を測るための手順です。

目的は「tokenが減ったことを証明する」ことではありません。

最終的に見るべきものは:

```text
credits per successful task
+
rework
+
success rate
```

です。

---

# 1. 測定設計

## 最低サンプル数

推奨:

```text
baseline: 5〜10 tasks以上
standard: 5〜10 tasks以上
```

1件だけの比較は避けてください。

理由:

- task難易度
- Agent/model差
- cache hit
- repository状態
- tool availability
-偶然のretry

の影響が大きいためです。

---

# 2. タスク群を揃える

baselineとstandardで、なるべく同程度の作業を比較します。

例:

```text
Bug fix
Bug fix
Small feature
Validation change
Refactor
API change
```

baseline側だけ簡単なタスク、
standard側だけ難しいタスクでは比較になりません。

完全に同じtaskを2回実行すると、
2回目がGit/cache/既知知識の影響を受ける場合があります。

そのため推奨は:

```text
同一taskではなく
同じカテゴリ・同程度の難易度のtask群
```

です。

---

# 3. Baseline

テンプレート未導入、または標準workflowを使わない状態で記録します。

## hook自動計測を使う場合(推奨)

Sentrithのcontractを外した状態へ切り替えます。

```bash
sentrith usage baseline start
```

Agent instruction fileが `.sentrith-private/baseline-stash/` へ退避され、以降の記録が `baseline` になります。計測hookとデータは動いたままです。

**新しいAgent sessionを開始してから**、普通に開発してください。手動記録は不要です。

進捗はいつでも確認できます。

```bash
sentrith usage status
```

十分たまったら戻します。

```bash
sentrith usage baseline stop
```

退避したfileが復元され、記録は `standard` へ戻ります。

phaseの優先順位は `--phase` > `.ai-usage/phase`(このコマンドが書くmarker) > `SENTRITH_PHASE` > `standard` です。

markerが環境変数より上位なのは、hookがAgent processから起動されるためです。Agent起動後にexportした変数はhookへ届きません。

## 手動記録の場合

例:

```bash
sentrith usage record \
  --agent codex \
  --model "<model>" \
  --phase baseline \
  --task "fix auth regression A" \
  --input 62000 \
  --cached-input 15000 \
  --output 5500 \
  --credits 14.3 \
  --tool-calls 21 \
  --duration 840 \
  --success yes \
  --rework 1
```

取得できない項目は省略可能です。

---

# 4. Standard

Sentrithを有効にして同様に記録します。

```bash
sentrith usage record \
  --agent codex \
  --model "<model>" \
  --phase standard \
  --task "fix auth regression B" \
  --input 39000 \
  --cached-input 21000 \
  --output 4200 \
  --credits 8.7 \
  --tool-calls 13 \
  --duration 510 \
  --success yes \
  --rework 0
```

---

# 5. 各数値の取得元

Agent/UIによって表示項目は異なります。

取得できる値だけ記録します。

## credits

各製品のUsage / Billing / Status表示から取得。

## input / cached input / output

CLIやUsage表示にtoken情報が出る場合のみ記録。

## tool calls

tool実行回数が確認できる場合。

手動集計でも構いません。

## duration

開始からtask完了まで。

人間の中断時間を含めるかどうかは、
baseline / standardでルールを統一してください。

## success

推奨定義:

```text
yes
= Acceptance Criteriaを満たし、必要なverificationがpass

partial
= 一部完成、manual step / unresolved issueあり

no
= 要求を満たせなかった、または重大なreworkが必要
```

## rework

Agentが「完成」とした後に必要になった追加修正回数。

例:

```text
0 = 一発でaccept
1 = review後に1回修正
2 = 2回追加修正
```

---

# 6. レポート

```bash
sentrith usage report --compare
```

Agent別:

```bash
sentrith usage report --agent codex --compare
```

---

# 7. 主指標

## Credits per successful task

可能ならこれを最重要指標にします。

考え方:

```text
total credits
/
successful tasks
```

単純な平均credits/taskより、
失敗した安いtaskを「効率が良い」と誤認しにくくなります。

現行 `sentrith usage report` は平均値とsuccess rateを表示します。

将来CLIへ直接この指標を追加しても構いません。

---

# 8. 改善判定

良い例:

```text
credits          -30%
tool calls       -25%
rework           -40%
success rate     same or better
```

悪い例:

```text
credits          -40%
success rate     -20pp
rework           +80%
```

後者はtoken節約に成功しても、
engineering workflowとしては失敗です。

---

# 9. Agent / Modelを混ぜない

可能なら:

```text
Codex + Model A baseline
vs
Codex + Model A standard
```

で比較します。

途中でmodelを変えた場合、そのdatasetは分離してください。

Claude / Copilot / Codexを一つの平均に混ぜると、
テンプレート効果なのかmodel差なのか判断しにくくなります。

---

# 10. Cacheの扱い

cached inputが見える場合は必ず別列で残します。

理由:

```text
input削減
```

と

```text
同じinputだがcache hit増加
```

は別の改善だからです。

両方ともcredit削減には寄与し得ますが、
原因を分けて見られる方が改善しやすくなります。

---

# 11. 結果をREADMEへ載せる条件

READMEへ「実測」として掲載する場合は、
最低限次を併記してください。

- Agent
- Model
- sample size
- repository/task category
- baseline / standardの定義
- measurement date
- primary metrics
- success rate
- illustrativeではなく実測であること

避ける:

```text
Credits -40%
```

だけを書くこと。

推奨:

```text
Codex / <model>, n=10 + 10 tasks,
bugfix + small-feature mix

credits/task      -31%
tool calls/task   -28%
rework/task       -44%
success rate      90% → 100%
```

---

# 12. 再現性

可能なら生データは:

```text
.ai-usage/usage.csv
```

から匿名化して別途公開できます。

ただし:

- private repository名
- task本文
- customer情報
- confidential code情報

を含む場合は公開しないでください。

READMEには集計値だけで十分です。
