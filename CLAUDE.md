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

## Rust自前実装エコシステムの優先利用(ユーザー指示、2026-09-13)

「実際にRust版が完成していれば、日頃の開発で使用して」という指示を
受け、**aruaru.proの今後の実装では、`RFrontEnd`傘下のRust自前実装
(既存実装コードを流用しない再開発)を優先的に使う**方針とする
(既存の他プロジェクト——audiocafe-tokyo等——への横断的な入れ替えは
対象外、ユーザー確認済み)。

- JSON処理 → [RS-JSON](https://github.com/aon-co-jp/RS-JSON)
  (`serde_json`を直接呼ぶ代わりに使う、`job-site`が既にこのパターン)
- HTML生成(SSR) → [RS-HTML](https://github.com/aon-co-jp/RS-HTML)
- CSS/スタイル計算 → [RS-CSS](https://github.com/aon-co-jp/RS-CSS)
- CSSフレームワーク相当(グリッド/基本コンポーネント) →
  [RS-BootStrap](https://github.com/aon-co-jp/RS-BootStrap)
- フロントエンド(コンポーネントモデル) →
  [RS-React](https://github.com/aon-co-jp/RS-React)
- GraphQL API(将来REST以外のAPIが必要になった場合) →
  [RS-GraphQL](https://github.com/aon-co-jp/RS-GraphQL)

これらのいずれかが該当機能に対してまだ未成熟(例: RS-Reactの`use_effect`
未実装、RS-CSSにCSSテキストシリアライザが無い等)な場合は、各リポジトリ
のCLAUDE.mdの「次にすべきこと」を先に完成させてから使う(既存方針
「投げやり・その場しのぎ禁止」に沿う)。

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
- **RS-React**: フロント基盤として使う想定。2026-09-13、`App::mount`+
  `render_to_html`によるSSR最小UI(カテゴリ一覧、`src/page.rs`)を実際に
  接続・動作確認済み(下記「現状」参照)。ただし`App::tick`(状態変化への
  追従・再レンダー)はこのページではまだ使っていない——カテゴリ一覧が
  現時点でクライアント操作を持たないため。

## Rust自前実装の実装ノウハウについて(ユーザー指示、2026-09-13)

「RS-などのRust版は、Rust+TauriやtokioなどRPoemでの実装時のノウハウを
上手く取り入れて」という指示があった。RPoem(`open-runo-poem-compat`
等)の開発で確立された非同期実行基盤・エラー処理・機能フラグ設計等の
パターンを、RS-HTML/RS-CSS/RS-React等のRust自前実装(および本リポジトリ
自身)の実装時に積極的に参考にする方針とする。具体例:
- `dom_bridge`のようなoptional feature設計(依存を必須にしない、
  RPoem側の各クレートが機能フラグでオプトインさせる設計と同じ考え方)。
- エラー型の扱い(`anyhow`での集約、呼び出し元での分類——job-siteの
  `aruaru_db_connector::Error`variant分岐と同じパターン)。
- 今後Wasmハイドレーション(RS-TypeScript/RS-JavaScript経由)を実装する
  際は、RPoemの「サーバー側実行基盤とブラウザ側実行環境は役割が異なる」
  という層の整理(`RFrontEnd/CLAUDE.md`参照)を踏襲する。

## 現状(2026-09-13)

- `Cargo.toml`: `open-runo-poem-compat`(RPoem)・`aruaru-db-connector`・
  `rust-json`(RS-JSON)・`rs-react`(`dom_bridge`フィーチャ有効)への
  path依存。
- `src/main.rs`: `GET /healthz`・`GET /categories`(JSON)・`GET /`
  (RS-ReactによるSSRカテゴリ一覧、下記参照)。出品・注文・エスクロー・
  決済・レビュー・チャット等は未着手。
- `src/categories.rs`: カテゴリマスタ(ユーザー提示のcoconala機能LISTを
  そのまま流用、「デザイン制作」は子カテゴリ21件を持つ)。テスト2件
  green。
- **`src/page.rs`(2026-09-13新設)**: RS-Reactの`App::mount`でカテゴリ
  一覧コンポーネントを初回render→`rs_react::render_to_html`でHTML文字列化
  する最小のSSRページ。RS-HTML側で新設した`serialize_node`公開・
  RS-React側で新設した`render_to_html`/`vnode_to_node`公開により実現
  (両方2026-09-13にこの一連の作業で追加)。テスト2件green
  (全カテゴリがHTMLに現れる・デザイン制作の子カテゴリが入れ子の`<ul>`で
  現れる)。一時的な`cargo run --example print_page`で実際のHTML出力を
  目視確認済み(確認後にexampleファイルは削除、動作確認用の一時ファイル
  だったため)。`App::tick`によるインタラクティブな再レンダーはまだ
  使っていない(カテゴリ一覧は現時点で静的)。
- **`src/services.rs`+`src/reviews.rs`(2026-09-13新設)**: 出品(Service)・
  レビュー(Review)のCRUD第一段。`job-site`のjobs実装パターン(RS-JSON
  デコード・`AruaruDb::commit`によるGit-on-SQLバージョン管理)を踏襲。
  - `POST /services`(作成/更新+commit)・`GET /services`(一覧)・
    `GET /services/:id`(単体取得)。
  - `POST /services/:id/reviews`(作成、対象serviceの実在確認込み)・
    `GET /services/:id/reviews`(一覧)。
  - **実バグを未然に回避**: `job-site`のid生成(`slugify`を`--`連結)を
    そのまま転用すると、`slugify`が英数字以外を除去するため**日本語の
    出品者名・タイトルは軒並み空文字に潰れ、異なる出品が同じidへ
    衝突する**(このサービスは日本語が主言語のため、job-siteの前提
    ——英語のテストデータ——がそのまま当てはまらない)。`services::
    make_service_id`でハッシュ(`DefaultHasher`)を必ず付与する設計に
    変更し、テストで日本語名の非衝突・決定性を確認済み。**job-site
    自体は別件としてそのまま残す方針(2026-09-13確認済み)のため、
    今回はaruaru.pro側だけを直し、job-site側のバグ修正は対象外**
    (別セッションで扱うべき指摘として認識——直接ユーザーに実行を
    依頼していないため、今回は変更していない)。
  - テスト10件追加(id生成の非衝突/決定性/ASCII時の読みやすいslug、
    出品/レビューのvalidate関数)、`cargo test`14件全green・警告0件。
- `cargo build`/`cargo test`: 通過確認済み(このリポジトリ自体の警告0件、
  RPoem側の既存warning 3件は対象外)。

## 2026-09-13 続き: 1〜5の実装完了+IT研修付き転職エージェント新設

ユーザー指示「1から5の順番で実装して」に基づき、以下を全て実装済み。

1. **注文(Order)モデル**(`src/orders.rs`): `POST /services/:id/orders`・
   `POST /orders/:id/transition`(pending→completed/cancelledのみ許可)。
   `reviews::create_review`は`orders::order_is_completed_by`で
   「その出品への完了済み注文の買い手本人か」を確認するようになった。
2. **Stripe Connect**(`src/stripe_connect.rs`): `async-stripe`0.41.0
   (`connect`+`checkout`+`runtime-tokio-hyper-rustls-webpki`feature)。
   `POST /sellers/:seller_name/stripe/onboarding`(Express Connect
   アカウント作成+Onboardingリンク)・`POST /orders/:order_id/checkout`
   (`application_fee_amount`+`transfer_data.destination`による手数料
   付きCheckout Session)。手数料率5%(定数`PLATFORM_FEE_PERCENT`)。
   **正直な開示**: テストAPIキーが無いため実際のAPI呼び出しは実機
   未検証(コンパイル通過+金額計算ロジックのテストのみ)。
3. **求人・アルバイト情報の専用データモデル**(`src/job_listings.rs`):
   `services`テーブルとは別の`job_listings`テーブル(勤務地・時給・
   雇用形態)。雇用形態は固定一覧(正社員/契約社員/アルバイト/パート/
   業務委託/派遣)。
4. **認証**(`src/auth.rs`): `POST /auth/register`でBearerトークン即発行
   (seller/buyer/job_seeker/recruiterの4ロール)。出品・注文・レビュー・
   求人作成の各ハンドラに`authenticate`+`require_role`+
   `require_name_matches`(なりすまし防止)を組み込み。**正直な開示**:
   パスワード・OAuth・メール確認は無い第一段の最小実装。
5. **カテゴリマスタのDBテーブル化**(`src/categories.rs`):
   `categories`テーブル(name/parent_name/position)をコンパイル時定数
   `CATEGORIES`から起動時にシード(`ensure_table_and_seed`、
   `ON CONFLICT DO NOTHING`で冪等)。`GET /categories`・`GET /`
   (SSRページ)ともDBから読むようになった。出品作成時のカテゴリ検証は
   `category_is_known_statically`(高速・I/O無し)→`category_exists_in_db`
   (フォールバック)の2段構成にし、既存の同期ユニットテストを保ちながら
   将来の管理者によるカテゴリ追加(再デプロイ無し)にも対応できるように
   した。

**追加(ユーザー指示、2026-09-13)**: 「正社員求人や無料のIT研修付き
無料の転職エージェントサービスコーナーも作って」。正社員求人自体は
上記3の`job_listings`(`employment_type = "正社員"`)で既にカバー済み
のため、新設したのは「無料IT研修+就職支援」という異なるサービス形態
(`src/career_agent_programs.rs`、`career_agent_programs`テーブル:
`agent_name`/`program_name`/`description`/`training_weeks`/
`target_job_type`/`is_free_for_trainee`)。`POST/GET
/career-agent-programs`・`GET /career-agent-programs/:id`を実装、
認証は`recruiter`ロールを流用(専用ロールは新設せず既存の役割分担に
収めた)。カテゴリマスタに「IT研修付き転職エージェント」を追加。

テスト合計38件全green・警告0件(services.rsのカテゴリ検証テストは
`validate_upsert`→`validate_upsert_fields`+`category_is_known_statically`
へ分割、既存テストの意図は保持)。

## 2026-09-13 続き2: Google/Facebook/X OAuth 2.0ログインを追加

ユーザー指示「認証はパスワード/OAuth無しの最小実装 GmailやFacebookや
XのOAuth認証は実装して」に基づき`src/oauth.rs`を新設。

- `GET /auth/oauth/:provider/start?role=<role>`(`provider`は
  `google`/`facebook`/`x`) → 各プロバイダの認可URLを`{"authorize_url":
  ...}`で返す(302直接リダイレクトではない——`open_runo_poem_compat`側に
  リダイレクト応答ヘルパーが無いため、呼び出し側がこのURLへ遷移する
  形。将来ヘルパーが整備されれば差し替え可能)。CSRF対策の`state`
  (Xのみ必要なPKCE `code_verifier`/`code_challenge`込み)をプロセス内
  メモリ(`OnceLock<Mutex<HashMap<..>>>`)で保持。
- `GET /auth/oauth/:provider/callback?code=...&state=...` →
  `code`をアクセストークンに交換→各プロバイダのuserinfo APIで
  `(subject, name, email)`取得→`(oauth_provider, oauth_subject)`で
  既存ユーザー検索、無ければ新規作成→自前のBearerトークンを発行して
  返す(`auth::register`と同じトークン形式、既存の`users`テーブルに
  `email`/`oauth_provider`/`oauth_subject`列を追加、既存DBへの
  `ALTER TABLE ADD COLUMN IF NOT EXISTS`で追従)。
- 依存クレート追加: `reqwest`(json/form/query feature、HTTPクライアント)・
  `sha2`(PKCE code_challenge用のSHA-256)。
- **正直な開示**: (1) state/PKCEのプロセス内メモリ保持はCookie
  セッションが無い簡易実装で、プロセス再起動・マルチインスタンス構成
  では機能しない。(2) X(旧Twitter)のuserinfo APIは既定でメール
  アドレスを返さないため`email`が`None`のまま登録されることがある。
  (3) 実際のOAuthアプリ登録(client_id/secret)がこのセッションの環境に
  無いため、実際のGoogle/Facebook/Xアカウントでのログイン往復は
  未検証——URL構築・PKCE生成(既知テストベクタで裏取り済み)・
  パーセントエンコードのロジックのみユニットテストで確認。

テスト5件追加、`cargo test`43件全green・警告0件。

## 次にすべきこと

1. Stripe Webhook(決済完了通知→注文の`completed`遷移の自動化、
   現状は`POST /orders/:id/transition`を手動で呼ぶ想定)。
2. StripeのテストモードAPIキーでの実機検証(オンボーディング〜
   チェックアウトの一連の流れを実際に動かして確認)。
3. Google/Facebook/X各プロバイダで実際にOAuthアプリを登録し
   (`GOOGLE_CLIENT_ID`/`GOOGLE_CLIENT_SECRET`等の環境変数設定)、
   実機でログイン往復を検証する。
4. `career_agent_programs`の収益モデル(エージェント企業からの成約
   報酬等)の検討——現状は掲載機能のみで決済との接続は無い。
5. インタラクティブなUI(カテゴリ絞り込み検索等)が必要になった時点で
   RS-Reactの`App::tick`(状態変化への追従・再レンダー)を実際に使う
   ——現状のカテゴリ一覧ページはまだ静的なSSRのみ。
6. state/PKCEのプロセス内メモリ保持をCookieセッション or
   DBバックエンドへ置き換える(マルチインスタンス構成対応)。

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
