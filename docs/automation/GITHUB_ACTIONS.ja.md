<p align="right"><a href="GITHUB_ACTIONS.en.md">English</a> ｜ <strong>日本語</strong></p>

# GitHub Actions — sentrith build / release

Sentrithには3つのworkflowを含めています。

## `sentrith-ci.yml`

`tools/sentrith/**` の変更時に:

- Linux
- Windows
- macOS

で `cargo test` とrelease buildを実行します。

## `sentrith-release.yml`

次のtagをpushすると自動Release:

```text
sentrith-v0.1.0
```

生成:

```text
sentrith-linux-x86_64
sentrith-linux-aarch64
sentrith-macos-x86_64
sentrith-macos-aarch64
sentrith-windows-x86_64.exe
```

workflow artifactへ保存した後、GitHub Releaseにも添付します。

手動 `workflow_dispatch` でも実行可能です。

## `sentrith-windows-arm-preview.yml`

Windows ARM64用の任意workflowです。

GitHub側のARM64 Windows runner可用性はrepository/account条件やPreview状態の影響を受けるため、
標準Release matrixからは分離しています。

## Release方法

```bash
git tag sentrith-v0.1.0
git push origin sentrith-v0.1.0
```

これだけです。

## 利用者

Release assetを取得して:

```text
bin/sentrith
bin/sentrith.exe
```

として配置します。

Rust/Python runtimeは不要です。
