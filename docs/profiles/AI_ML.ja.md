<p align="right"><a href="AI_ML.en.md">English</a> ｜ <strong>日本語</strong></p>

# AI / ML Engineering Profile

AI / ML Profileは、**コードが動いても品質が落ちうる**変更を扱います。

```text
テストがgreen
≠
モデルの出力品質が保たれている
```

## 適用トリガー

```text
prompt / system prompt / few-shot例
model / providerの切り替え、parameter変更
RAGのchunking / embedding / retrieval
fine-tuning、学習データ、前処理
評価コード、閾値、スコア計算
推論コスト・latencyに影響する経路
```

## 追加するVerification次元

- **Baseline比較** — 変更前のスコアを持たない変更は「改善」と呼べません
- **Golden Eval** — 代表入力と期待特性の固定セット。回帰検知の主手段
- **Provenance** — prompt / model id / dataset version / パラメータを記録し、再現可能にする
- **Regression on failure modes** — 過去に壊れた入力を評価セットへ残す
- **Cost / latency budget** — 品質向上がコスト爆発と引き換えになっていないか

## 技法ゲート

| 技法 | 適用条件 | 適用しない場合 |
|---|---|---|
| **Golden Eval set** | 出力品質が要件で、変更が繰り返される | 一度きりの実験 |
| **LLM-as-judge** | 人手評価がボトルネックで、judge自体を検証できる | 判定基準が機械的に書ける(その場合は決定的checkが安い) |
| **Statistical Review**(有意差の確認) | サンプル間の差が小さく、ノイズと区別が必要 | 差が明確に大きい |
| **Ablation** | 複数変更を同時に入れており、寄与が不明 | 変更が単一 |
| **Data provenance tracking** | 学習・評価データが更新される | 固定の公開データセット |

## 「正しさ」の定義

最低限、次を明示します。

- 比較対象のbaseline(いつ・どの構成で測ったか)
- 評価に使った入力集合と、その選定理由
- 許容できる劣化の範囲(全指標の同時改善は稀)
- 再現に必要な情報(model id、温度等のparameter、dataset version)

## コスト上の注意

評価の自動化はモデル呼び出しを伴うため、**Sentrithのcredit方針と直接ぶつかります**。

`docs/development/CREDIT_POLICY.md` の原則は変わりません。

- 評価は毎ターンではなく、**変更が品質に影響しうるときだけ**回す
- 小さなgolden setを常用し、大きなsetはrelease前に限定する
- 決定的checkで代替できる判定にLLM judgeを使わない

品質検証のためのモデル呼び出しは「無駄な追加呼び出し」ではありませんが、**予算の対象**です。
