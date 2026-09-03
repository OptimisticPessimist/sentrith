<p align="right"><a href="DATA.en.md">English</a> ｜ <strong>日本語</strong></p>

# Data Science / Data Engineering Profile

Data Profileは、**結果が再現できるか**と**データが静かに壊れていないか**を扱います。

## 適用トリガー

```text
ETL / ELT pipeline、DAG、scheduler
dataset生成・変換・結合
分析notebook、集計ロジック、指標定義
data schemaの変更、backfill
データ品質チェック、欠損・重複処理
```

## 追加するVerification次元

- **Reproducibility** — 同じ入力から同じ出力が得られるか(乱数seed、実行順序、タイムゾーン)
- **Leakage check** — 学習・評価の分割に未来情報や目的変数が混入していないか
- **Schema contract** — 上流schema変更が下流を壊さないか
- **Row-level integrity** — 件数、粒度、結合による意図しない増殖
- **Idempotency / backfill safety** — 再実行が二重計上を生まないか
- **Provenance** — dataset version、生成日時、生成コードの対応

## 技法ゲート

| 技法 | 適用条件 | 適用しない場合 |
|---|---|---|
| **Data contract test** | 上流を自分で制御していない | 単一repo内の閉じた変換 |
| **Great-expectations型の品質assert** | 品質劣化が静かに進行しうる本番pipeline | 一度きりの調査 |
| **Property-Based Testing** | 変換関数の入力空間が広い | 代表例で十分 |
| **Statistical Review** | 指標の変化がノイズと区別しにくい | 変化が構造的で明確 |
| **Snapshot / golden dataset** | 出力が大きく差分レビューが困難 | 出力が小さく直接検査できる |

## 「正しさ」の定義

```text
pipelineが完走した
≠
データが正しい
```

最低限、次を観測可能にします。

- 出力件数・粒度が期待通りであること
- 再実行しても結果が変わらない(または変わる理由が明示されている)こと
- 欠損・重複・型の扱いが暗黙でないこと
- backfillが既存の正しい行を壊さないこと

## Notebookの扱い

notebookは探索には有効ですが、**実行順序が状態に依存する**ため再現性検証には弱い媒体です。

durableなロジックはnotebookからモジュールへ移し、検証可能にします。
