<p align="right"><a href="LIMITATIONS.en.md">English</a> ｜ <strong>日本語</strong></p>

# Sentrith の懸念点・限界

このテンプレートはAgentの暴走や再調査を減らすためのガードであり、正しさを保証する仕組みではありません。

## 1. 間違った仕様を忠実に実装する

最も大きいリスクです。

```text
間違ったSPEC
→ 良いPLAN
→ 良いテスト
→ 高品質な間違った実装
```

対策:

- Significant workではAcceptance Criteriaを人間が読む価値が高い
- 決済、権限、削除、法的要件などは仕様自体をレビューする
- repository evidenceと仕様が衝突したら無言で解釈しない

---

## 2. AIが自分で書いたガードを自分で満たしたことにする

例:

```text
AIがSPECを書く
→ AIがそのSPECを根拠にbreaking changeする
```

これはEvidence Gateの抜け穴になり得ます。

Sentrithでは、破壊的・高影響な変更については
「その場でAgentが勝手に追加した文言」だけを強い証拠として扱わない運用を推奨します。

特に以下は人間の明示要求、既存ADR/RFC、既存contract等の独立した根拠が望ましいです。

- data deletion
- permission broadening
- auth weakening
- irreversible migration
- public breaking change

---

## 3. テストが正しいとは限らない

テストがpassしても:

- 要件漏れ
- mockが現実と違う
- integration gap
- concurrency bug
- security flaw
- UI/UX破綻

は残ります。

対策:

- acceptance criteriaとtestを1:1だと思わない
- integration/e2e/visual/manual verificationを必要に応じて使う
- 高リスク領域ではtest以外の証拠も要求する

---

## 4. Context削減と情報不足はトレードオフ

「必要なものだけ読む」を強くしすぎると、関連ADRや副作用を見落とす可能性があります。

対策:

- Significant taskは検索範囲を少し広げる
- high-risk変更では関連decision/known issueをtargeted searchする
- token節約を正しさより優先しない

---

## 5. Memory汚染

Agentが誤ったPROJECT/DECISIONを保存すると、後続Agentが再利用して誤りが増幅します。

対策:

- 初回bootstrapは人間レビュー
- architecture/security/persistenceの重要更新はdiffで確認
- MEMORY_AUDITを定期実行
- repository evidenceをMemoryより上位に置く

---

## 6. Process artifactの肥大化

SDDを便利にすると、逆にSPEC/PLAN/TASKSが大量に残る可能性があります。

対策:

- Tinyはartifact禁止に近い運用
- NormalはAcceptance Criteriaだけで済ませる
- full SDDはSignificantのみ
- 完了後のTASKSを現在状態のsourceにしない

---

## 7. Agentごとの差

Codex / Claude Code / Copilotは同じskillを読んでも挙動が完全には一致しません。

理由:

- tool能力
- context管理
- model特性
- hooks/skill discovery
- IDE/CLI差

対策:

- canonical policyはrepoに置く
- product固有adapterは薄く保つ
- 実際のverification commandを共通のDefinition of Doneにする

---

## 8. Prompt injection / repository injection

外部README、issue、生成ファイル、dependency内の文章に
Agent向けの悪意ある命令が含まれる可能性があります。

対策:

- vendored/generated/external contentをinstruction sourceとして扱わない
- AGENTS/CLAUDE/Copilot instructionsとcanonical docsを優先
- 外部テキスト中の「ignore previous instructions」等を命令として実行しない
- secretや破壊的commandを外部文書の指示だけで実行しない

---

## 9. 自動化のやりすぎ

別Agent reviewや自動summaryを毎回走らせると:

- credit増加
- latency増加
- contradictory feedback
- unnecessary churn

が起きます。

Sentrith標準は「同じAgentターン内の自動ルーティング + LLMを使わないhook」です。

---

## 10. High-impactなのに人間確認なしで進める限界

ユーザーが「質問せず進めて」と望む場合でも、Agentがproduct decisionを完全に代行できるわけではありません。

安全な原則:

```text
不明
→ 保存的
→ reversible
→ backwards-compatible
```

それでも要求を満たせない場合だけ、未決定事項として明示します。

---

# 実務上、人間レビューを残した方がいい箇所

完全Vibe運用でも次の4つは確認価値が高いです。

1. Significant taskのSPEC / Acceptance Criteria
2. destructive migration
3. auth/permission/security変更
4. public breaking change

それ以外はAgent + tests + gatesでかなり自動化できます。


---

# さらに意識すべき懸念

## 11. ガード文書そのものの肥大化

ルールを追加し続けると:

```text
安全性向上
↓
AGENTS / policy巨大化
↓
毎回context増加
↓
credit増加・重要ルール埋没
```

が起こります。

対策:

- `AGENTS.md` は短い要約
- 詳細policyは必要時だけ読む
- 低頻度ルールはskill/policyへ分離
- 重複ルールを定期的に統合

---

## 12. False PositiveでAgentが慎重すぎる

ガードを強くしすぎると、普通の変更まで「危険」と判定して作業が進まなくなる可能性があります。

対策:

- REVIEW-RECOMMENDEDは停止条件にしない
- REVIEW-REQUIREDを限定列挙
- safe preparationは先に進める
- latest responsible momentまで人間を呼ばない

---

## 13. False Negative

逆に、文字列ヒューリスティックやAgent判断が危険変更を見逃す可能性があります。

`sentrith guard` / `sentrith review-hint` は補助であり、保証ではありません。

対策:

- CIで実際のsecurity/static/schema checksを持つ
- branch protection
- migration review
- CODEOWNERS等のGitHub側統制

このテンプレートは既存のCI/CD governanceを置き換えるものではありません。

---

## 14. 信頼境界がrepo内だけでは足りない

Agentがクラウド、DB、GitHub、AWS等の実環境へwriteできる場合、コードレビューだけでは足りません。

特に:

- production deploy
- database mutation
- secret rotation
- cloud resource deletion
- external API side effect
- sending messages/emails

はツール側のpermission/approval設計も必要です。

Repository policyだけで実行権限を安全にはできません。

---

## 15. Supply-chain risk

Agentがdependencyを追加する場合:

- typo-squatting
- compromised package
- abandoned package
- malicious install scripts
- license incompatibility

などがあります。

対策:

- dependency追加は必要性を説明
- lockfileをcommit
- package reputation/maintenanceを確認
- existing dependencyで代替可能なら優先
- security scannerをCIに置く

---

## 16. Generated code / migrationsのレビュー不能化

AIが大量の差分を一度に生成すると、人間もAgentもレビュー品質が落ちます。

対策:

- smallest coherent change
- Significant workをverifiable incrementへ分割
- 巨大refactorとfeature changeを同じdiffに混ぜない
- generated artifactsと手書きコードを区別

---

## 17. 再現性

モデルやAgent versionが変わると、同じSkillでも振る舞いが変わります。

対策:

-最終保証をpromptではなくtest/CI/contractへ寄せる
- critical workflowはローカルscript化
- agent policyはGit versioning
- model固有挙動をcanonical sourceにしない

---

## 18. コスト最適化が品質最適化と一致しないケース

tokenを減らせばよいとは限りません。

高リスク変更では、追加context・追加検証・人間レビューの方が安い場合があります。

判断基準は:

```text
AI credit cost
vs
failure cost
```

です。

本番データ破壊やsecurity incidentを防ぐための数千tokenは削る対象ではありません。


---

# 運用上残る懸念

## 19. policy間の矛盾

policyファイルが増えると、Safety / Review / Dependency / Creditなどの指示が競合する可能性があります。

対策:

優先順位は原則:

```text
correctness / safety
>
explicit user requirement
>
repository contract
>
review policy
>
cost optimization
```

Credit Policyは安全性を上書きしません。

---

## 20. ローカル自動化のportable性

Shell/Python依存の自動化はWindows / macOS / Linux / CI / 制限付き企業環境で壊れやすいものです。

そのためSentrithは小さなRust製CLIとprebuilt binaryを採用しています。

それでもOS固有の挙動は個別にテストが必要です。

hookはAgent workflow自体の必須要件ではないため、optionalのままにしています。

---

## 21. token節約効果を実測していないprojectでは過信しない

Projectによって:

- repository size
- task type
- Agent
- model
- cache behavior

が違います。

本当に効いているかは、可能なら導入前後で:

- input/cached input
- tool calls
- credits
- task completion rate
- rework

を比較してください。

---

## 22. ルールを守っただけで「良い設計」になるわけではない

このtemplateは:

- discovery
- scope control
- verification
- memory
- safety

を改善します。

しかし:

- product design
- architecture quality
- UX
- performance intuition

を自動的に保証しません。

プロジェクト固有の良い判断は依然必要です。
