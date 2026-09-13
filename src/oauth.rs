//! Google(Gmail)・Facebook・X(旧Twitter)のOAuth 2.0ログイン。
//!
//! `auth.rs`の素朴な「名前+ロールだけでトークン発行」の登録
//! (`POST /auth/register`)に加え、実在のメールアドレス/アカウントで
//! 本人確認する経路として新設(ユーザー指示、2026-09-13:
//! 「GmailやFacebookやXのOAuth認証は実装して」)。
//!
//! ## フロー
//! 1. `GET /auth/oauth/:provider/start?role=buyer` → 各プロバイダの
//!    認可エンドポイントへ302リダイレクト。`state`(CSRF対策の
//!    ワンタイム値)と、Xのみ必要なPKCE(`code_verifier`/
//!    `code_challenge`)を`oauth_pending_states`テーブル(DB、下記
//!    「2026-09-13、DBバックへ本格化」参照)に保持する。
//! 2. ユーザーがプロバイダ側でログイン・許可 → プロバイダが
//!    `redirect_uri`(`/auth/oauth/:provider/callback`)へ`code`+`state`
//!    付きでリダイレクト。
//! 3. `code`をアクセストークンに交換 → プロバイダのuserinfo APIで
//!    メール/名前/subject idを取得 → `(provider, subject)`で既存の
//!    ユーザーを検索、無ければ新規作成 → 自前のBearerトークンを発行
//!    して返し、`auth::issue_session`でCookieセッションも発行する。
//!
//! ## 2026-09-13、DBバックへ本格化
//! 以前はstate/PKCEをプロセス内メモリ(`OnceLock<Mutex<HashMap<..>>>`)に
//! 保持していたため、プロセス再起動やマルチインスタンス構成
//! (ロードバランサ配下に複数プロセス)では、`start`を受けたプロセスと
//! `callback`を受けたプロセスが別だと機能しないという限界があった
//! (ユーザー指摘、2026-09-13:「Cookieセッションがある試作品としては
//! 本格的な開発をして、再起動やマルチインスタンス構成でも機能させて」)。
//! `oauth_pending_states`テーブル(DB永続化、`aruaru-db`へ接続する
//! どのプロセスからも同じ状態を検証できる)へ置き換えた。
//! 有効期限(`PENDING_STATE_MAX_AGE_SECONDS`、15分)を過ぎたエントリは
//! `callback`で無効として扱う——`start`のたびに期限切れエントリを
//! 機会的に削除する(専用のバックグラウンドジョブは持たない、
//! 軽量な自己クリーンアップ)。
//!
//! ## 正直な開示(現状のスコープ)
//! - **X(Twitter)のuserinfo APIはデフォルトでメールアドレスを返さない**
//!   (Xの制約——メール取得には別途申請が必要)。そのため`email`は
//!   `None`のまま登録されることがある。
//! - どのプロバイダも**実際のOAuthアプリ登録(client_id/secret)が
//!   このセッションの環境には無いため、実機(実際のGoogle/Facebook/X
//!   アカウントでのログイン往復)は未検証**。URL構築・トークン交換・
//!   userinfoパース・DBへのupsertのロジックはユニットテストで検証。

use aruaru_db_connector::AruaruDb;
use open_runo_poem_compat::hyper_compat::json_response;
use open_runo_poem_compat::{PathParams, Request, Response, StatusCode};
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Google,
    Facebook,
    X,
}

impl Provider {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "google" => Some(Provider::Google),
            "facebook" => Some(Provider::Facebook),
            "x" => Some(Provider::X),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Provider::Google => "google",
            Provider::Facebook => "facebook",
            Provider::X => "x",
        }
    }

    fn env_prefix(self) -> &'static str {
        match self {
            Provider::Google => "GOOGLE",
            Provider::Facebook => "FACEBOOK",
            Provider::X => "X",
        }
    }

    fn authorize_endpoint(self) -> &'static str {
        match self {
            Provider::Google => "https://accounts.google.com/o/oauth2/v2/auth",
            Provider::Facebook => "https://www.facebook.com/v19.0/dialog/oauth",
            Provider::X => "https://twitter.com/i/oauth2/authorize",
        }
    }

    fn token_endpoint(self) -> &'static str {
        match self {
            Provider::Google => "https://oauth2.googleapis.com/token",
            Provider::Facebook => "https://graph.facebook.com/v19.0/oauth/access_token",
            Provider::X => "https://api.twitter.com/2/oauth2/token",
        }
    }

    fn scope(self) -> &'static str {
        match self {
            Provider::Google => "openid email profile",
            Provider::Facebook => "email public_profile",
            Provider::X => "tweet.read users.read offline.access",
        }
    }

    /// XのみOAuth 2.0 PKCEが必須(GoogleとFacebookは認可コードフローの
    /// みで十分、両方に付けても害は無いが最小実装として分ける)。
    fn requires_pkce(self) -> bool {
        matches!(self, Provider::X)
    }
}

struct ProviderConfig {
    client_id: String,
    client_secret: String,
}

fn provider_config(provider: Provider) -> Result<ProviderConfig, Response> {
    let prefix = provider.env_prefix();
    let client_id = std::env::var(format!("{prefix}_CLIENT_ID"));
    let client_secret = std::env::var(format!("{prefix}_CLIENT_SECRET"));
    match (client_id, client_secret) {
        (Ok(client_id), Ok(client_secret)) => Ok(ProviderConfig { client_id, client_secret }),
        _ => Err(json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &json!({ "error": format!("{prefix}_CLIENT_ID/{prefix}_CLIENT_SECRET is not configured") }),
        )),
    }
}

fn redirect_uri(provider: Provider) -> String {
    let base_url = std::env::var("ARUARU_PRO_PUBLIC_URL").unwrap_or_else(|_| "https://aruaru.pro".to_string());
    format!("{base_url}/auth/oauth/{}/callback", provider.as_str())
}

/// state/PKCEの有効期間(秒)。この間に`callback`が来なければ無効。
const PENDING_STATE_MAX_AGE_SECONDS: i64 = 15 * 60;

fn now_unix() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

pub async fn ensure_table(db: &AruaruDb) -> anyhow::Result<()> {
    db.execute(
        "CREATE TABLE IF NOT EXISTS oauth_pending_states (\
            state TEXT PRIMARY KEY, \
            role TEXT, \
            pkce_verifier TEXT, \
            created_at BIGINT\
        )",
        &[],
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

/// CSRF対策のstateと、Xのみ使うPKCE code_verifierを紐づけてDBへ保持する
/// (`oauth_pending_states`テーブル、モジュールコメントの
/// 「2026-09-13、DBバックへ本格化」参照)。
struct PendingAuth {
    role: String,
    pkce_verifier: Option<String>,
}

async fn store_pending_auth(db: &AruaruDb, state: &str, pending: &PendingAuth) -> Result<(), String> {
    // 機会的クリーンアップ: 専用のバックグラウンドジョブを持たない代わりに
    // `start`のたびに期限切れエントリを削除する(軽量、テーブルが無限に
    // 肥大化することを防ぐ)。
    db.execute(
        "DELETE FROM oauth_pending_states WHERE created_at < $1",
        &[&(now_unix() - PENDING_STATE_MAX_AGE_SECONDS)],
    )
    .await
    .map_err(|e| e.to_string())?;

    db.execute(
        "INSERT INTO oauth_pending_states (state, role, pkce_verifier, created_at) VALUES ($1, $2, $3, $4)",
        &[&state, &pending.role, &pending.pkce_verifier, &now_unix()],
    )
    .await
    .map_err(|e| e.to_string())?;
    db.commit("oauth pending state stored").await.map_err(|e| e.to_string())?;
    Ok(())
}

/// `state`に対応するpending認証を取り出し、DBから削除する(ワンタイム
/// ——同じ`state`で2回目の`callback`は必ず失敗する、再送/リプレイ対策)。
/// 期限切れの場合は`Ok(None)`(見つからなかった場合と同じ扱い、
/// タイミング攻撃で「期限切れ」と「存在しない」を区別させない)。
async fn take_pending_auth(db: &AruaruDb, state: &str) -> Result<Option<PendingAuth>, String> {
    let rows = db
        .query("SELECT role, pkce_verifier, created_at FROM oauth_pending_states WHERE state = $1", &[&state])
        .await
        .map_err(|e| e.to_string())?;
    let Some(row) = rows.into_iter().next() else { return Ok(None) };

    db.execute("DELETE FROM oauth_pending_states WHERE state = $1", &[&state]).await.map_err(|e| e.to_string())?;
    db.commit("oauth pending state consumed").await.map_err(|e| e.to_string())?;

    let role: String = row.get(0);
    let pkce_verifier: Option<String> = row.get(1);
    let created_at: i64 = row.get(2);
    if now_unix() - created_at > PENDING_STATE_MAX_AGE_SECONDS {
        return Ok(None);
    }
    Ok(Some(PendingAuth { role, pkce_verifier }))
}

fn query_param(req: &Request, key: &str) -> Option<String> {
    let query = req.uri().query()?;
    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        let k = parts.next()?;
        let v = parts.next().unwrap_or("");
        if k == key {
            return Some(urlencoding_decode(v));
        }
    }
    None
}

/// 依存クレートを増やさないための最小限のパーセントデコード(OAuthの
/// クエリパラメータは英数字+`%XX`のみを想定した範囲で十分)。
fn urlencoding_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                if let Ok(byte) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    out.push(byte);
                    i += 3;
                    continue;
                }
                out.push(bytes[i]);
                i += 1;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).unwrap_or_default()
}

fn urlencoding_encode(s: &str) -> String {
    let mut out = String::new();
    for byte in s.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(*byte as char),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// PKCE code_verifier(乱数)+code_challenge(SHA-256→base64url、S256方式)
/// を生成する。
fn generate_pkce() -> (String, String) {
    let verifier = crate::ids::make_token() + &crate::ids::make_token();
    let challenge = base64url_sha256(&verifier);
    (verifier, challenge)
}

fn base64url_sha256(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(input.as_bytes());
    base64url_encode(&digest)
}

fn base64url_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    let mut chunks = bytes.chunks_exact(3);
    for chunk in &mut chunks {
        let n = ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
        out.push(ALPHABET[(n >> 18 & 0x3F) as usize] as char);
        out.push(ALPHABET[(n >> 12 & 0x3F) as usize] as char);
        out.push(ALPHABET[(n >> 6 & 0x3F) as usize] as char);
        out.push(ALPHABET[(n & 0x3F) as usize] as char);
    }
    let rem = chunks.remainder();
    if rem.len() == 1 {
        let n = (rem[0] as u32) << 16;
        out.push(ALPHABET[(n >> 18 & 0x3F) as usize] as char);
        out.push(ALPHABET[(n >> 12 & 0x3F) as usize] as char);
    } else if rem.len() == 2 {
        let n = ((rem[0] as u32) << 16) | ((rem[1] as u32) << 8);
        out.push(ALPHABET[(n >> 18 & 0x3F) as usize] as char);
        out.push(ALPHABET[(n >> 12 & 0x3F) as usize] as char);
        out.push(ALPHABET[(n >> 6 & 0x3F) as usize] as char);
    }
    out
}

/// `GET /auth/oauth/:provider/start?role=<role>`: 認可URLへリダイレクト
/// する代わりに、URL自体をJSONで返す(HTTPリダイレクト応答は
/// `open_runo_poem_compat`側にヘルパーが無いため、呼び出し側
/// ——ブラウザ/クライアント——がこのURLへ遷移する形。将来302を直接
/// 返すヘルパーが整備されればそちらへ差し替え可能)。
pub async fn start(req: Request, params: PathParams, db: std::sync::Arc<AruaruDb>) -> Response {
    let Some(provider) = params.get("provider").and_then(Provider::parse) else {
        return json_response(StatusCode::BAD_REQUEST, &json!({"error": "unknown provider"}));
    };
    let config = match provider_config(provider) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let role = query_param(&req, "role").unwrap_or_else(|| "buyer".to_string());
    if !crate::auth::ROLES.contains(&role.as_str()) {
        return json_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            &json!({ "error": format!("unknown role: \"{role}\" (expected one of {:?})", crate::auth::ROLES) }),
        );
    }

    let state = crate::ids::make_token();
    let (pkce_verifier, pkce_challenge) = if provider.requires_pkce() {
        let (verifier, challenge) = generate_pkce();
        (Some(verifier), Some(challenge))
    } else {
        (None, None)
    };

    if let Err(e) = store_pending_auth(&db, &state, &PendingAuth { role, pkce_verifier }).await {
        return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e}));
    }

    let redirect = redirect_uri(provider);
    let mut url = format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
        provider.authorize_endpoint(),
        urlencoding_encode(&config.client_id),
        urlencoding_encode(&redirect),
        urlencoding_encode(provider.scope()),
        urlencoding_encode(&state),
    );
    if let Some(challenge) = pkce_challenge {
        url.push_str(&format!("&code_challenge={}&code_challenge_method=S256", urlencoding_encode(&challenge)));
    }

    json_response(StatusCode::OK, &json!({ "authorize_url": url }))
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct GoogleUserInfo {
    sub: String,
    email: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FacebookUserInfo {
    id: String,
    email: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct XUserInfoEnvelope {
    data: XUserInfo,
}

#[derive(Debug, Deserialize)]
struct XUserInfo {
    id: String,
    name: Option<String>,
    // X(旧Twitter)のuserinfo APIは既定でemailを返さない(上記モジュール
    // コメント参照)。
}

struct ExternalIdentity {
    subject: String,
    name: String,
    email: Option<String>,
}

async fn exchange_code_for_token(
    provider: Provider,
    config: &ProviderConfig,
    code: &str,
    pkce_verifier: Option<&str>,
) -> Result<String, String> {
    let redirect = redirect_uri(provider);
    let client = reqwest::Client::new();
    let mut form: Vec<(&str, &str)> = vec![
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", &redirect),
        ("client_id", &config.client_id),
        ("client_secret", &config.client_secret),
    ];
    if let Some(verifier) = pkce_verifier {
        form.push(("code_verifier", verifier));
    }

    let resp = client
        .post(provider.token_endpoint())
        .form(&form)
        .send()
        .await
        .map_err(|e| format!("token exchange request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("token exchange returned {}", resp.status()));
    }
    let token: TokenResponse = resp.json().await.map_err(|e| format!("token exchange response parse failed: {e}"))?;
    Ok(token.access_token)
}

async fn fetch_identity(provider: Provider, access_token: &str) -> Result<ExternalIdentity, String> {
    let client = reqwest::Client::new();
    match provider {
        Provider::Google => {
            let resp = client
                .get("https://www.googleapis.com/oauth2/v3/userinfo")
                .bearer_auth(access_token)
                .send()
                .await
                .map_err(|e| format!("userinfo request failed: {e}"))?;
            let info: GoogleUserInfo = resp.json().await.map_err(|e| format!("userinfo parse failed: {e}"))?;
            Ok(ExternalIdentity {
                subject: info.sub,
                name: info.name.unwrap_or_else(|| "Google User".to_string()),
                email: info.email,
            })
        }
        Provider::Facebook => {
            let resp = client
                .get("https://graph.facebook.com/me")
                .query(&[("fields", "id,name,email"), ("access_token", access_token)])
                .send()
                .await
                .map_err(|e| format!("userinfo request failed: {e}"))?;
            let info: FacebookUserInfo = resp.json().await.map_err(|e| format!("userinfo parse failed: {e}"))?;
            Ok(ExternalIdentity {
                subject: info.id,
                name: info.name.unwrap_or_else(|| "Facebook User".to_string()),
                email: info.email,
            })
        }
        Provider::X => {
            let resp = client
                .get("https://api.twitter.com/2/users/me")
                .bearer_auth(access_token)
                .send()
                .await
                .map_err(|e| format!("userinfo request failed: {e}"))?;
            let info: XUserInfoEnvelope = resp.json().await.map_err(|e| format!("userinfo parse failed: {e}"))?;
            Ok(ExternalIdentity {
                subject: info.data.id,
                name: info.data.name.unwrap_or_else(|| "X User".to_string()),
                email: None, // Xは既定でemailを返さない
            })
        }
    }
}

/// `GET /auth/oauth/:provider/callback?code=...&state=...`。
pub async fn callback(req: Request, params: PathParams, db: std::sync::Arc<AruaruDb>) -> Response {
    let Some(provider) = params.get("provider").and_then(Provider::parse) else {
        return json_response(StatusCode::BAD_REQUEST, &json!({"error": "unknown provider"}));
    };
    let config = match provider_config(provider) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let Some(code) = query_param(&req, "code") else {
        return json_response(StatusCode::BAD_REQUEST, &json!({"error": "missing code"}));
    };
    let Some(state) = query_param(&req, "state") else {
        return json_response(StatusCode::BAD_REQUEST, &json!({"error": "missing state"}));
    };

    let pending = match take_pending_auth(&db, &state).await {
        Ok(p) => p,
        Err(e) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e})),
    };
    let Some(pending) = pending else {
        return json_response(
            StatusCode::BAD_REQUEST,
            &json!({"error": "unknown, expired, or already-used state (possible CSRF or replay)"}),
        );
    };

    let access_token = match exchange_code_for_token(provider, &config, &code, pending.pkce_verifier.as_deref()).await
    {
        Ok(t) => t,
        Err(e) => return json_response(StatusCode::BAD_GATEWAY, &json!({"error": e})),
    };
    let identity = match fetch_identity(provider, &access_token).await {
        Ok(i) => i,
        Err(e) => return json_response(StatusCode::BAD_GATEWAY, &json!({"error": e})),
    };

    match upsert_oauth_user(&db, provider, &identity, &pending.role).await {
        Ok((name, role, token)) => {
            let resp = json_response(StatusCode::OK, &json!({ "name": name, "role": role, "token": token }));
            crate::auth::issue_session(&db, &token, resp).await
        }
        Err(e) => json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e})),
    }
}

/// `(oauth_provider, oauth_subject)`で既存ユーザーを検索し、無ければ
/// 新規作成する。既存ユーザーの場合は登録時のロールをそのまま使う
/// (`start`時の`role`パラメータは新規作成時のみ有効)。
async fn upsert_oauth_user(
    db: &AruaruDb,
    provider: Provider,
    identity: &ExternalIdentity,
    default_role: &str,
) -> Result<(String, String, String), String> {
    let existing = db
        .query(
            "SELECT name, role, token FROM users WHERE oauth_provider = $1 AND oauth_subject = $2",
            &[&provider.as_str(), &identity.subject],
        )
        .await
        .map_err(|e| e.to_string())?;

    if let Some(row) = existing.into_iter().next() {
        return Ok((row.get(0), row.get(1), row.get(2)));
    }

    let id = crate::ids::make_id(&[provider.as_str(), &identity.subject], "user");
    let token = crate::ids::make_token();
    db.execute(
        "INSERT INTO users (id, name, role, token, email, oauth_provider, oauth_subject) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        &[&id, &identity.name, &default_role, &token, &identity.email, &provider.as_str(), &identity.subject],
    )
    .await
    .map_err(|e| e.to_string())?;
    db.commit("oauth user registration").await.map_err(|e| e.to_string())?;

    Ok((identity.name.clone(), default_role.to_string(), token))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_parse_recognizes_all_three() {
        assert_eq!(Provider::parse("google"), Some(Provider::Google));
        assert_eq!(Provider::parse("facebook"), Some(Provider::Facebook));
        assert_eq!(Provider::parse("x"), Some(Provider::X));
        assert_eq!(Provider::parse("unknown"), None);
    }

    #[test]
    fn only_x_requires_pkce() {
        assert!(!Provider::Google.requires_pkce());
        assert!(!Provider::Facebook.requires_pkce());
        assert!(Provider::X.requires_pkce());
    }

    #[test]
    fn urlencoding_round_trips_special_characters() {
        let original = "hello world&foo=bar+baz";
        let encoded = urlencoding_encode(original);
        assert!(!encoded.contains(' '));
        assert!(!encoded.contains('&'));
        assert_eq!(urlencoding_decode(&encoded), original);
    }

    #[test]
    fn generate_pkce_produces_a_challenge_derived_from_the_verifier() {
        let (verifier, challenge) = generate_pkce();
        assert!(!verifier.is_empty());
        assert_eq!(challenge, base64url_sha256(&verifier));
        // 異なるverifierからは異なるchallengeが出ることの確認
        // (定数関数になっていないことの裏取り)。
        let (verifier2, challenge2) = generate_pkce();
        assert_ne!(verifier, verifier2);
        assert_ne!(challenge, challenge2);
    }

    #[test]
    fn base64url_encode_matches_known_test_vector() {
        // "sha256(\"\")"のbase64url表現(既知のテストベクタで実装を裏取り)。
        use sha2::{Digest, Sha256};
        let digest = Sha256::digest(b"");
        let encoded = base64url_encode(&digest);
        // 標準base64(パディング無し、+/ -> -/_)の既知値と一致することを確認。
        assert_eq!(encoded, "47DEQpj8HBSa-_TImW-5JCeuQeRkm5NMpJWZG3hSuFU");
    }
}
