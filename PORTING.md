# PORTING.md — 引き継ぎ・お引越し / Hand-over and moving

**日本語** | [English](#english)

---

## 日本語

### このファイルの目的

別のアカウント・別のパソコンで、この作業をそのまま続けられるようにするための手順書です。
Claude の記憶は引き継がれない前提で、必要なことをすべてここに書きます。

### 場所と前提

- リポジトリ: <https://github.com/aon-co-jp/aruaru.pro>(ローカルは `F:\runo\repository\aruaru.pro`)
- 兄弟リポジトリ(path 依存): `RPoem`・`RS-JSON`・`RFrontEnd`(RS-React / RS-HTML)・`aruaru-db` を `F:\runo\repository` 直下に置く
- Rust は PATH に `%USERPROFILE%\.cargo\bin` を足す
- ビルドは `CARGO_BUILD_JOBS=2`(`async-stripe` の `webhook-events` でメモリが足りなくなるため)
- 秘密(OAuth・Stripe のキー)は **`F:\` 直下の txt に置き、画面・コミット・README に出さない**。例: `F:\google-oauth.txt`(1 行目=クライアント ID、2 行目=シークレット。書式は確認済み)

### いまの到達点(2026-11-19)

| 項目 | 状態 |
|---|---|
| コード | 出品・注文・レビュー・求人・転職エージェント・認証(OAuth/Cookie)・Stripe Connect・Webhook まで実装済み、push 済み(`1107dbc`) |
| テスト | `cargo test` 48 件成功 |
| VPS ビルド | WSL で release ビルド(`.build-wsl.sh`、出力 `~/aruaru-pro-target/release/aruaru-pro`) |
| VPS 配置 | **済み**: `aruaru-pro.service`(`127.0.0.1:8143`)+専用DB `aruaru-pro-db.service`(`127.0.0.1:15435`)。`/healthz`・`/categories`・`/`・出品/注文/レビュー/求人/研修/ログアウトを実測。`.deploy-vps.sh`で再配備 |
| 公開 DNS | **未了**: `aruaru.pro` に A レコードが無い(NS は ConoHa DNS)。VPS の IPv4 は `160.251.237.162` |
| Google 登録 | クライアント作成済み。リダイレクト URI `https://aruaru.pro/auth/oauth/google/callback` の追加は本人の作業 |
| 実機ログイン往復 | 未検証 |

### 次にやること(この順)

1. **本人の作業**: ConoHa の DNS で `aruaru.pro`(と必要なら `www`)に A レコード `160.251.237.162` を追加。Google コンソールの OAuth クライアントに上記リダイレクト URI を追加。
2. (済み)VPS に配置: バイナリを `/root/aruaru.pro/aruaru-pro` へ、設定を `/etc/aruaru-pro/aruaru-pro.env`(権限 600)へ置き、`aruaru-pro.service`(`127.0.0.1:8143`)を作る。VPS は 4 コアなので、負荷をかける試験はしない(配備と実測確認だけ)。
3. 前面の `open-web-server` の `/root/open-web-server/domains.toml` に `aruaru.pro` を追記(`backend_addr = "127.0.0.1:8143"`)。先にバックアップを取り、DNS が引けてから追記して証明書を取得する。
4. 実機で Google ログインの往復を確認(`/auth/oauth/google/start` → 認可 → `callback` → `aruaru_session` Cookie)。
5. Stripe のテストキーと `stripe listen` で、オンボーディング → チェックアウト → Webhook → 注文 `completed` を通す。
6. REST 風ルートを Connect(`POST /<パッケージ>.<Service>/<Method>`)へ移す。OAuth コールバック・Stripe Webhook・`/healthz` は規格どおりの HTTP のまま(例外)。

### 落とし穴

- `aruaru-db` の ID 生成: 日本語名が空に潰れる slugify を使わず、`ids::make_id` で必ずハッシュを付ける。
- WSL へ渡す PATH にカッコ(`Program Files (x86)` など)が入ると `bash -lc` が壊れる。ビルドはスクリプトファイル(`.build-wsl.sh`)で実行し、PATH は短く指定する。
- `git push` が固まる時は credential helper を `!gh auth git-credential` に切り替える。
- `job-site`(別の試作品)は触らない。

### 貼り付け用の再開メッセージ

> `F:\runo\repository\aruaru.pro` の続きです。`PORTING.md` の「次にやること」から再開してください。VPS へは配備と実測確認だけ(負荷試験なし)、秘密は表示せず、返答は日本語で、実機で確認してから完了と報告してください。まず `aruaru.pro` の DNS(A レコード)と Google のリダイレクト URI が済んでいるかを確認し、済んでいれば手順 2〜4 を進めてください。

---

## English

### Why this file exists

This is a guide so that another account or another PC can pick up this work exactly where it stopped.
Claude does not remember earlier chats, so everything you need is written here.

### Where things are, and what you need

- Repository: <https://github.com/aon-co-jp/aruaru.pro> (on this PC: `F:\runo\repository\aruaru.pro`)
- The sibling repos `RPoem`, `RS-JSON`, `RFrontEnd` (RS-React and RS-HTML) and `aruaru-db` must sit directly under `F:\runo\repository` (the project depends on them by path).
- Add `%USERPROFILE%\.cargo\bin` to your PATH so Rust works.
- Build with `CARGO_BUILD_JOBS=2`. Without it, `async-stripe` (with `webhook-events`) can run out of memory.
- Keep secrets (OAuth and Stripe keys) **in a txt file directly under `F:\`. Never print them, commit them, or put them in the README.** Example: `F:\google-oauth.txt` (line 1 is the client ID, line 2 is the secret; we already checked the format).

### Where we are (2026-11-19)

| Item | State |
|---|---|
| Code | Listings, orders, reviews, jobs, career-agent programs, login (OAuth and cookies), Stripe Connect, webhook and logout are built and pushed. |
| Tests | 48 `cargo test` tests pass. |
| VPS | **Running.** `aruaru-pro.service` (`127.0.0.1:8143`) plus its own database `aruaru-pro-db.service` (`127.0.0.1:15435`). We called `/healthz`, `/categories`, `/`, and the listing, order, review, job, program and logout APIs, and they worked. Redeploy with `.deploy-vps.sh`. |
| Public address (DNS) | **Not done.** `aruaru.pro` has no A record (its name servers are ConoHa DNS). The VPS IPv4 is `160.251.237.162`. |
| Google | The client is created. Adding the redirect URI `https://aruaru.pro/auth/oauth/google/callback` is the owner's job. |
| Real login round trip | Not checked yet. |

### Next steps (in this order)

1. **The owner's job**: in ConoHa DNS, add an A record `160.251.237.162` for `aruaru.pro` (and for `www` if you want it). In the Google console, add the redirect URI above to the OAuth client.
2. Add `aruaru.pro` to the front server's file `/root/open-web-server/domains.toml` with `backend_addr = "127.0.0.1:8143"`. Make a backup first, and add it only after DNS works, so the certificate can be issued.
3. Check Google login on the real site: `/auth/oauth/google/start` → consent screen → `callback` → the `aruaru_session` cookie.
4. With Stripe test keys and `stripe listen`, run the whole path: onboarding → checkout → webhook → order `completed`.
5. Move the REST-style routes to Connect (`POST /<package>.<Service>/<Method>`). The OAuth callback, the Stripe webhook and `/healthz` stay as plain HTTP, because outside standards decide them.

### Things that can trip you up

- **aruaru-db is smaller than PostgreSQL.** There is no `ALTER TABLE`, `ORDER BY`, `LIMIT` or `SELECT 1`, and `WHERE a AND b` returns nothing. Numbers and booleans cannot be sent as parameters, so put them into the SQL with `format!`. Results come back as text, and NULL comes back as the text `"NULL"`. See [`CLAUDE.md`](CLAUDE.md).
- **Do not use the production `aruaru-server` (port 5433).** It has no users. Use the dedicated database `aruaru-pro-db.service`.
- **IDs**: do not use a slugify that turns Japanese names into empty text. Always use `ids::make_id`, which adds a hash.
- **WSL and PATH**: if the PATH you pass into WSL contains parentheses, such as `Program Files (x86)`, `bash -lc` breaks. Run the build from a script file (`.build-wsl.sh`).
- **Git history was rewritten.** Do not `git pull`. Run `git fetch`, then `git reset --mixed origin/main`.
- If `git push` hangs, switch the credential helper to `!gh auth git-credential`.
- **VPS rule**: develop and test on this PC. The VPS (4 cores) is only for deploying and checking, with no load tests.
- Leave `job-site` (a separate prototype) alone.

### Message to paste when you resume

> Continue `F:\runo\repository\aruaru.pro` from "Next steps" in `PORTING.md`. Use the VPS only to deploy and check (no load tests). Never print secrets. Reply in Japanese. Say "done" only after checking on a real system. First check whether the A record for `aruaru.pro` and the Google redirect URI are done. If they are, do steps 2 to 4.

## Versionless API の方針 / Versionless API policy (2026-11-25)
**日本語**: このリポジトリのAPIは **Versionless**(URL・パッケージ名に `v1` 等を付けず、互換性を壊さず足すだけで進化させる)。欄は足すだけ・足す欄は省略可能・知らない欄は無視・名前変更は新名を足して旧を非推奨に(期限は既定で6か月以上、使用0を確認してから削除)・enumは未知の値を許容・安全上の緊急修正は例外。既存の `.v1` は呼び出し側を壊さないよう当面残し、版なしの別名を追加して段階的に畳む。詳細・出典は [`runo/API-POLICY.md`](https://github.com/aon-co-jp/runo/blob/main/API-POLICY.md) 末尾。適用はこれから順に行う。
**English**: This repository's APIs are **Versionless**: no `v1` in URLs or package names; evolve by adding only, without breaking compatibility. Only add fields, new fields are optional, ignore unknown fields, rename by adding the new name and deprecating the old (default deadline 6 months or more, remove only after confirming zero use), tolerate unknown enum values, emergency security fixes are the exception. Existing `.v1` names stay for now so callers do not break; a versionless alias is added and `.v1` is retired in stages. Details and sources: the end of [`runo/API-POLICY.md`](https://github.com/aon-co-jp/runo/blob/main/API-POLICY.md). Rollout is step by step.

## API 方針: REST 全廃・gRPC / Connect へ
**日本語**: このリポジトリでは **REST API を新規に作らない**。ブラウザ・`curl` から呼ぶ API は **Connect の単純呼び出し**(`POST /<パッケージ>.<Service>/<Method>`、`content-type: application/json`)、サービス間は **gRPC / gRPC-web**。GraphQL は読み取り専用。既存の REST は触るときに移し、移したら旧経路は消す。JSON は RS-JSON(`from_str_secure` / `parse_secure`)で読む。例外は相手の規格が形を決めるもの(ACME、OAuth/OIDC のコールバック、第三者の Webhook、ヘルスチェック、静的ファイル、OTLP)のみ。迷ったら Connect。正本は [`runo/API-POLICY.md`](https://github.com/aon-co-jp/runo/blob/main/API-POLICY.md)。
**English**: This repository **does not add new REST APIs**. APIs called from browsers or `curl` use **Connect unary calls** (`POST /<package>.<Service>/<Method>`, `content-type: application/json`); service-to-service calls use **gRPC / gRPC-web**. GraphQL is read-only. Existing REST is migrated when touched and the old route is removed. Read JSON with RS-JSON (`from_str_secure` / `parse_secure`). Exceptions are only shapes dictated by someone else's standard (ACME, OAuth/OIDC callbacks, third-party webhooks, health checks, static files, OTLP). When in doubt, use Connect. Source of truth: the link above.

## 完成まで作り切る方針 / Finish everything policy (2026-11-27)
**日本語**: 全リポジトリで、**未確認は自動で確認し、未実施は実施し、未了は完成させる**(確認を求めず進める)。使用頻度が低いと判断される機能や、**1から作り直す大規模開発になる内容でも、やめずに最後まで開発・TEST・DEBUGする**(別の方式で済ませない、後回しにしない)。進め方: 調査(英語・日本語の検索とGitHub) → 設計 → 実装 → 実機/実データで確認 → 失敗は原因を直して再試験 → README・CLAUDE・PORTING・POLICY を日英で更新 → push。「できた」と書くのは実測した範囲だけ、確認できなかった所は「未確認」と正直に書く。例外(確認を取る): 元に戻せない削除、秘密の公開、所有者の認証・決済・実機操作が要るもの、利用者のシステム設定の変更。
**English**: In every repository, **verify what is unverified, do what is not yet done, and finish what is unfinished** automatically, without asking. Even features that look rarely used, or work that means **a large rewrite from scratch, are developed, tested and debugged to the end** (no substituting another approach, no postponing). Method: research (English and Japanese search plus GitHub) -> design -> implement -> verify on real devices or data -> fix causes and retest on failure -> update README, CLAUDE, PORTING and POLICY in Japanese and English -> push. Write "done" only for what was measured, and say "unverified" honestly for the rest. Exceptions that still need confirmation: irreversible deletions, publishing secrets, anything needing the owner's authentication, payment or real-device actions, and changes to the user's system settings.