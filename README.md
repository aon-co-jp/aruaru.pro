# aruaru.pro

コンコナラ([coconala.com](https://coconala.com/))型のスキルシェア
マーケットプレイスと、求人・アルバイト情報サイトを統合した新規サービス。
既存サービスより手数料を大幅に安く提供することを目標とする。

ドメイン`aruaru.pro`(取得済み)を活かした命名。`aon-co-jp`エコシステム内の
`aruaru`系列(aruaru-tokyo・aruaru-db・aruaru-llm)とは別サービスだが、
名称の系譜は引き継ぐ。

## 技術スタック

- **バックエンド**: Rust + [RPoem](https://github.com/aon-co-jp/RPoem)
  (Poem互換ファサード) + [aruaru-db](https://github.com/aon-co-jp/aruaru-db)
  (Git-on-SQLバージョン管理付きPostgres互換DB)
- **フロントエンド**: [RS-React](https://github.com/aon-co-jp/RS-React)
  (Rust製React、関数コンポーネント+hooks実装済み。ツリー全体の
  再レンダーループは未着手)
- **決済**: Stripe Connect(出品者オンボーディング・プラットフォーム
  手数料・エスクロー的な支払いフロー、詳細は`CLAUDE.md`参照)

## 現状(2026-09-13リポジトリ新設)

`job-site`(`aon-co-jp`の別リポジトリ、RPoem+aruaru-dbの結線検証用試作品)
の構成を踏襲した最小の起動骨格のみ。

- `GET /healthz`
- `GET /categories`(カテゴリマスタ一覧、coconalaの機能LISTを初期データ
  としてそのまま流用+「求人・アルバイト情報」を追加)

出品・注文・エスクロー・Stripe Connect決済・レビュー・チャット等の
実装は未着手。詳細な開発方針・ロードマップは[`CLAUDE.md`](CLAUDE.md)参照。

## 関連プロジェクト

- [RPoem](https://github.com/aon-co-jp/RPoem) — サーバー側実行基盤
- [RS-React](https://github.com/aon-co-jp/RS-React) — フロントエンド(コンポーネントモデル)
- [aruaru-db](https://github.com/aon-co-jp/aruaru-db) — データベース
- [RFrontEnd](https://github.com/aon-co-jp/RFrontEnd) — RS-React等の親リポジトリ
- [open-raid-z](https://github.com/aon-co-jp/open-raid-z) — 開発ルールの正本
