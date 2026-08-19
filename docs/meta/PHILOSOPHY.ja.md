<p align="right"><a href="PHILOSOPHY.en.md">English</a> ｜ <strong>日本語</strong></p>

# Sentrith — 設計思想

このテンプレートの目的は、AIに「巨大な記憶」を持たせることではありません。

目的は:

```text
人間の入力は軽く
+
Agent内部の開発工程は堅く
+
同じ調査を何度も繰り返さず
+
必要なcontextだけ読み
+
追加のLLM呼び出しを極力増やさない
```

ことです。

---

# 1. 出発点

AI Coding Agentを長期プロジェクトで使うと、同じ問題が繰り返し発生します。

```text
毎回:
- このprojectは何でできている？
- build方法は？
- なぜこの設計？
- 前にもこのerrorなかった？
- どこまで実装済み？
```

これを毎回repositoryから再発見すると:

- input tokenが増える
- tool callが増える
- 同じ調査へcreditを使う
- Agentごとに理解がぶれる
- 過去の失敗を繰り返す

という問題が起きます。

そこで、repository自体を「AIが読むProject Memory」にします。

---

# 2. AIの記憶ではなく、Projectの記憶

基本原則:

```text
AI's memory != Project memory
```

Claude Auto Memory、chat history、session memoryなどは便利ですが、
projectの正式な事実をそこへ依存させません。

canonical knowledgeはGit管理された:

```text
docs/ai/
```

です。

理由:

- Agentを乗り換えても使える
- 人間がreviewできる
- Gitで差分/historyを追える
- 誤りを修正できる
- vendor lock-inを減らせる

---

# 3. Memoryを4種類へ分ける

1つの巨大な `MEMORY.md` は採用しません。

## PROJECT.md

長期的に安定した事実。

```text
stack
architecture
commands
repository map
external systems
conventions
```

## STATE.md

現在だけ重要な状態。

```text
unfinished work
blockers
failing checks
pending migration
next action
```

履歴ではありません。

Gitが履歴です。

## DECISIONS.md

重要な「なぜ」。

```text
なぜPostgreSQLなのか
なぜこのAPI contractなのか
なぜRedisを入れなかったのか
```

ADR相当です。

## KNOWN_ISSUES.md

再発見コストの高いトラブル知識。

```text
symptom
root cause
diagnosis
fix
failed workaround
```

---

# 4. Contextは資産ではなく予算

Project Memoryを増やせば増やすほど良いわけではありません。

悪い例:

```text
PROJECT 20k
STATE 10k
DECISIONS 100k
KNOWN_ISSUES 100k
↓
毎回全部読む
```

これはcredit削減と逆方向です。

そのため:

```text
毎回読む:
AGENTS
PROJECT
STATE

必要時だけ:
DECISIONS
KNOWN_ISSUES
SPEC
RFC
Skills
```

というProgressive Disclosureを基本にしています。

---

# 5. Credit削減の主因

このテンプレートはPrompt Cacheを魔法のように利用する仕組みではありません。

credit削減の主因は:

1. 同じproject調査を繰り返さない
2. 無関係なsourceを読まない
3. 過去の失敗を再調査しない
4. 人間が同じ説明を毎回書かない
5. 大量の不要な最終レポートを生成しない
6. 同じcontextが再利用されればcacheの恩恵も受けられる

です。

重要:

```text
Project Memory
≠
Prompt Cache
```

Project Memoryは「何を読むか」を改善します。

Prompt Cacheはモデル側の計算再利用です。

---

# 6. 「別Agentで自動化」を標準にしない

例えば:

```text
Implementation Agent
↓
Review Agent
↓
Memory Agent
↓
Summary Agent
```

と毎回実行すれば、品質が上がるケースはあります。

しかし:

- model call増加
- token増加
- latency増加
- Agent間の矛盾
- credit増加

につながります。

このtemplateの標準方針は:

```text
同じAgent turn内で
classify
→ plan
→ implement
→ verify
→ closeout
```

です。

別Agentは明示的に必要なときだけ使用します。

---

# 7. Vibe Codingを否定しない

このtemplateはVibe Codingを禁止するためのものではありません。

人間側は:

```text
このerror直して
```

```text
検索遅いから速くして
```

```text
OAuthにして
```

くらいの入力で構いません。

ただしAgent内部では:

```text
Vibe input
↓
task classification
↓
repository evidence
↓
必要ならSDD
↓
必要ならTDD
↓
Safety Gate
↓
Human Review Gate
↓
implementation
↓
verification
↓
Memory Gate
```

を行います。

思想:

```text
Vibe Coding UI
+
Structured Engineering Backend
```

です。

---

# 8. SDDは全タスクへ強制しない

仕様駆動開発はAI codingと相性が良い一方で、
小さな変更へ適用するとdocumentation costが過剰になります。

そこで3段階にしています。

## Tiny

```text
inspect
→ change
→ verify
```

SPEC不要。

## Normal

```text
goal
→ acceptance criteria
→ test/check
→ implementation
→ verify
```

通常はfull SPEC不要。

## Significant

```text
SPEC
→ PLAN
→ TASKS
→ tests/checks
→ implementation
→ verify
→ ADR
```

---

# 9. TDDも全タスクへ強制しない

TDDは:

- bug
- parser
- API behavior
- business logic
- regression

には強いです。

一方:

- visual
- game engine behavior
- deployment
- hardware
- exploratory prototype

には別のverificationの方が適切な場合があります。

そのため:

```text
test-first where meaningful
```

としています。

---

# 10. Hard Gates / Evidence Gates

短い依頼を理由にAgentが勝手に:

- DROP
- data deletion
- auth relaxation
- security weakening
- breaking API
- test削除
- CI無効化
- framework全面置換

へ進まないようにしています。

高影響変更にはEvidenceが必要です。

---

# 11. Self-authorizationを禁止する

重要な問題:

```text
Agent:
breaking changeしたい

↓
Agent自身がSPECに
breaking changeすると書く

↓
「SPECが根拠です」
```

ではGateになりません。

そのため同じtask内でAgent自身が作った:

- SPEC
- ADR
- PLAN
- changed test expectation

だけでは、破壊的変更の独立した承認になりません。

---

# 12. Human Reviewは最小限

毎回人間へ確認する設計にもしていません。

```text
REVIEW-NOT-NEEDED
→ 自動

REVIEW-RECOMMENDED
→ 止めずに進行

REVIEW-REQUIRED
→ safe preparationまで進める
→ 最後の不可逆操作だけ確認
```

つまり:

```text
latest responsible moment
```

でのみ人間を呼びます。

---

# 13. PromptよりCI/Testを信頼する

Agent instructionはsoft guardです。

本当に強い保証は:

```text
tests
contracts
type checks
lint
security scanners
CI
branch protection
CODEOWNERS
permissions
```

に置くべきです。

Agentを賢くするより、失敗できない仕組みをrepository/platform側へ置く方が強いです。

---

# 14. Repositoryの外側は別の安全設計が必要

AI Coding Agentが:

- production DB
- AWS/GCP/Azure
- Kubernetes
- GitHub settings
- external API
- mail/messages

へwriteできるなら、repository ruleだけでは不十分です。

必要:

```text
Repository policy
+
Tool permission
+
Environment permission
+
Human approval for irreversible operations
```

---

# 15. 追加ルール自体を増やしすぎない

このtemplateの将来的な最大リスクの1つは
「安全ルールを増やし続けること」です。

ルールが巨大になると:

- input token増
- cache効率悪化
- 重要ルール埋没
- maintenance cost増加

となります。

今後の原則:

```text
新ルール追加より
既存ルール統合を優先
```

です。

---

# 16. 最終的な目標

理想形:

```text
人間:
「これ直して」
        ↓
Agent:
意図理解
↓
project context取得
↓
risk判定
↓
必要なengineering process
↓
safe implementation
↓
verification
↓
knowledge compression
        ↓
人間:
結果だけ確認
```

人間に開発手法の操作を強制せず、
Agentにも無制限の裁量を与えません。

この中間を狙っています。


---

# 17. ローカル自動化もruntime依存を減らす

Python scriptは実装が簡単ですが、利用者環境にPythonを要求します。

v3.6ではローカルguard/usage機能をRust製単一バイナリへ統合しました。

思想:

```text
利用者:
runtime install不要

maintainer:
Rust sourceをGit管理
↓
GitHub Actions build
↓
prebuilt binariesをRelease
```

`sentrith` 自体はAI/API/networkを呼ばず、追加creditを消費しません。


---

# Sentrith pre-release prototype — Vibe Coding positioning

SentrithはVibe Codingを「危険だから禁止する」のではなく、
自然言語中心の軽い操作感を維持しつつ、その成果物へengineering disciplineを追加する。

```text
Vibe Coding UX
+
Structured Engineering Backend
```

をブランド上の中心メッセージとする。

Vibe Codingという語が将来陳腐化しても、
製品本体の定義は「vendor-neutral project memory + adaptive engineering workflow + guardrails」であり、
Vibe Codingは入口を説明するマーケティング語として扱う。
