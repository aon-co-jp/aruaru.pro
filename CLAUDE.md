# 開発方針＆開発環境ルール(aruaru.pro) / Development rules (aruaru.pro)

**日本語** | [English](#english)

作業場所は `F:\runo\repository\aruaru.pro`。ルールの正本は
[`open-raid-z`](https://github.com/aon-co-jp/open-raid-z) の `CLAUDE.md` です。
このファイルは、昔の経過をすべて書く場所ではなく、**いま必要なことだけを短く**書く場所です(古い経過は git の履歴にあります)。

---

## 日本語

### 1. このリポジトリは何か

ココナラ([coconala.com](https://coconala.com/))のような「スキルのお店」と、求人・アルバイトのサイトを **ひとつにしたサービス** です。
目標は **手数料をとても安く** すること(いまはお金の 5%)。ドメイン `aruaru.pro` は取得済みです(ユーザー指示、2026-09-13)。
いまは **試作品(実験)** で、一般公開はまだです。

### 2. 技術と方針

- **バックエンド**: Rust + [RPoem](https://github.com/aon-co-jp/RPoem) + [aruaru-db](https://github.com/aon-co-jp/aruaru-db)
- **画面**: [RS-React](https://github.com/aon-co-jp/RS-React)(サーバー側で HTML を作る SSR)
- **決済**: Stripe Connect。お金の移動は Stripe に任せ、自前の送金システムは作らない(法律の面から)
- **Rust 製の自前部品を優先**: RS-JSON・RS-HTML・RS-CSS・RS-BootStrap・RS-React・RS-GraphQL(未成熟な部品は、先に部品側を仕上げてから使う)
- **スコープ**: ココナラの主要カテゴリを最初から幅広く(1〜2 カテゴリの MVP にしない)+ 求人・アルバイト + 無料 IT 研修つき転職エージェント
- **`job-site`(別の試作品)は触らない**(ユーザー指示)
- **API の方針**: REST は全廃して Connect / gRPC に移す(下の方針の節を参照)。このリポジトリの今のルートは REST 風なので、**あとで Connect へ移す。新しい REST は増やさない**。OAuth のコールバック・Stripe の Webhook・`/healthz` は、相手の規格が決めているので例外
- **JSON は RS-JSON**(サイズ・深さ・重複キーの上限つき)

### 3. いまの状態(2026-11-19)

| 項目 | 状態 |
|---|---|
| コード | 出品・注文・レビュー・求人・転職エージェント・認証(Bearer + DB の Cookie + Google/Facebook/X の OAuth)・Stripe Connect・Webhook・ログアウト |
| テスト | `cargo test` 48 件成功 |
| VPS | `aruaru-pro.service`(`127.0.0.1:8143`)と専用 DB `aruaru-pro-db.service`(`127.0.0.1:15435`)が稼働。API を実際に通して確認済み |
| 公開 | **未了**。`aruaru.pro` に A レコードがない(ネームサーバーは ConoHa DNS、VPS の IPv4 は `160.251.237.162`) |
| Google ログインの往復 | 未検証(認可 URL の生成までは確認) |
| Stripe の往復 | 未検証(テストキーがない) |

### 4. 作り方と配備

- Rust は PATH に `%USERPROFILE%\.cargo\bin` を足す。`CARGO_BUILD_JOBS=2` で作る(`async-stripe` はメモリを多く使う)。
- 兄弟リポジトリ(`RPoem`・`RS-JSON`・`RFrontEnd`・`aruaru-db`)を `F:\runo\repository` の直下に置く(path 依存)。
- **開発・テスト・デバッグはこの PC(WSL を含む)で行う。VPS は配備と実測確認だけ**(負荷をかける試験はしない。VPS は 4 コア)。
- 配備は `./.deploy-vps.sh`(WSL で release ビルド → `/root/aruaru.pro/aruaru-pro` へ転送 → 再起動。旧バイナリは `aruaru-pro.prev`)。WSL の PATH にカッコが入ると壊れるので、ビルドは `.build-wsl.sh` で行う。
- VPS の設定: `/etc/aruaru-pro/aruaru-pro.env`(権限 600。DB の接続文字列と Google の ID/シークレット)、`/etc/aruaru-pro/aruaru-db.env`(DB のユーザー)。
- 前面は `open-web-server`(`/root/open-web-server/domains.toml`)。**DNS が引けるようになってから** `aruaru.pro` を追記し、`backend_addr = "127.0.0.1:8143"` にする(先にバックアップを取る)。

### 5. aruaru-db の方言(VPS の実機で確認)

- 未対応: `ALTER TABLE`、`ORDER BY`、`LIMIT`、`SELECT 1`、`WHERE a = x AND b = y`(AND でつなぐと 0 件になる)。条件 1 つで絞り、残りは Rust で判定する。
- 引数の型は「不明」なので、文字列以外(整数・真偽値)は `format!` で数値として SQL に埋め込む(値は Rust の整数・真偽値なので注入の心配はない)。
- 結果の列はすべて文字列で返る(整数は `parse`、真偽値は `"t"`/`"true"`)。NULL は文字列 `"NULL"` で返る。
- 本番の `aruaru-server`(5433)はユーザーが無いので使わず、専用の DB(`aruaru-pro-db.service`)を使う。

### 6. 守ること・落とし穴

- **秘密(キー・シークレット)は、コード・README・チャットに書かない。** `F:\` 直下の txt や、権限 600 のファイルに置く。例: `F:\google-oauth.txt`(1 行目 = ID、2 行目 = シークレット)。
- ID は `ids::make_id` を使う(日本語の名前が空になる slugify は使わない。必ずハッシュを付ける)。
- このリポジトリの GitHub 履歴は書き直されている。**`git pull` しない**。`git fetch` のあと `git reset --mixed origin/main` で合わせる。
- `git push` が固まるときは `-c credential.helper='!gh auth git-credential'` を付ける。
- 返事は日本語で。実機で確認してから「完了」と報告する。

### 7. 次にやること

1. **本人の作業**: ConoHa DNS に `aruaru.pro` の A レコード `160.251.237.162` を追加。Google コンソールの OAuth クライアントに `https://aruaru.pro/auth/oauth/google/callback` を追加。
2. 前面の `domains.toml` に追記して証明書を取る。
3. 本物のサイトで Google ログインの往復を確認(`start` → 認可 → `callback` → `aruaru_session` Cookie)。
4. Stripe のテストキー + `stripe listen` で、オンボーディング → チェックアウト → Webhook → 注文 `completed` を通す。
5. REST 風のルートを Connect(`POST /<パッケージ>.<Service>/<Method>`)へ移す。
6. Facebook・X のアプリ登録(本人の作業)。X はメールアドレスを返さない。
7. `career_agent_programs` の収益モデルの検討。`App::tick` を使う対話 UI(カテゴリ絞り込みなど)。

### 8. OAuth のリダイレクト URI(本人が各サービスに登録)

| サービス | URI |
|---|---|
| Google | `https://aruaru.pro/auth/oauth/google/callback` |
| Facebook | `https://aruaru.pro/auth/oauth/facebook/callback` |
| X | `https://aruaru.pro/auth/oauth/x/callback` |

### 9. 関連プロジェクト

[RPoem](https://github.com/aon-co-jp/RPoem) / [RS-React](https://github.com/aon-co-jp/RS-React) / [aruaru-db](https://github.com/aon-co-jp/aruaru-db) / [RS-JSON](https://github.com/aon-co-jp/RS-JSON) / [RFrontEnd](https://github.com/aon-co-jp/RFrontEnd) / [open-raid-z](https://github.com/aon-co-jp/open-raid-z)(ルールの正本)

### 10. HANDOFF(経過の要約)

- **2026-09-13** リポジトリ新設。出品・注文・レビュー・求人・転職エージェント・認証・Stripe Connect・OAuth・DB の Cookie セッションを実装。
- **2026-09-24** Stripe Webhook(支払い完了 → 注文 `completed`)とログアウトを実装。
- **2026-11-19** VPS に配備(専用 DB つき)。aruaru-db の方言に合わせて SQL を修正。README・PORTING・CLAUDE を日英で書き直し。公開は DNS 待ち。再開は [`PORTING.md`](PORTING.md) から。

---

## English

### 1. What is this repository?

It is **one service that joins two things**: a "skill shop" like Coconala ([coconala.com](https://coconala.com/)) and a site for jobs and part-time work.
The goal is **very low fees** (right now, 5% of each payment). The owner already has the domain `aruaru.pro` (user instruction, 2026-09-13).
Today it is a **prototype (an experiment)** and is not open to the public yet.

### 2. Tech and policy

- **Back end**: Rust + [RPoem](https://github.com/aon-co-jp/RPoem) + [aruaru-db](https://github.com/aon-co-jp/aruaru-db)
- **Screens**: [RS-React](https://github.com/aon-co-jp/RS-React) (the server builds the HTML, called SSR)
- **Payments**: Stripe Connect. Stripe moves the money; we do not build our own money-transfer system (for legal reasons).
- **Prefer our own Rust parts**: RS-JSON, RS-HTML, RS-CSS, RS-BootStrap, RS-React, RS-GraphQL (if a part is not ready, finish that part first, then use it).
- **Scope**: wide Coconala-style categories from day one (not a 1–2 category MVP), plus jobs and part-time work, plus a career-agent corner with free IT training.
- **Do not touch `job-site`** (a separate prototype) — user instruction.
- **API policy**: REST is being abolished in favor of Connect / gRPC (see the policy sections below). The routes here are REST-style today, so **they will move to Connect later. Do not add new REST routes.** The OAuth callback, the Stripe webhook and `/healthz` are exceptions because someone else's standard decides their shape.
- **JSON goes through RS-JSON** (with size, depth and duplicate-key limits).

### 3. Where we are (2026-11-19)

| Item | State |
|---|---|
| Code | Listings, orders, reviews, jobs, career-agent programs, auth (Bearer + DB cookie + Google/Facebook/X OAuth), Stripe Connect, webhook, logout |
| Tests | 48 `cargo test` tests pass |
| VPS | `aruaru-pro.service` (`127.0.0.1:8143`) and a dedicated DB `aruaru-pro-db.service` (`127.0.0.1:15435`) are running. The APIs were really called and checked. |
| Going public | **Not done.** `aruaru.pro` has no A record (name servers are ConoHa DNS; the VPS IPv4 is `160.251.237.162`). |
| Google login round trip | Not verified (we only checked that the consent URL is built). |
| Stripe round trip | Not verified (no test keys). |

### 4. Build and deploy

- Put `%USERPROFILE%\.cargo\bin` on PATH. Build with `CARGO_BUILD_JOBS=2` (`async-stripe` uses a lot of memory).
- Sibling repos (`RPoem`, `RS-JSON`, `RFrontEnd`, `aruaru-db`) must sit directly under `F:\runo\repository` (path dependencies).
- **Develop, test and debug on this PC (WSL included). Use the VPS only to deploy and to check** — no load tests (the VPS has 4 cores).
- Deploy with `./.deploy-vps.sh` (release build in WSL → copy to `/root/aruaru.pro/aruaru-pro` → restart; the old binary is kept as `aruaru-pro.prev`). A PATH with parentheses breaks WSL, so the build runs through `.build-wsl.sh`.
- VPS settings: `/etc/aruaru-pro/aruaru-pro.env` (mode 600: DB connection string and the Google ID/secret) and `/etc/aruaru-pro/aruaru-db.env` (the DB user).
- The front server is `open-web-server` (`/root/open-web-server/domains.toml`). **Only after DNS resolves**, add `aruaru.pro` with `backend_addr = "127.0.0.1:8143"` (back the file up first).

### 5. aruaru-db dialect (checked on the real VPS)

- Not supported: `ALTER TABLE`, `ORDER BY`, `LIMIT`, `SELECT 1`, and `WHERE a = x AND b = y` (an AND chain returns zero rows). Filter on one condition and check the rest in Rust.
- Parameter types are "unknown", so anything that is not a string (integers, booleans) is put into the SQL with `format!` as a plain number (they are Rust integers and bools, so there is no injection risk).
- Every result column comes back as text (parse integers; booleans are `"t"`/`"true"`). NULL comes back as the string `"NULL"`.
- Production `aruaru-server` (5433) has no users, so we do not use it. We use our own DB (`aruaru-pro-db.service`).

### 6. Rules and traps

- **Never write secrets (keys, client secrets) in code, the README or chat.** Keep them in a txt file under `F:\` or a mode-600 file. Example: `F:\google-oauth.txt` (line 1 = ID, line 2 = secret).
- Use `ids::make_id` for IDs (do not use a slugify that turns Japanese names into empty strings; always add a hash).
- This repository's GitHub history was rewritten. **Do not `git pull`.** Run `git fetch`, then `git reset --mixed origin/main`.
- If `git push` hangs, add `-c credential.helper='!gh auth git-credential'`.
- Reply in Japanese. Say "done" only after checking on a real system.

### 7. Next steps

1. **Owner's task**: add an A record `160.251.237.162` for `aruaru.pro` in ConoHa DNS. In the Google console add `https://aruaru.pro/auth/oauth/google/callback` to the OAuth client.
2. Add the site to the front `domains.toml` and get the certificate.
3. Check the Google login round trip on the real site (`start` → consent → `callback` → `aruaru_session` cookie).
4. With Stripe test keys and `stripe listen`, run onboarding → checkout → webhook → order `completed`.
5. Move the REST-style routes to Connect (`POST /<package>.<Service>/<Method>`).
6. Register the Facebook and X apps (owner's task). X does not return an email address.
7. Think about how the career-agent programs earn money. Build an interactive UI with `App::tick` (for example, category filtering).

### 8. OAuth redirect URIs (the owner registers these with each provider)

| Provider | URI |
|---|---|
| Google | `https://aruaru.pro/auth/oauth/google/callback` |
| Facebook | `https://aruaru.pro/auth/oauth/facebook/callback` |
| X | `https://aruaru.pro/auth/oauth/x/callback` |

### 9. Related projects

[RPoem](https://github.com/aon-co-jp/RPoem) / [RS-React](https://github.com/aon-co-jp/RS-React) / [aruaru-db](https://github.com/aon-co-jp/aruaru-db) / [RS-JSON](https://github.com/aon-co-jp/RS-JSON) / [RFrontEnd](https://github.com/aon-co-jp/RFrontEnd) / [open-raid-z](https://github.com/aon-co-jp/open-raid-z) (source of truth for the rules)

### 10. HANDOFF (short history)

- **2026-09-13** Repository created. Built listings, orders, reviews, jobs, career-agent programs, auth, Stripe Connect, OAuth and DB-backed cookie sessions.
- **2026-09-24** Stripe webhook (payment done → order `completed`) and logout.
- **2026-11-19** Deployed to the VPS (with its own DB). Fixed the SQL to fit the aruaru-db dialect. Rewrote README, PORTING and CLAUDE in Japanese and English. Going public waits for DNS. Resume from [`PORTING.md`](PORTING.md).

---

## API 方針: REST 全廃・gRPC / Connect へ (2026-11-04)

### 日本語
- **REST は全廃する。新しい API は REST で作らない。** ブラウザや `curl` から呼ぶ API は **Connect の単純呼び出し**(`POST /<パッケージ>.<Service>/<Method>`、`content-type: application/json`、本文は JSON)、サービス間は **gRPC / gRPC-web**(RPoem の管理API `open_runo.admin.v1.Admin` と同じ流儀)。ストリーミングは、必要になってから。
- **既存の REST は、触るときに順に移す。** 移したら旧 REST の経路は消す。移行が終わるまで残るが、**新規には追加しない**。
- **JSON は RS-JSON に統一**する(読み込みは `from_str_secure` / `parse_secure` でサイズ・深さ・重複キーの上限つき、書き出しは `rust_json::fast`)。ブラウザの WASM に `prost` は入れない(サイズが増えるため)。
- **例外(自分たちの API ではなく、相手の規格が決めているもの)**: ACME、OAuth/OIDC のコールバック、第三者(GitHub など)からの Webhook、ヘルスチェック、静的ファイル配信、OTLP など。これらは規格どおりの HTTP を受ける。
- 参考実装: `RPoem`(`open_runo.admin.v1.Admin`)、`open-easy-web`(`server/src/connect.rs`、`proto/open_easy_web/admin/v1/admin.proto`)。
- ※ このリポジトリのコードが既に方針どおりかどうかは、各リポジトリの現状による。この節は方針の記録であり、コードの変更を意味しない。

### English
- **REST is being abolished. New APIs are not built as REST.** APIs called from browsers or `curl` use **Connect unary calls** (`POST /<package>.<Service>/<Method>`, `content-type: application/json`, JSON body); service-to-service calls use **gRPC / gRPC-web** (the same style as RPoem's admin API `open_runo.admin.v1.Admin`). Streaming only when it is actually needed.
- **Existing REST endpoints are migrated one by one, when they are touched.** The old REST route is removed once migrated. Until migration is finished they remain, but **no new ones are added**.
- **JSON is unified on RS-JSON**: reading with `from_str_secure` / `parse_secure` (size, depth and duplicate-key limits), writing with `rust_json::fast`. `prost` is not put into browser WASM (it would grow the binary).
- **Exceptions (shapes dictated by someone else's standard, not our own API)**: ACME, OAuth/OIDC callbacks, webhooks from third parties (e.g. GitHub), health checks, static file serving, OTLP, and so on. These accept plain HTTP as the standard requires.
- Reference implementations: `RPoem` (`open_runo.admin.v1.Admin`) and `open-easy-web` (`server/src/connect.rs`, `proto/open_easy_web/admin/v1/admin.proto`).
- Note: whether the code in a given repository already follows this policy depends on that repository. This section records the policy; it does not mean the code has been changed.

### Deutsch (KI-Übersetzung, redaktionell überarbeitet; noch nicht von Muttersprachlern geprüft)
- **REST wird vollständig abgeschafft; neue APIs entstehen nicht mehr als REST.** APIs, die aus dem Browser oder per `curl` aufgerufen werden, verwenden **unäre Connect-Aufrufe** (`POST /<Paket>.<Service>/<Methode>`, `content-type: application/json`, JSON-Body). Zwischen Diensten kommen **gRPC bzw. gRPC-web** zum Einsatz, im selben Stil wie die Admin-API von RPoem (`open_runo.admin.v1.Admin`). Streaming wird erst ergänzt, wenn es tatsächlich gebraucht wird.
- **Bestehende REST-Endpunkte werden schrittweise umgestellt, und zwar immer dann, wenn man ohnehin an ihnen arbeitet.** Sobald ein Endpunkt umgestellt ist, wird die alte REST-Route entfernt. Bis zum Abschluss der Migration bleiben die übrigen bestehen, es werden aber **keine neuen mehr hinzugefügt**.
- **JSON wird einheitlich über RS-JSON verarbeitet**: Gelesen wird mit `from_str_secure` bzw. `parse_secure` (mit Obergrenzen für Größe, Verschachtelungstiefe und doppelte Schlüssel), geschrieben mit `rust_json::fast`. `prost` wird nicht in das Browser-WASM aufgenommen, da es die Datei vergrößern würde.
- **Ausnahmen (Schnittstellen, deren Form ein fremder Standard vorgibt und die nicht zu unserer eigenen API gehören)**: ACME, OAuth/OIDC-Callbacks, Webhooks von Drittanbietern (z. B. GitHub), Health-Checks, die Auslieferung statischer Dateien, OTLP und Ähnliches. Diese nehmen, wie vom jeweiligen Standard verlangt, gewöhnliches HTTP entgegen.
- Referenzimplementierungen: `RPoem` (`open_runo.admin.v1.Admin`) und `open-easy-web` (`server/src/connect.rs`, `proto/open_easy_web/admin/v1/admin.proto`).
- Hinweis: Ob der Code eines einzelnen Repositorys diese Richtlinie bereits erfüllt, hängt vom jeweiligen Repository ab. Dieser Abschnitt dokumentiert lediglich die Richtlinie; er besagt nicht, dass der Code schon angepasst wurde.

### Русский (перевод выполнен с помощью ИИ и отредактирован; носителями языка пока не проверен)
- **От REST мы полностью отказываемся; новые API больше не создаются в стиле REST.** API, к которым обращаются из браузера или через `curl`, используют **унарные вызовы Connect** (`POST /<пакет>.<Service>/<Method>`, `content-type: application/json`, тело запроса — JSON). Для взаимодействия между сервисами применяются **gRPC и gRPC-web** — в том же стиле, что и административный API RPoem (`open_runo.admin.v1.Admin`). Потоковая передача данных добавляется лишь тогда, когда в ней действительно возникает необходимость.
- **Существующие REST-эндпоинты переводятся поэтапно — по мере того, как с ними приходится работать.** После перевода старый REST-маршрут удаляется. Пока миграция не завершена, оставшиеся эндпоинты сохраняются, но **новые не добавляются**.
- **Работа с JSON унифицирована на основе RS-JSON**: чтение — через `from_str_secure` или `parse_secure` (с ограничениями на размер, глубину вложенности и повторяющиеся ключи), запись — через `rust_json::fast`. `prost` в браузерный WASM не включается, поскольку это увеличило бы размер файла.
- **Исключения (интерфейсы, форму которых определяет чужой стандарт, а не наш собственный API)**: ACME, обратные вызовы OAuth/OIDC, вебхуки сторонних сервисов (например, GitHub), проверки работоспособности, раздача статических файлов, OTLP и тому подобное. Такие интерфейсы принимают обычный HTTP, как того требует соответствующий стандарт.
- Эталонные реализации: `RPoem` (`open_runo.admin.v1.Admin`) и `open-easy-web` (`server/src/connect.rs`, `proto/open_easy_web/admin/v1/admin.proto`).
- Примечание: соответствует ли код конкретного репозитория этой политике, зависит от самого репозитория. Данный раздел лишь фиксирует политику и не означает, что код уже изменён.

## API 方針の補足: 用語の整理と使い分け (2026-11-04)

### 日本語
**1つに統一するものではなく、別の層の話です。** 「REST の代わり」になるのは Connect・gRPC・GraphQL だけで、WASM は画面を作る技術です。

| 用語 | 何か | REST の代わりか |
|---|---|---|
| REST | URL と HTTP メソッドで操作を表す API の作り方 | 全廃する側 |
| **Connect** | `POST /サービス名/メソッド名` に JSON を送る形式。`curl` でもブラウザの `fetch` でも呼べる | はい(**採用**) |
| **gRPC** | 同じ「サービス/メソッド」の考え方を、バイナリ(protobuf)と HTTP/2 で行う形式。速く、ストリーミングも得意 | はい(**サービス同士に採用**) |
| gRPC-web | gRPC のブラウザ向けの変形版 | はい(必要なら) |
| GraphQL | 欲しい項目を問い合わせる形式 | 一部だけ(**問い合わせ(読み取り)専用**) |
| WASM | Rust をブラウザの中で動かす仕組み | いいえ(画面を作る技術) |

- **ブラウザの画面や `curl` から呼ぶ API は Connect**(POST、JSON を RS-JSON で読み書き)。ブラウザの WASM は素の gRPC を話せず、Connect なら `fetch` のままで、`prost` を足さずに済む(画面のサイズも増えない)。
- **サービス同士の通信(サーバー間、重い処理、ストリーミング)は gRPC。** 多くは Connect で足りるので、ストリーミングが必要になった所だけ gRPC に上げる。
- **GraphQL は、まとめて問い合わせたいときの読み取り専用。** 書き込み(操作)には使わない。
- **WASM は画面を作る技術として、そのまま使う。** 画面から呼ぶ API が Connect になっただけ。
- 外部の規格が形を決めるもの(ACME、OAuth のコールバック、第三者の Webhook、ヘルスチェック、静的ファイル、OTLP)は、規格どおりの HTTP のまま。
- **迷ったら Connect。**

### English
**These are not alternatives to unify; they sit at different layers.** Only Connect, gRPC and GraphQL can replace REST; WASM is a way to build the UI.

| Term | What it is | Replaces REST? |
|---|---|---|
| REST | API style that expresses operations through URLs and HTTP methods | The thing being abolished |
| **Connect** | `POST /Service/Method` with a JSON body; callable from `curl` and from a browser `fetch` | Yes (**adopted**) |
| **gRPC** | The same service/method idea over binary (protobuf) and HTTP/2; fast, good at streaming | Yes (**for service-to-service**) |
| gRPC-web | A browser-friendly variant of gRPC | Yes (if needed) |
| GraphQL | A query language: ask for exactly the fields you want | Partly (**read-only queries**) |
| WASM | Runs Rust inside the browser | No (a UI technology) |

- **APIs called from browser screens or `curl` use Connect** (POST, JSON read and written with RS-JSON). Browser WASM cannot speak plain gRPC; with Connect it stays a `fetch`, and `prost` is not needed (the binary does not grow).
- **Service-to-service calls (server to server, heavy work, streaming) use gRPC.** Connect is enough in most places; move a call to gRPC only where streaming is actually needed.
- **GraphQL is read-only**, for queries that gather several pieces of data. It is not used for writes (operations).
- **WASM stays as the way to build the UI.** The API it calls simply became Connect.
- Interfaces whose shape an outside standard dictates (ACME, OAuth callbacks, third-party webhooks, health checks, static files, OTLP) stay plain HTTP.
- **When in doubt, use Connect.**

### Deutsch (KI-Übersetzung, sprachlich überarbeitet; noch nicht von Muttersprachlern geprüft)
**Es geht nicht darum, eine einzige Technik auszuwählen – die Begriffe liegen auf unterschiedlichen Ebenen.** Als Ersatz für REST kommen nur Connect, gRPC und GraphQL in Frage; WASM dagegen ist eine Technik, mit der die Benutzeroberfläche gebaut wird.

| Begriff | Was ist das? | Ersatz für REST? |
|---|---|---|
| REST | Ein API-Stil, bei dem Operationen über URLs und HTTP-Methoden ausgedrückt werden | wird abgeschafft |
| **Connect** | `POST /Dienst/Methode` mit JSON im Body; aufrufbar mit `curl` ebenso wie mit `fetch` im Browser | ja (**gewählt**) |
| **gRPC** | Dasselbe Prinzip aus Dienst und Methode, aber über binäres protobuf und HTTP/2; schnell und gut für Streaming geeignet | ja (**zwischen Diensten**) |
| gRPC-web | Eine für den Browser angepasste Variante von gRPC | ja (bei Bedarf) |
| GraphQL | Abfragesprache, mit der man genau die benötigten Felder anfordert | teilweise (**nur für lesende Abfragen**) |
| WASM | Führt Rust-Code direkt im Browser aus | nein (Technik für die Oberfläche) |

- **APIs, die von Browseroberflächen oder per `curl` aufgerufen werden, verwenden Connect** (POST; JSON wird mit RS-JSON gelesen und geschrieben). WASM im Browser kann kein reines gRPC sprechen. Mit Connect bleibt es bei einem gewöhnlichen `fetch`, und `prost` ist nicht nötig – die WASM-Datei wird dadurch nicht größer.
- **Für die Kommunikation zwischen Diensten (Server zu Server, rechenintensive Aufgaben, Streaming) wird gRPC eingesetzt.** In den meisten Fällen reicht Connect aus; auf gRPC wird nur dort umgestellt, wo Streaming tatsächlich gebraucht wird.
- **GraphQL dient ausschließlich lesenden Abfragen**, wenn mehrere Informationen gebündelt abgefragt werden sollen. Für Schreibzugriffe (Operationen) wird es nicht verwendet.
- **WASM bleibt die Technik für die Oberfläche.** Geändert hat sich lediglich, dass die aufgerufene API nun auf Connect beruht.
- Schnittstellen, deren Form durch einen externen Standard festgelegt ist (ACME, OAuth-Callbacks, Webhooks von Drittanbietern, Health-Checks, statische Dateien, OTLP), bleiben gewöhnliches HTTP.
- **Im Zweifel: Connect.**

### Русский (перевод выполнен с помощью ИИ и литературно отредактирован; носителями языка пока не проверен)
**Это не варианты, из которых нужно выбрать один, а технологии разных уровней.** Заменить REST могут только Connect, gRPC и GraphQL; WASM же — это способ создания пользовательского интерфейса.

| Термин | Что это | Заменяет REST? |
|---|---|---|
| REST | Стиль построения API, при котором операции выражаются через URL и HTTP-методы | отменяется |
| **Connect** | `POST /Сервис/Метод` с телом в формате JSON; вызывается как из `curl`, так и из браузерного `fetch` | да (**выбран**) |
| **gRPC** | Тот же принцип «сервис — метод», но поверх двоичного формата protobuf и HTTP/2; работает быстро и хорошо подходит для потоковой передачи | да (**для связи между сервисами**) |
| gRPC-web | Адаптированный для браузера вариант gRPC | да (при необходимости) |
| GraphQL | Язык запросов: клиент запрашивает ровно те поля, которые ему нужны | частично (**только запросы на чтение**) |
| WASM | Позволяет выполнять код на Rust прямо в браузере | нет (технология для интерфейса) |

- **API, которые вызываются из браузерных интерфейсов или через `curl`, строятся на Connect** (POST; JSON читается и записывается через RS-JSON). Браузерный WASM не умеет работать с «чистым» gRPC. С Connect достаточно обычного `fetch`, а `prost` не требуется — размер WASM-файла не увеличивается.
- **Для обмена между сервисами (сервер — сервер, ресурсоёмкие задачи, потоковая передача) применяется gRPC.** В большинстве случаев достаточно Connect; на gRPC переходят только там, где потоковая передача действительно необходима.
- **GraphQL используется только для чтения**, когда нужно получить сразу несколько связанных данных. Для записи (операций) он не применяется.
- **WASM по-прежнему остаётся технологией для интерфейса.** Изменилось лишь то, что вызываемый им API теперь построен на Connect.
- Интерфейсы, форма которых задана внешним стандартом (ACME, обратные вызовы OAuth, вебхуки сторонних сервисов, проверки работоспособности, статические файлы, OTLP), остаются обычным HTTP.
- **Если есть сомнения — выбирайте Connect.**

## Versionless API の方針 / Versionless API policy (2026-11-25)
**日本語**: このリポジトリのAPIは **Versionless**(URL・パッケージ名に `v1` 等を付けず、互換性を壊さず足すだけで進化させる)。欄は足すだけ・足す欄は省略可能・知らない欄は無視・名前変更は新名を足して旧を非推奨に(期限は既定で6か月以上、使用0を確認してから削除)・enumは未知の値を許容・安全上の緊急修正は例外。既存の `.v1` は呼び出し側を壊さないよう当面残し、版なしの別名を追加して段階的に畳む。詳細・出典は [`runo/API-POLICY.md`](https://github.com/aon-co-jp/runo/blob/main/API-POLICY.md) 末尾。適用はこれから順に行う。
**English**: This repository's APIs are **Versionless**: no `v1` in URLs or package names; evolve by adding only, without breaking compatibility. Only add fields, new fields are optional, ignore unknown fields, rename by adding the new name and deprecating the old (default deadline 6 months or more, remove only after confirming zero use), tolerate unknown enum values, emergency security fixes are the exception. Existing `.v1` names stay for now so callers do not break; a versionless alias is added and `.v1` is retired in stages. Details and sources: the end of [`runo/API-POLICY.md`](https://github.com/aon-co-jp/runo/blob/main/API-POLICY.md). Rollout is step by step.

## 完成まで作り切る方針 / Finish everything policy (2026-11-27)
**日本語**: 全リポジトリで、**未確認は自動で確認し、未実施は実施し、未了は完成させる**(確認を求めず進める)。使用頻度が低いと判断される機能や、**1から作り直す大規模開発になる内容でも、やめずに最後まで開発・TEST・DEBUGする**(別の方式で済ませない、後回しにしない)。進め方: 調査(英語・日本語の検索とGitHub) → 設計 → 実装 → 実機/実データで確認 → 失敗は原因を直して再試験 → README・CLAUDE・PORTING・POLICY を日英で更新 → push。「できた」と書くのは実測した範囲だけ、確認できなかった所は「未確認」と正直に書く。例外(確認を取る): 元に戻せない削除、秘密の公開、所有者の認証・決済・実機操作が要るもの、利用者のシステム設定の変更。
**English**: In every repository, **verify what is unverified, do what is not yet done, and finish what is unfinished** automatically, without asking. Even features that look rarely used, or work that means **a large rewrite from scratch, are developed, tested and debugged to the end** (no substituting another approach, no postponing). Method: research (English and Japanese search plus GitHub) -> design -> implement -> verify on real devices or data -> fix causes and retest on failure -> update README, CLAUDE, PORTING and POLICY in Japanese and English -> push. Write "done" only for what was measured, and say "unverified" honestly for the rest. Exceptions that still need confirmation: irreversible deletions, publishing secrets, anything needing the owner's authentication, payment or real-device actions, and changes to the user's system settings.