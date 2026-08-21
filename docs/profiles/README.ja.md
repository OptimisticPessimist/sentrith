<p align="right"><a href="README.en.md">English</a> ｜ <strong>日本語</strong></p>

# Engineering Profiles

Profileは分野固有のVerificationと技法ゲートを追加します。Sentrith Coreはvendor/tool neutralのままです。

- [Web / Backend](WEB_BACKEND.ja.md)
- [AI / ML](AI_ML.ja.md)
- [Data Science / Data Engineering](DATA.ja.md)
- [Game / Interactive 3D](GAME_INTERACTIVE_3D.ja.md)
  - [VRChat](VRCHAT.ja.md)

Profileは手法名より先に、解くべき問題を記述します。

---

## Profileは排他モードではない

Profileは「どれか1つを選ぶモード」ではなく、**加算的なoverlay**です。

1つのprojectで複数を有効化できます。実際、複合は普通です。

```text
RAG付きAPIサーバー      -> Web/Backend + AI/ML
特徴量pipeline付きML     -> AI/ML + Data
ゲームのbackend         -> Game/3D + Web/Backend
```

「Backend×AI/ML複合Profile」のような組み合わせ専用文書は**作りません**。
4分野で15通りの組み合わせが必要になり、ルールの重複と肥大化を招くためです。

## 合成規則

各Profileは次の3つだけを宣言します。

```text
1. 適用トリガー(どのファイル・どの種類の変更に効くか)
2. 追加するVerification次元
3. 技法ゲート(条件を満たしたときだけ適用する技法)
```

タスク実行時は:

1. 有効なProfileのうち、**トリガーに合致したものだけ**読む
2. 合致した全Profileの検証次元の**和集合**を適用する
3. 重複する項目(回帰テスト等)は一本化する

Profileは実装構造を指図せず検証を足すだけなので、原理的に衝突しにくい設計です。

矛盾したように見える場合は**厳しい方に従います**。

```text
correctness / safety
> explicit user requirement
> repository contract
> review policy
> cost optimization
```

## 有効なProfileはどこに書くか

`docs/ai/PROFILE.md` に、有効化したProfileとトリガーの索引だけを置きます。

技法の解説は書きません。解説はこのdirectoryの各Profileにあります。

`PROFILE.md` は毎タスクのcontextに載るため、**短く保つことが要件**です(目安100行以下)。

初期化は `docs/ai/BOOTSTRAP.md` のProfile質問で行います。
