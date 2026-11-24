> Latest translations / 翻訳の最新版: [0-LATEST-TRANSLATIONS.md](0-LATEST-TRANSLATIONS.md)

# aruaru.pro

**日本語** | [English](#english)

---

## 日本語

### これは何?(子どもにもわかる説明)

**「得意なことを、ほしい人に届ける」ためのサイト**です。

- 絵がかける人、文章がうまい人、プログラムが作れる人が、自分の「できること」を **出品** します。
- 困っている人が、それを **買って(注文して)** お願いします。
- お仕事やアルバイトの **求人** も、同じサイトでさがせます。
- IT の勉強がしたい人には、**無料の研修つきの転職サポート** のコーナーもあります。

ココナラ([coconala.com](https://coconala.com/))のような「スキルのお店」と「求人サイト」を、ひとつにしたものです。
目標は **手数料をとても安く** すること。今のしくみでは、お金の 5% だけをサイトの手数料にしています。

### 専門家向けの説明

| 項目 | 内容 |
|---|---|
| 言語・基盤 | Rust + [RPoem](https://github.com/aon-co-jp/RPoem)(Poem 互換ファサード) |
| DB | [aruaru-db](https://github.com/aon-co-jp/aruaru-db)(Git-on-SQL、PostgreSQL 互換) |
| 画面(SSR) | [RS-React](https://github.com/aon-co-jp/RS-React) + RS-HTML(`App::mount` → `render_to_html`) |
| JSON | [RS-JSON](https://github.com/aon-co-jp/RS-JSON)(サイズ・深さ・重複キーの上限つき) |
| 決済 | Stripe Connect(Express アカウント、`application_fee_amount` 5% + `transfer_data.destination`) |
| 認証 | ロール 4 種(seller / buyer / job_seeker / recruiter)、Bearer トークン + DB の Cookie セッション、Google / Facebook / X の OAuth 2.0(X は PKCE) |
| 状態 | すべて DB に保存(再起動・複数インスタンスでも動く設計) |

### 今できること(2026-11-19 時点)

- `GET /healthz`、`GET /categories`、`GET /`(カテゴリ一覧の SSR ページ)
- 出品: `POST/GET /services`、`GET /services/:id`
- 注文: `POST /services/:id/orders`、`POST /orders/:id/transition`
- レビュー(完了済み注文の買い手のみ): `POST/GET /services/:id/reviews`
- 求人: `POST/GET /jobs`、`GET /jobs/:id`
- 無料 IT 研修つき転職エージェント: `POST/GET /career-agent-programs`、`GET /career-agent-programs/:id`
- 認証: `POST /auth/register`、`POST /auth/logout`、`GET /auth/oauth/:provider/start|callback`
- 決済: `POST /sellers/:seller_name/stripe/onboarding`、`POST /orders/:order_id/checkout`、`POST /stripe/webhook`
- `cargo test` 48 件すべて成功
- VPS(`127.0.0.1:8143`、専用 DB つき)で稼働中。登録・出品・注文・レビュー・求人・研修・ログアウトを実際に通して確認済み

### 正直なお知らせ(まだできていないこと・ご注意)

1. **これは試作品(実験)です。** 本番の公開はまだです。
2. **Google・Facebook・X のログイン、Stripe の決済は、実際のアカウント・実際の Stripe では未検証**です(URL 作成・署名検証・金額計算などは単体テストで確認済み)。
3. **公開用のドメイン設定が未完了**です。`aruaru.pro` に A レコード(IPv4)がまだありません(ネームサーバーは ConoHa DNS)。
4. **API の形が REST 風**です。エコシステムの方針(REST 全廃・Connect へ)に合わせて、あとで Connect に移します。新しい REST は増やしません。
5. X(旧 Twitter)はメールアドレスを返さないため、`email` が空のまま登録されることがあります。

### 動かしてみる(開発者向け)

```bash
# 必要: Rust、aruaru-db(PostgreSQL 互換ポート)。兄弟リポジトリ RPoem / RS-JSON / RFrontEnd / aruaru-db を F:\runo\repository 配下に置く
export ARUARU_PRO_DSN="host=127.0.0.1 port=5433 user=app password=secret dbname=app"
export ARUARU_PRO_BIND="127.0.0.1:8081"
export ARUARU_PRO_PUBLIC_URL="https://aruaru.pro"
# 任意: STRIPE_SECRET_KEY / STRIPE_WEBHOOK_SECRET / {GOOGLE,FACEBOOK,X}_CLIENT_ID / _CLIENT_SECRET
CARGO_BUILD_JOBS=2 cargo run --release
```

秘密(キー・シークレット)は **コードや README に書かず**、環境変数やサーバー上の権限 600 のファイルに置いてください。
`async-stripe` のコンパイルはメモリを多く使うので、`CARGO_BUILD_JOBS=2` を勧めます。

aruaru-db の SQL は PostgreSQL より小さく、`ALTER TABLE`・`ORDER BY`・`LIMIT`・条件の `AND` 連結は使えません。詳しくは [`CLAUDE.md`](CLAUDE.md) を見てください。

OAuth のリダイレクト先(各サービスの管理画面に登録する URL):

| サービス | リダイレクト URI |
|---|---|
| Google | `https://aruaru.pro/auth/oauth/google/callback` |
| Facebook | `https://aruaru.pro/auth/oauth/facebook/callback` |
| X | `https://aruaru.pro/auth/oauth/x/callback` |

### 関連プロジェクト

- [RPoem](https://github.com/aon-co-jp/RPoem) — サーバー側の土台
- [RS-React](https://github.com/aon-co-jp/RS-React) — 画面のしくみ
- [aruaru-db](https://github.com/aon-co-jp/aruaru-db) — データベース
- [RS-JSON](https://github.com/aon-co-jp/RS-JSON) — JSON の読み書き
- [RFrontEnd](https://github.com/aon-co-jp/RFrontEnd) — RS-React などの親リポジトリ
- [open-raid-z](https://github.com/aon-co-jp/open-raid-z) — 開発ルールの正本

詳しい開発方針・経緯は [`CLAUDE.md`](CLAUDE.md)、引き継ぎ・お引越し手順は [`PORTING.md`](PORTING.md) を見てください。

---

## English

### What is this? (so simple that a child can understand)

**It is a website where people can share the things they are good at.**

- Some people draw pictures. Some write well. Some can make computer programs. They **put up** what they can do.
- Other people need help. They **order** it.
- You can also look for **jobs and part-time work** on the same site.
- If you want to learn computers, there is a corner for **free IT training and help finding a job**.

It joins two ideas into one: a "skill shop" like Coconala ([coconala.com](https://coconala.com/)) and a job site.
Our goal is **very low fees**. Right now the site keeps only 5% of each payment.

### For experts

| Item | Details |
|---|---|
| Language and runtime | Rust + [RPoem](https://github.com/aon-co-jp/RPoem) (a Poem-compatible layer) |
| Database | [aruaru-db](https://github.com/aon-co-jp/aruaru-db) (Git-on-SQL, PostgreSQL-compatible) |
| Screens (SSR) | [RS-React](https://github.com/aon-co-jp/RS-React) + RS-HTML (`App::mount` → `render_to_html`) |
| JSON | [RS-JSON](https://github.com/aon-co-jp/RS-JSON) (limits on size, depth and duplicate keys) |
| Payments | Stripe Connect (Express accounts, `application_fee_amount` of 5% + `transfer_data.destination`) |
| Login | Four roles (seller / buyer / job_seeker / recruiter), a Bearer token plus a cookie session stored in the DB, and Google / Facebook / X OAuth 2.0 (X uses PKCE) |
| State | Everything lives in the database, so it survives restarts and works with several copies of the server |

### What works today (2026-11-19)

- `GET /healthz`, `GET /categories`, `GET /` (the category page, built on the server)
- Listings: `POST/GET /services`, `GET /services/:id`
- Orders: `POST /services/:id/orders`, `POST /orders/:id/transition`
- Reviews (only the buyer of a finished order can write one): `POST/GET /services/:id/reviews`
- Jobs: `POST/GET /jobs`, `GET /jobs/:id`
- Career-agent programs with free IT training: `POST/GET /career-agent-programs`, `GET /career-agent-programs/:id`
- Login: `POST /auth/register`, `POST /auth/logout`, `GET /auth/oauth/:provider/start|callback`
- Payments: `POST /sellers/:seller_name/stripe/onboarding`, `POST /orders/:order_id/checkout`, `POST /stripe/webhook`
- All 48 `cargo test` tests pass.
- It runs on our VPS (`127.0.0.1:8143`, with its own database). We really called the APIs there: sign-up, listing, order, review, job, training program and logout all worked.

### Honest notes: what is not done yet

1. **This is a prototype (an experiment).** It is not open to the public yet.
2. **Google, Facebook and X login, and Stripe payments, have not been tried with real accounts.** We only tested the URLs, the signature check and the fee math with small tests. For Google we confirmed that the sign-in link is built correctly.
3. **The public address is not ready.** `aruaru.pro` has no A (IPv4) record yet. Its name servers are ConoHa DNS.
4. **The API looks like REST.** Our team is moving everything from REST to Connect, so these routes will move later. We will not add new REST routes.
5. X (the old Twitter) does not give us an email address, so the email may stay empty.

### Try it (for developers)

```bash
# You need Rust and aruaru-db (it speaks the PostgreSQL protocol).
# Put the sibling repos RPoem, RS-JSON, RFrontEnd and aruaru-db under F:\runo\repository.
export ARUARU_PRO_DSN="host=127.0.0.1 port=5433 user=app password=secret dbname=app"
export ARUARU_PRO_BIND="127.0.0.1:8081"
export ARUARU_PRO_PUBLIC_URL="https://aruaru.pro"
# Optional: STRIPE_SECRET_KEY, STRIPE_WEBHOOK_SECRET,
#           {GOOGLE,FACEBOOK,X}_CLIENT_ID and {GOOGLE,FACEBOOK,X}_CLIENT_SECRET
CARGO_BUILD_JOBS=2 cargo run --release
```

**Never put secrets in code or in this README.** Keep them in environment variables, or in a file on the server that only you can read (mode 600).
Building `async-stripe` uses a lot of memory, so we suggest `CARGO_BUILD_JOBS=2`.

aruaru-db speaks a smaller SQL than PostgreSQL (no `ALTER TABLE`, `ORDER BY`, `LIMIT`, or `AND` between conditions). See [`CLAUDE.md`](CLAUDE.md) for the details.

OAuth redirect addresses (add these in each provider's console):

| Provider | Redirect URI |
|---|---|
| Google | `https://aruaru.pro/auth/oauth/google/callback` |
| Facebook | `https://aruaru.pro/auth/oauth/facebook/callback` |
| X | `https://aruaru.pro/auth/oauth/x/callback` |

### Related projects

- [RPoem](https://github.com/aon-co-jp/RPoem) — the server base
- [RS-React](https://github.com/aon-co-jp/RS-React) — how the screens are built
- [aruaru-db](https://github.com/aon-co-jp/aruaru-db) — the database
- [RS-JSON](https://github.com/aon-co-jp/RS-JSON) — reading and writing JSON
- [RFrontEnd](https://github.com/aon-co-jp/RFrontEnd) — the parent repository of RS-React and friends
- [open-raid-z](https://github.com/aon-co-jp/open-raid-z) — where our development rules live

For the rules and history, read [`CLAUDE.md`](CLAUDE.md). To hand the work over or move it, read [`PORTING.md`](PORTING.md).
