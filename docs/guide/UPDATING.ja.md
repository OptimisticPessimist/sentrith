<p align="right"><a href="UPDATING.en.md">English</a> ｜ <strong>日本語</strong></p>

# 既存プロジェクトのSentrithを更新する

新しいSentrith releaseを展開するかrepositoryをpullした後、Sentrith側のdirectoryから実行します。

**macOS / Linux**

```bash
./scripts/install.sh --update /path/to/your-project
```

**Windows PowerShell**

```powershell
./scripts/install.ps1 -Target C:\path\to\your-project -Update
```

CLIを使っている場合は同じversionへ揃えます。

```bash
./scripts/get-sentrith.sh /path/to/your-project
```

---

## `--update` と `--force` の違い

**`--update` を使ってください。**

| | `--update` | `--force` |
|---|---|---|
| Contract file(AGENTS.md、skills、policy、profiles) | 置換する | 置換する |
| Project Memory(PROJECT.md、STATE.md、PROFILE.md、DECISIONS.md、KNOWN_ISSUES.md) | **保持する** | **上書きする** |
| あなたのfeature spec(`docs/specs/<feature>/`) | 保持する | 保持する |
| `.claude/settings.json`、`.codex/hooks.json` | 保持する | 保持する |

`--force` は「導入をやり直す」ためのものです。更新に使うと、蓄積したProject Memoryが空のtemplateへ戻ります。

引数なしの通常installは、既存fileを検出すると**停止します**。これは事故防止のための挙動です。

---

## 更新で何が起きるか

1. Contract fileが新versionへ置換されます
2. その版で**新しく追加されたMemory file**があれば、未初期化のtemplateとして追加されます(既存fileには触れません)
3. 何が追加されたかがコンソールへ表示されます

例:

```text
New memory files added as uninitialized templates:
  docs/ai/PROFILE.md
```

---

## 更新後に確認すること

### 1. 差分を読む

```bash
git diff -- AGENTS.md docs/development docs/profiles .agents
```

Contract fileは対象repositoryにcommitされている前提です。差分が読めない場合、Sentrith管理fileをcommitしていない可能性があります。

### 2. 追加されたMemory fileを埋める

新しいtemplateが追加された場合、Coding Agentへ依頼します。

```text
docs/ai/BOOTSTRAP.md のProfile質問を実行して docs/ai/PROFILE.md を埋めて。
Applicationの挙動は変更しないで。
```

未初期化のままでも動作しますが、その分の機能は無効です。

### 3. ローカル改変の再適用

`AGENTS.md` などを独自に書き換えていた場合、その変更は置換されています。

`git diff` で確認し、必要な部分を再適用してください。

**Sentrith管理fileを直接編集するのは避けてください。**
project固有のruleは、更新で消えない場所へ置きます。

- repositoryの `CLAUDE.md` / `AGENTS.md` へ追記する代わりに、`docs/ai/PROJECT.md` の conventions へ書く
- 共有したくないローカル文脈は `.sentrith-private/` へ置く

### 4. 機械的checkを回す

```bash
sentrith preflight
```

---

## 注意すべき変更

### Usage CSVのschema

`.ai-usage/usage.csv` のschemaが更新された場合、`sentrith` は次回の書き込み時に**自動でin-place移行**します。

- 既存行は保持され、新しい列は空で埋められます
- 移行は冪等です
- baseline測定の途中でも、集計は列名で解決するため継続できます

移行前のCSVを保全したい場合は、更新前にcopyしてください。

### Hook設定

`.claude/settings.hooks.example.json` と `.codex/hooks.example.json` は**example**なので置換されます。

あなたが実際に使う `.claude/settings.json` / `.codex/hooks.json` は置換されません。

つまり **hook定義の変更は自動では反映されません**。exampleの差分を見て、必要ならmergeしてください。

```bash
git diff -- .claude/settings.hooks.example.json .codex/hooks.example.json
```

### CLIとContractのversion差

CLIとContractは別々に更新できますが、**同じversionへ揃えることを推奨**します。

古いCLIは新しいcheckを知らず、新しいCLIは古いcontractのpathを期待しません。

```bash
sentrith version
```

---

## 更新しても安全なもの / 危険なもの

安全:

- Contract fileの置換(内容がGitで追跡されている場合)
- Memory fileの追加

危険:

- `--force` での更新(Project Memoryが失われます)
- Sentrith管理fileへの直接編集(更新のたびに失われます)
- 対象repositoryでSentrith fileをcommitしていない状態(差分が確認できません)
