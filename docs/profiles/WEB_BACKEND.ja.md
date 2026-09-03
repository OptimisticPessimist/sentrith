<p align="right"><a href="WEB_BACKEND.en.md">English</a> ｜ <strong>日本語</strong></p>

# Web / Backend Engineering Profile

Web / Backend Profileは、**contract・状態・権限**が壊れると外部から観測される種類の変更を扱います。

## 適用トリガー

次に該当するタスクでのみ、このProfileを読みます。

```text
HTTP/RPC/GraphQLのendpointやschema
DB schema / migration
認証・認可・session
外部サービス連携
background job / queue / scheduler
料金・請求・在庫など不可逆な状態遷移
```

該当しないタスク(UI文言修正、内部refactorなど)では読み込みません。

## 追加するVerification次元

- **Contract Verification** — request/response schema、status code、error shape、後方互換
- **State Verification** — migrationの前後不変条件、rollback可否
- **AuthZ Verification** — 権限境界ごとの期待レスポンス(200 / 401 / 403 / 404)
- **Concurrency Verification** — 競合更新、冪等性、リトライ安全性
- **Boundary Verification** — 入力検証、境界値、エラー経路

Unit testだけで足りることは少なく、**contract testとintegration test**を優先します。

## 技法ゲート

技法は「使うこと」が目的ではありません。**条件を満たしたときだけ**適用します。

| 技法 | 適用条件 | 適用しない場合 |
|---|---|---|
| **DDD Lite**(境界づけられた文脈と用語の統一) | ドメイン用語が実装とズレている / 同じ概念が複数名称で存在する | CRUDが素直に対応している |
| **DDD Full**(Aggregate / Invariantの明示) | 不変条件が複数エンティティにまたがり、壊れると業務的に致命的 | 不変条件が単一テーブル制約で表現できる |
| **Ports & Adapters** | 外部サービス依存をテストで差し替える必要がある | 依存が1つでmock不要 |
| **CQRS** | 読み取りと書き込みの要求(スケール・モデル形状)が明確に食い違う | 単純に同じモデルで足りる |
| **Event Sourcing** | 「なぜ今の状態になったか」の履歴自体が要件 | 監査ログで足りる |
| **Threat Modeling** | 認証・認可・秘密情報・課金・PIIに触れる | 影響が内部限定 |
| **Property-Based Testing** | parser / serializer / 計算 / 変換で入力空間が広い | 代表例で十分に網羅できる |

判断に迷う場合は**適用しない**を選び、必要になった時点で昇格します。

過剰な技法適用は、Sentrithが避けようとしているprocess overheadそのものです。

## 「正しさ」の定義

```text
コードが動く
≠
contractが守られている
```

最低限、次を観測可能な形で定義します。

- 既存clientが壊れないこと(または破壊的変更として明示されていること)
- 権限のない主体が権限のある操作をできないこと
- migrationが既存行を保全すること
- 失敗時に部分適用された状態が残らないこと

## Safety Gatesとの関係

このProfileの対象は、`docs/development/SAFETY_GATES.md` の高影響操作と重なります。

Profileは検証を**足す**だけで、Hard Gateを**緩めません**。

矛盾した場合は厳しい方に従います。
