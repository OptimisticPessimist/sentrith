<p align="right"><a href="CONTRIBUTING.md">English</a> ｜ <strong>日本語</strong></p>

# Sentrithへのコントリビューション

Sentrithへの改善参加は、コードを書くことだけではありません。

主な参加方法は3つです。

1. **Code & Automation** — Rust CLI、hooks、CI、provider adapter。
2. **Documentation & Engineering Profile** — ガイド、例、分野別Profile、翻訳。
3. **Measurement & Benchmark** — 匿名化したqualified benchmark data。

## 1. 最初に読むもの

挙動を変える変更では、まず以下を確認してください。

- [Development Method](docs/development/DEVELOPMENT_METHOD.md)
- [Verification Policy](docs/development/VERIFICATION_POLICY.md)
- [Safety Gates](docs/development/SAFETY_GATES.md)
- [Dependency Policy](docs/development/DEPENDENCY_POLICY.md)

小さな誤字修正まで全Policyを読む必要はありません。

## 2. 開発環境

### 必要なもの

- Git
- `tools/sentrith` を変更する場合のみRust toolchain
- Sentrith利用者にPython runtimeは不要

CLI build:

```bash
cargo build --manifest-path tools/sentrith/Cargo.toml
```

Test:

```bash
cargo test --manifest-path tools/sentrith/Cargo.toml
```

binaryが利用できる場合のdeterministic check:

```bash
sentrith preflight
sentrith guard
sentrith closeout-check
```

> Rust CLIを変更するPRでは、merge前にcompile/testすることを推奨します。

## 3. 最小の参加経路を選ぶ

### Code contribution

向いているもの:

- deterministic CLI check
- documented interfaceに基づくprovider usage adapter
- CI / release改善
- portability fix
- 再現可能なbug fix

決定論的なコードで解決できる処理に、新しいruntime dependencyや追加LLM callを導入しないでください。

### Documentation / Profile contribution

向いているもの:

- より明確な具体例
- 新しい分野固有のverification guidance
- stale docsの修正
- 日英parity改善
- methodology用語を要求しないEngineering Profile質問

日英で提供しているuser-facing docsは、可能なら同一PRで両方更新してください。

### Benchmark contribution

source codeやpromptを送らず、匿名集約結果だけを投稿できます。

```bash
sentrith usage contribute --agent <agent> --model "<model>"
```

詳細: [Community Benchmark](docs/metrics/COMMUNITY_BENCHMARK.ja.md)

## 4. 挙動を変える前に

変更scopeを明示してください。

通常の変更では、目的とacceptance criteriaをPR descriptionまたはtask/specへ書けば十分です。

以下のようなSignificant changeでは、必要に応じてSPEC / PLANへ昇格します。

- public compatibility change
- database / data migration
- authentication / authorization
- security-sensitive behavior
- 大きなprovider integration
- architecture change
- multi-subsystem change

Sentrithが対応しているからという理由だけでprocess artifactを増やさないでください。

## 5. Verification

変更内容に合ったEvidenceを使います。

例:

- Rust code → compile + test + focused behavior check
- docs → link/path確認 + 必要なら日英parity
- benchmark code → fixture/input validation + privacy-field check
- provider adapter → documented provider interface + fallback確認
- Game/3D profile → runtime / visual / platform verification guidance

Testがgreenでも、designが正しいことまでは自動的に証明しません。

## 6. Pull Request checklist

PR作成前に確認します。

- [ ] Scopeが狭く明確に説明されている。
- [ ] 意味がある場合はtest/checkを追加・更新した。
- [ ] 影響範囲の既存test/checkが通る。
- [ ] 高影響変更ではSafety Gatesを満たした。
- [ ] CIをgreenにする目的だけでtest/guardを弱めていない。
- [ ] 必要なuser-facing docsの日英を同期した。
- [ ] Durable knowledgeが変わった場合だけProject Memoryを更新した。
- [ ] Measurement変更ではprivacy/benchmark policyを維持した。
- [ ] Breaking changeは回避するか、明示・reviewしている。

## 7. Security-sensitive changes

Sentrithを公開するrepositoryにprivate reporting pathが用意されるまでは、実在する脆弱性の詳細をpublic issueへ書かないでください。

Security-sensitive変更では:

- diffを小さくする
- independent evidenceを保つ
- auth / secrets / crypto / checkを安易に弱めない
- Policy上必要なら `REVIEW-REQUIRED` とする

## 8. Language policy

Canonical landing page:

```text
README.md      English
README.ja.md   日本語
```

日英ペアの詳細Doc:

```text
*.en.md
*.ja.md
```

詳細: [Language Policy](docs/meta/LANGUAGE_POLICY.md)

## 9. Community behavior

具体的・Evidence-based・Respectfulに議論してください。

Architecture、methodology、benchmark、product directionへの反対意見は歓迎します。個人攻撃やgatekeepingは認めません。

## 10. 良いSentrith contributionとは

良いContributionは、たいてい次のどれかを改善します。

- repeated rediscoveryを減らす
- verification qualityを上げる
- safety boundaryを実行可能にする
- correctnessを落とさず不要なAI usageを減らす
- 導入を簡単にする
- domain profileを正確にする
- marketing claimではなくmeasured evidenceを強くする

> **効果をうたう前に、まず測る。**
