# 開発方針＆開発環境ルール(aruaru.pro)

作業ドライブは`F:\runo`(サブディレクトリ`repository\aruaru.pro`)。
この節は[`open-raid-z`](https://github.com/aon-co-jp/open-raid-z)の
`CLAUDE.md`を正本とし、各プロジェクトへコピーして同期する既存の運用
ルール継承方針に準じる。

## このリポジトリの役割(2026-09-13新設)

コンコナラ([coconala.com](https://coconala.com/))型のスキルシェア
マーケットプレイスと、求人・アルバイト情報サイトを統合した新規サービス。
既存サービスより手数料を大幅に安く提供することを目標とする(ユーザー
指示、2026-09-13)。

ドメイン`aruaru.pro`(ユーザーが取得済み)を活かした命名。

## 技術スタック(ユーザー指示、2026-09-13)

- **バックエンド**: Rust + [RPoem](https://github.com/aon-co-jp/RPoem) +
  [aruaru-db](https://github.com/aon-co-jp/aruaru-db)。
- **フロントエンド**: [RS-React](https://github.com/aon-co-jp/RS-React)
  (旧`RReact`)。ユーザー指示「Rust＋RPoem+RReactなどをベースにして」
  (2026-09-13)。
- **決済**: Stripe Connect(既存決済プラットフォームを利用する方針、
  独自の資金移動システムは実装しない——法令対応の観点から)。

## スコープ(ユーザー指示、2026-09-13)

- コンコナラの主要カテゴリを幅広くカバーするフルスコープ(1〜2カテゴリの
  MVPではなく、当初からフルスコープで進める方針をユーザーが選択)。
- 求人・アルバイト情報サイトを統合する(コンコナラ型スキルマーケット
  プレイス単体ではなく、両方を1つのサービスにまとめる)。
- カテゴリマスタは、ユーザーが提示したcoconalaの機能LISTをそのまま
  初期データとして流用する(推測で埋めていない、`src/categories.rs`
  参照)。「求人・アルバイト情報」を追加のトップレベルカテゴリとして
  新設。

## 既存プロジェクトとの関係

- **`job-site`**(`F:\runo\repository\job-site`、RPoem+aruaru-dbの結線
  検証用試作品、リモート未push)とは別件として現状のまま残す
  (ユーザー指示、2026-09-13:「別件として現状場所に残し、aruaru.proは
  新規リポジトリでゼロから」)。ただし本リポジトリの`src/main.rs`は
  job-siteのRPoem+aruaru-db結線パターン(リクエストボディのRS-JSON
  デコード、`AruaruDb::connect`等)を参考にしている。
- **RS-React**: フロント基盤として使う想定。2026-09-13時点で関数
  コンポーネント+`use_state`フックまで実装済みだが、「ツリー全体の
  再レンダーループ」(dirtyなコンポーネントだけ再render→diff→
  `apply_patch`する「アプリループ」自体)がまだ無い——本格的なSPA/SSR
  ハイドレーションに使うには、まずRS-React側でこれを実装する必要がある。

## 現状(2026-09-13リポジトリ新設)

- `Cargo.toml`: `open-runo-poem-compat`(RPoem)・`aruaru-db-connector`・
  `rust-json`(RS-JSON)へのpath依存。
- `src/main.rs`: `GET /healthz`・`GET /categories`のみ。出品・注文・
  エスクロー・決済・レビュー・チャット等は未着手。
- `src/categories.rs`: カテゴリマスタ(ユーザー提示のcoconala機能LISTを
  そのまま流用、「デザイン制作」は子カテゴリ21件を持つ)。テスト2件
  green(カテゴリ名の重複無し、デザイン制作の子カテゴリ確認)。
- `cargo build`/`cargo test`: 通過確認済み。

## 次にすべきこと

1. **RS-React側の「アプリループ」実装**(RS-React CLAUDE.mdの「次にすべき
   こと」参照)——これが無いとフロントの本格実装に進めない。
2. データモデル設計: 出品(Service)・注文(Order)・レビュー・出品者
   プロフィール・カテゴリツリー(現状は静的定数、DBテーブル化が必要)。
3. Stripe Connectオンボーディング(Standard/Express)・
   `application_fee_amount`によるプラットフォーム手数料の実装方針決定
   (Rust向けstripe SDKクレートの選定含む)。
4. 求人・アルバイト情報カテゴリの専用データモデル(スキル出品とは
   フィールドが異なる——勤務地・時給・雇用形態等)の検討。
5. 認証(出品者/依頼者/求職者/採用担当のロール分け)。

## 関連プロジェクト

- [RPoem](https://github.com/aon-co-jp/RPoem) — サーバー側実行基盤
- [RS-React](https://github.com/aon-co-jp/RS-React) — フロントエンド(コンポーネントモデル)、2026-09-13にRReactから改称
- [aruaru-db](https://github.com/aon-co-jp/aruaru-db) — データベース
- [RS-JSON](https://github.com/aon-co-jp/RS-JSON) — JSONデコード
- [RFrontEnd](https://github.com/aon-co-jp/RFrontEnd) — RS-React等の親リポジトリ
- [job-site](F:\runo\repository\job-site) — RPoem+aruaru-db結線検証用試作品(別件、リモート未push)
- [open-raid-z](https://github.com/aon-co-jp/open-raid-z) — 開発ルールの正本

## HANDOFF

- **2026-09-13 リポジトリ新設**: GitHub `aon-co-jp/aruaru.pro`作成、
  ローカル`F:\runo\repository\aruaru.pro`に最小骨格(healthz+categories
  API)を実装し`cargo build`/`cargo test`(2件green)で検証。次回セッション
  では「次にすべきこと」1(RS-React側のアプリループ)から着手すること。
