<p align="right"><a href="README_UPDATE_GUIDE.en.md">English</a> ｜ <strong>日本語</strong></p>

# READMEへ実測値を反映する方法

READMEのUsage Benchmark表示を、
サンプル値から実測値へ置き換えるための手順です。

---

# 1. 先に測定する

`BENCHMARK_GUIDE.ja.md` に従ってbaseline / standardを記録します。

```bash
sentrith usage report --compare
```

で結果を確認します。

---

# 2. 実測掲載の最低条件

推奨最低条件:

```text
baseline >= 5 tasks
standard >= 5 tasks
```

望ましい:

```text
10 + 10 tasks以上
```

1件の成功例だけでREADMEの削減率を更新しないでください。

---

# 3. READMEの対象箇所

英語 (canonical):

```text
README.md
```

日本語:

```text
README.ja.md
```

現在はUsage sectionに**illustrative output sample**があります。

実測値が十分集まるまでは、
この表示例を残してください。

---

# 4. 実測値へ変更するとき

表示例の注記:

```text
上記は表示例です
```

を、

```text
実測結果
```

へ単純に変えるだけでは不十分です。

必ず測定条件を併記します。

日本語例:

```md
### 実測結果

測定条件:

- Agent: OpenAI Codex
- Model: <model>
- Baseline: 10 tasks
- Standard: 10 tasks
- Task mix: bugfix 6 / small feature 4
- Measured: YYYY-MM-DD

| Metric | Baseline | Standard | Change |
|---|---:|---:|---:|
| Credits / task | 14.2 | 9.8 | -31.0% |
| Tool calls / task | 20.8 | 15.0 | -27.9% |
| Rework / task | 0.9 | 0.5 | -44.4% |
| Success rate | 90% | 100% | +10pp |
```

英語版にも同じ条件・値を反映します。

---

# 5. claimsを盛らない

良い表現:

```text
In this benchmark...
```

```text
この測定条件では...
```

避ける:

```text
AI creditsを必ず30%削減
```

```text
全プロジェクトで40%高速化
```

テンプレートの効果は:

- repository
- agent
- model
- task mix
- cache
- CI/tooling

で変わります。

---

# 6. READMEのおすすめ構成

実測値が十分集まったら:

```text
Usage Benchmark
├─ headline result
├─ condition
├─ compact table
├─ success/rework
└─ detailed benchmark guide link
```

にします。

長いraw logはREADMEへ載せません。

---

# 7. 更新時チェック

README更新前に確認:

- [ ] n数を書いた
- [ ] Agentを書いた
- [ ] Modelを書いた
- [ ] 測定日を書いた
- [ ] baseline定義を書いた
- [ ] standard定義を書いた
- [ ] success rateを書いた
- [ ] 実測と表示例を混同していない
- [ ] 日本語/英語で数字が一致している
- [ ] confidential task名を載せていない

---

# 8. 更新コミット例

```text
docs: publish initial usage benchmark
```

測定条件が変わった場合は、
以前の結果を黙って上書きせず、
測定条件の変更が分かるようにしてください。
