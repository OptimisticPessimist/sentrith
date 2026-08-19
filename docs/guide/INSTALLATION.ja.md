<p align="right"><a href="INSTALLATION.en.md">English</a> ｜ <strong>日本語</strong></p>

# Installation

Sentrithは2つの要素で構成されます。

1. **Repository Contract** — あなたのproject内に置き、Coding Agentが読むファイル群。
2. **Optional Rust CLI** — deterministic checkとUsage Measurement。

Repository Contractだけでも利用できます。

## Option A — 既存repositoryへ入れる

Sentrithのrelease archiveを展開するかrepositoryをcloneした後、Sentrith側のdirectoryからhelperを実行します。

### macOS / Linux

```bash
./scripts/install.sh /path/to/your-project
```

### Windows PowerShell

```powershell
./scripts/install.ps1 -Target C:\path\to\your-project
```

helperは以下のvendor-neutral repository contractを対象repoへcopyします。

```text
AGENTS.md
CLAUDE.md
.github/copilot-instructions.md
.github/prompts/
.claude/
.codex/
docs/ai/
docs/development/
docs/specs/
docs/rfcs/
```

Application source codeは変更しません。
> 対象repoに同名のSentrith管理pathがある場合、installerは上書きせず停止します。意図的にmergeするか、置換が明確な場合だけ `--force` / `-Force` を使ってください。


## Project MemoryをBootstrapする

対象repositoryでCoding Agentへ次のように依頼します。

```text
docs/ai/BOOTSTRAP.md を読んで、このrepositoryのbootstrapを実行して。
Applicationの挙動は変更しないで。
```

完了後、一度だけ特に以下をReviewします。

```text
docs/ai/PROJECT.md
docs/ai/STATE.md
```

生成されたproject factsをdurable memoryとして扱う前に、人間が一度確認することが重要です。

## Optional CLI

Release assetが用意されている場合、利用者はprebuilt `sentrith` binaryを使うのが基本です。

Maintainerはlocal buildできます。

```bash
cargo build --release --manifest-path tools/sentrith/Cargo.toml
```

`PATH` に `sentrith` を置いたら確認します。

```bash
sentrith preflight
sentrith guard
```

## 最初の実タスク

あとは普段のCoding Agentへ普通に依頼します。

```text
login timeout bugを直して。
```

SentrithはRepository Contractを読み、別のclassification agentを起動せずに必要なEngineering depthを選ぶ想定です。

## Measurementはあとで足せる

AI Usage削減効果を検証するなら、workflow変更前にbaselineを取ります。

詳細: [Usage Measurement](../metrics/MEASUREMENT_ARCHITECTURE.ja.md)
