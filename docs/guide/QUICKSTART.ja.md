<p align="right"><a href="QUICKSTART.en.md">English</a> ｜ <strong>日本語</strong></p>

# Sentrith Sentrith pre-release prototype — クイックスタート

普段は**開発手法のコマンドを覚える必要はありません**。

## まずInstallする

既存repositoryへ導入する場合はRepository Contractから始めます。

```bash
./scripts/install.sh /path/to/your-project
```

Windows:

```powershell
./scripts/install.ps1 -Target C:\\path\\to\\your-project
```

何がcopyされるかは [Installation](INSTALLATION.ja.md) を参照してください。


## 基本

いつも通り自然言語で依頼してください。

```text
ログイン時、期限切れセッションで500になるのを直して
```

```text
CSVエクスポート機能を追加して
```

```text
決済基盤をStripeへ移行して
```

Agentは `AGENTS.md` のルールに従って、同じ作業ターンの中で自動的に:

1. Tiny / Normal / Significant を判定
2. 必要な場合だけ仕様を作成
3. test-first / regression-first が有効なら先に検証を作成
4. 実装
5. テスト・ビルド等を検証
6. 必要な場合だけADR/Project Memoryを更新

します。

**ユーザーが `/task-classify` → `/spec-feature` → `/implement-feature` と順番に操作する必要はありません。**

---

# 迷ったら `dev` だけ

明示的に標準フローを使わせたい場合だけ `dev` を使います。

## Codex

```text
$dev ログインの500エラーを直して
```

または普通に:

```text
ログインの500エラーを直して
```

## Claude Code

```text
/dev ログインの500エラーを直して
```

## GitHub Copilot CLI

```text
Use /dev to fix the login 500 error.
```

## GitHub Copilot IDE

```text
/dev
```

または普通のCopilot Agent依頼でも構いません。

---

# 既存プロジェクトへ最初に入れるときだけ

一度だけbootstrapします。

## Codex

```text
$project-bootstrap
```

## Claude Code

```text
/project-bootstrap
```

## Copilot CLI

```text
Use /project-bootstrap to initialize this repository.
```

## Copilot IDE

```text
/sdd-bootstrap
```

bootstrap結果の `docs/ai/PROJECT.md` と `STATE.md` を一度人間が確認したら、以後は普通に開発してください。

---

# Agentが内部で選ぶ開発レベル

## Tiny

例:

- typo
- CSS微調整
- 小さい局所バグ
- 機械的rename

```text
調査 → 修正 → 最小検証 → 終了
```

仕様書は作りません。

## Normal

例:

- 普通のバグ
- 局所的な機能追加
- validation変更
- 数ファイルにまたがる修正

```text
ゴール
→ Acceptance Criteria
→ regression test/check（有効なら）
→ 実装
→ 検証
→ Memory Gate
```

通常は専用SPECファイルを作りません。

## Significant

例:

- 新しい主要機能
- DB migration
- 公開API変更
- 認証・認可
- セキュリティ
- 外部サービス追加
- アーキテクチャ変更

```text
SPEC
→ PLAN
→ TASKS
→ test/check
→ 実装
→ 検証
→ 必要ならADR
→ Memory Gate
```

Agentが `docs/specs/<feature>/` を作ります。

---

# 必要なときだけ使う専門コマンド

普段は不要です。

| 用途 | Skill |
|---|---|
| 標準フロー全部おまかせ | `dev` |
| タスク分類だけ | `task-classify` |
| 仕様だけ作る | `spec-feature` |
| 実装 | `implement-feature` |
| 原因究明型デバッグ | `debug-root-cause` |
| コードレビュー | `review-change` |
| タスク終了処理 | `task-closeout` |
| Memory掃除 | `memory-audit` |
| 既存repo初期化 | `project-bootstrap` |

---

# 「自動化」の範囲

標準では次を自動化します。

```text
通常のユーザー依頼
        ↓
Agent内部で分類
        ↓
必要ならSDD
        ↓
必要ならtest-first
        ↓
実装
        ↓
検証
        ↓
Memory Gate
```

これは**同じAgentターン内のワークフロー**なので、分類やcloseoutのために別AIを起動しません。

---

# Hooksについて

Sentrithにはoptionalなhook用スクリプトがあります。

目的は:

- Project Memoryが未bootstrapなら短い警告
- `PROJECT.md` / `STATE.md` の肥大化を検知
- SPECやMemoryの状態を機械的に検査

です。

これらはPythonのローカルスクリプトだけで動き、LLMを呼びません。

そのため**hook自体のAIクレジット消費はありません**。

ただしhookが大量の出力をAgent contextへ渡すと、その文字列は入力tokenになり得るため、出力は短く抑えています。

---

# やらない自動化

標準では以下を行いません。

```text
タスク終了
→ 別のAI Agentを起動
→ 自動レビュー
→ 別のAIでMemory判定
→ さらにAIで要約
```

これは品質面では使い道がありますが、毎回モデル呼び出しが増えるのでクレジット節約目的と矛盾します。

必要な場合だけ明示的に `/review-change` 等を使用してください。

---

# 最も楽な普段の使い方

結局これだけです。

```text
普通にやりたいことを書く
```

例:

```text
このエラー直して
```

```text
ユーザー検索機能を追加して
```

```text
認証をOAuthに変えて
```

Agent側が開発手法を選びます。

**SDDはユーザーが操作する工程ではなく、Agentが必要なときだけ内部で適用する工程**として扱います。


# Sentrith pre-release prototype: 強いガード

短い依頼でも、Agentは `docs/development/SAFETY_GATES.md` に従います。

例えば:

```text
認証エラー直して
```

という依頼から、権限チェックを勝手に削除することは禁止されています。

```text
migration簡単にして
```

という依頼から、データを勝手にDROPすることも禁止されています。

高影響変更には、ユーザー明示要求・既存SPEC/RFC/ADR・repository contract等の根拠が必要です。

ガードの限界は `docs/guide/LIMITATIONS.ja.md` を参照してください。


# Sentrith pre-release prototype: 人間レビューも自動選別

Agentは変更を次の3段階で内部判定します。

```text
REVIEW-NOT-NEEDED
→ そのまま完了

REVIEW-RECOMMENDED
→ 止めずに完了し、要点だけ報告

REVIEW-REQUIRED
→ 安全な準備まで進める
→ 破壊的/不可逆ステップ直前だけ承認を求める
```

また、Agentが同じタスク内で自分で書いたSPEC/ADRだけを根拠に、
破壊的変更を自己承認することは禁止しています。


# Sentrith pre-release prototype: クレジット方針

標準では追加のreview Agent / memory Agent / summary Agentを自動起動しません。

同じAgentの作業内で処理します。

詳細:

- `docs/development/CREDIT_POLICY.md`
- `docs/meta/PHILOSOPHY.ja.md`
- `docs/meta/HISTORY.ja.md`

Dependency追加や巨大diffにも低コストのguardを追加しています。


# Usageを測る

導入効果を測る場合:

```bash
sentrith usage record --agent codex --phase standard --task "task name" --credits 5.0
sentrith usage report --compare
```

詳細は `docs/metrics/README.ja.md`。
