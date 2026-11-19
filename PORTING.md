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
| VPS 配置 | 未了(下記) |
| 公開 DNS | **未了**: `aruaru.pro` に A レコードが無い(NS は ConoHa DNS)。VPS の IPv4 は `160.251.237.162` |
| Google 登録 | クライアント作成済み。リダイレクト URI `https://aruaru.pro/auth/oauth/google/callback` の追加は本人の作業 |
| 実機ログイン往復 | 未検証 |

### 次にやること(この順)

1. **本人の作業**: ConoHa の DNS で `aruaru.pro`(と必要なら `www`)に A レコード `160.251.237.162` を追加。Google コンソールの OAuth クライアントに上記リダイレクト URI を追加。
2. VPS に配置(新規の置き場所): バイナリを `/root/aruaru.pro/aruaru-pro` へ、設定を `/etc/aruaru-pro/aruaru-pro.env`(権限 600)へ置き、`aruaru-pro.service`(`127.0.0.1:8143`)を作る。VPS は 4 コアなので、負荷をかける試験はしない(配備と実測確認だけ)。
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

### Purpose of this file

A hand-over guide so another account or another PC can continue this work as is.
Assume Claude's memory is lost; everything needed is written here.

### Where things are and what you need

- Repository: <https://github.com/aon-co-jp/aruaru.pro> (local: `F:\runo\repository\aruaru.pro`)
- Sibling repos (path dependencies): `RPoem`, `RS-JSON`, `RFrontEnd` (RS-React / RS-HTML) and `aruaru-db` must sit directly under `F:\runo\repository`
- Add `%USERPROFILE%\.cargo\bin` to PATH for Rust
- Build with `CARGO_BUILD_JOBS=2` (`async-stripe` with `webhook-events` can run out of memory otherwise)
- Keep secrets (OAuth / Stripe keys) in **a txt file directly under `F:\`; never print them, commit them or put them in the README.** Example: `F:\google-oauth.txt` (line 1 = client ID, line 2 = secret; format already validated)

### Where we are (2026-11-19)

| Item | State |
|---|---|
| Code | Listings, orders, reviews, jobs, career-agent programs, auth (OAuth / cookies), Stripe Connect and webhook are implemented and pushed (`1107dbc`) |
| Tests | 48 `cargo test` tests pass |
| VPS build | Release build in WSL (`.build-wsl.sh`, output `~/aruaru-pro-target/release/aruaru-pro`) |
| VPS placement | Not done yet (see below) |
| Public DNS | **Not done**: `aruaru.pro` has no A record (name servers are ConoHa DNS). The VPS IPv4 is `160.251.237.162` |
| Google registration | Client created. Adding the redirect URI `https://aruaru.pro/auth/oauth/google/callback` is the owner's job |
| Real login round trip | Not verified |

### Next steps (in this order)

1. **Owner's task**: in ConoHa DNS add an A record `160.251.237.162` for `aruaru.pro` (and `www` if wanted). In the Google console add the redirect URI above to the OAuth client.
2. Place it on the VPS (new location): binary at `/root/aruaru.pro/aruaru-pro`, settings in `/etc/aruaru-pro/aruaru-pro.env` (mode 600), and an `aruaru-pro.service` (`127.0.0.1:8143`). The VPS has 4 cores, so do no load tests (deploy and verify only).
3. Add `aruaru.pro` to the front `open-web-server`'s `/root/open-web-server/domains.toml` (`backend_addr = "127.0.0.1:8143"`). Back it up first, and add it only once DNS resolves so the certificate can be issued.
4. Verify the Google login round trip on the real site (`/auth/oauth/google/start` → consent → `callback` → `aruaru_session` cookie).
5. With Stripe test keys and `stripe listen`, run onboarding → checkout → webhook → order `completed`.
6. Move the REST-style routes to Connect (`POST /<package>.<Service>/<Method>`). The OAuth callback, Stripe webhook and `/healthz` stay plain HTTP (exceptions).

### Pitfalls

- ID generation: do not use a slugify that collapses Japanese names to empty strings; always use `ids::make_id`, which appends a hash.
- A PATH containing parentheses (e.g. `Program Files (x86)`) breaks `bash -lc` when passed into WSL. Run the build from a script file (`.build-wsl.sh`) with a short PATH.
- If `git push` hangs, switch the credential helper to `!gh auth git-credential`.
- Leave `job-site` (a separate prototype) alone.

### Paste-ready resume message

> Continue `F:\runo\repository\aruaru.pro` from "Next steps" in `PORTING.md`. Use the VPS only for deployment and verification (no load tests), never print secrets, reply in Japanese, and verify on real systems before reporting done. First check whether the `aruaru.pro` DNS A record and the Google redirect URI are done; if so, carry out steps 2–4.
