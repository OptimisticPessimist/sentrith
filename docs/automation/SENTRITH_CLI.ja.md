<p align="right"><a href="SENTRITH_CLI.en.md">English</a> ｜ <strong>日本語</strong></p>

# sentrith CLI

Sentrith v1.0からローカル自動化・Usage測定はPythonではなくRust製単一CLIに統合しています。

## 利用者

RustもPythonも不要です。

GitHub Releaseから自分のOS向けバイナリを取得して `bin/` に置きます。

```text
bin/
├─ sentrith        # Linux / macOS
└─ sentrith.exe    # Windows
```

## コマンド

```text
sentrith preflight
sentrith closeout-check
sentrith guard
sentrith review-hint
sentrith diff-budget
```

Usage:

```text
sentrith usage record --agent codex --phase standard --task "fix login" --credits 5.2
sentrith usage report --compare
sentrith usage note "Codex /status before task: ..."
```

## 性質

`sentrith` は:

- AIモデル/APIを呼びません
- ネットワークへ接続しません
- Rust標準ライブラリだけで実装されています
- ローカルfilesystemとGit差分だけを読みます

したがって `sentrith` の実行自体によるAI credit消費はありません。

## 開発者

Rust source:

```text
tools/sentrith/
```

Build:

```bash
cargo build --release --manifest-path tools/sentrith/Cargo.toml
```

Test:

```bash
cargo test --manifest-path tools/sentrith/Cargo.toml
```

通常利用者はbuild不要です。


## Sentrith pre-release prototype Usage commands

```bash
sentrith usage snapshot copilot --github-user USER [--org ORG]

sentrith usage task start --agent copilot --task "..." [--github-user USER] [--org ORG]
sentrith usage task stop --success yes [--rework 0]

sentrith usage contribute --agent copilot --model "<model>"
sentrith usage aggregate
sentrith usage aggregate --publish
```

`task` はIDE/Desktop/CLIに依存しない作業境界です。
Provider snapshotが使えない場合は`--snapshot-credits`等で同じledgerへ入れます。
