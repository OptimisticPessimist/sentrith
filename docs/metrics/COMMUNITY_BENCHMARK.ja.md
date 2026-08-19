<p align="right"><a href="COMMUNITY_BENCHMARK.en.md">English</a> ｜ <strong>日本語</strong></p>

# Community Benchmark

## 目的

Sentrithがusageを削減することを「作者の主張」で終わらせず、有志ユーザーの実測値で検証します。

## Contribution

```bash
sentrith usage contribute --agent copilot --model "<model>"
```

標準qualification:

- baseline >= 10 tasks
- Sentrith >= 10 tasks
- success結果あり
- 両phaseに同じnative usage metricあり

5+5など小さいsampleは`--force`でexperimental contributionにできますが、標準aggregateから除外します。

## Privacy

Contribution JSONには集計結果のみを書きます。

送らないもの:

- task label
- prompt
- repository / customer / company
- paths
- source code
- transcript
- session ID
- Git remote

## Aggregation

```bash
sentrith usage aggregate
sentrith usage aggregate --publish
```

標準集計はqualified contributionのみ。

Cross-providerではAI Credits/USD/tokensを直接合算せず:

```text
normalized_usage_change_pct
```

の中央値を使います。

## README表示

少なくとも:

- contributions
- baseline task count
- Sentrith task count
- median normalized usage / successful task
- community-reportedであること

を明示します。

Maintainer-controlled benchmarkとCommunity Benchmarkは分離します。
