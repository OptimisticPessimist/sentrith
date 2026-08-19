<p align="right"><a href="HISTORY.en.md">English</a> ｜ <strong>日本語</strong></p>

# Sentrith — Design History

> `docs/meta/` は通常のfeature/bugfix/refactor時に読み込まない、人間向けの設計史です。

## Pre-release prototype era

Sentrithという名前で公開する前に、内部プロトタイプとして複数回の設計反復を行いました。
当時は内部snapshot番号を付けていましたが、**それらはSentrithの公開リリースではありません**。

主な進化:

1. Project Memoryを `PROJECT / STATE / DECISIONS / KNOWN_ISSUES` に分離
2. vendor-neutralなCodex / Claude Code / GitHub Copilot adapter
3. Tiny / Normal / Significant のadaptive development flow
4. SDD / TDD / ADRを必要な場合だけ適用
5. Safety Gates / Evidence Gates / Human Review
6. 追加LLM呼び出しを避けるCredit Policy
7. deterministicなRust CLI / GitHub Actions
8. Usage logging / baseline comparison / README publication
9. Agent別automatic usage capture
10. IDE / Desktop / CLIを横断するProvider Usage + Task Ledgerへ一般化
11. 匿名Community Benchmark contribution

## Sentrith v1.0.0 — First public release

最初の公開リリース。

中心メッセージ:

> **雑に頼める。雑には作らせない。**

> **Vibe fast. Engineer the result.**

製品の4本柱:

- **Remember** — durable Project Memory
- **Structure** — adaptive engineering workflow
- **Guard** — safety/evidence/review gates
- **Measure** — real usage measurement and community benchmarking

公開version historyは **Sentrith v1.0.0から開始**する。
pre-release prototypeの内部snapshot番号は公開versionとして扱わない。
