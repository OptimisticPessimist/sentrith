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
.agents/            # canonical cross-agent skills
.claude/skills/     # thin adapters
.claude/settings.hooks.example.json
.codex/hooks.example.json
docs/ai/
docs/development/
docs/profiles/
docs/specs/
docs/rfcs/
```

Application source codeは変更しません。

`.agents/` は必須です。`.claude/skills/` のadapterはここを参照します。

> 対象repoに同名のSentrith管理pathがある場合、installerは上書きせず停止します。
> **すでにSentrithを導入済みで新versionへ上げる場合は `--update` を使ってください。** Contract fileだけを置換し、Project Memoryを保持します。
> `--force` はProject Memoryごと置換するため、導入をやり直す場合にのみ使用します。

更新手順の詳細: [Updating](UPDATING.ja.md)


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

取得スクリプトで、最新Releaseから現在のOS向けbinaryを対象repositoryの `bin/` へダウンロードできます(SHA256検証つき)。

```bash
./scripts/get-sentrith.sh /path/to/your-project
```

Windows PowerShell:

```powershell
./scripts/get-sentrith.ps1 -Target C:\path\to\your-project
```

Rust toolchainは不要です。

Maintainerはlocal buildもできます。

```bash
cargo build --release --manifest-path tools/sentrith/Cargo.toml
```

`PATH` に `sentrith` を置いたら確認します。

```bash
sentrith preflight
sentrith guard
```

計測hookを有効にする場合は、手動のJSON編集ではなく次を実行します。

```bash
sentrith hooks install
```

既存の `.claude/settings.json` は保持され、Sentrithのhookだけが冪等にmergeされます。

## 最初の実タスク

あとは普段のCoding Agentへ普通に依頼します。

```text
login timeout bugを直して。
```

SentrithはRepository Contractを読み、別のclassification agentを起動せずに必要なEngineering depthを選ぶ想定です。

## Measurementはあとで足せる

AI Usage削減効果を検証するなら、workflow変更前にbaselineを取ります。

詳細: [Usage Measurement](../metrics/MEASUREMENT_ARCHITECTURE.ja.md)
