<p align="right"><a href="README.en.md">English</a> ｜ <strong>日本語</strong></p>

# Architecture Documentation

このディレクトリには、**現在のシステム設計を理解するために実装時に参照する文書**だけを置きます。

ここに置くべきもの:

- system architecture
- component boundaries
- data flow
- deployment architecture
- runtime topology
- major interface relationships

ここに置かないもの:

- このAI開発テンプレート自体の思想
- 過去の議論履歴
- テンプレートの設計経緯
- 長い運用メモ

それらは `docs/meta/` に置きます。

通常の実装タスクでは、`docs/meta/` は検索・読込対象にしません。
テンプレート自体を変更・評価・監査するときだけ参照します。
