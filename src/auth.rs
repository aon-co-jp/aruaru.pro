//! 認証: ロール別のBearerトークン認証+DBバックのCookieセッション。
//!
//! `POST /auth/register`で名前+ロールを渡すとトークンが即発行される
//! 最小の登録経路(パスワード・メール確認等は無い、開発/テスト用途向け)
//! に加え、2026-09-13にGoogle(Gmail)/Facebook/X(旧Twitter)の
//! OAuth 2.0ログイン(`oauth.rs`)を追加し、実在のメールアドレス/
//! アカウントで本人確認する経路も提供する(ユーザー指示「GmailやFacebook
//! やXのOAuth認証は実装して」)。どちらの経路でも最終的に同じ`users`
//! テーブル・同じ形式のBearerトークンを発行する。ロールは
//! 出品者(`seller`)・依頼者(`buyer`)・求職者(`job_seeker`)・
//! 採用担当(`recruiter`)の4種(ユーザー指示「出品者/依頼者/求職者/
//! 採用担当のロール分け」、2026-09-13)。
//!
//! ## Cookieセッション(2026-09-13、DBバックへ本格化)
//!
//! 以前はOAuthのCSRF対策state/PKCEをプロセス内メモリ(`OnceLock<Mutex<
//! HashMap<..>>>`)に保持していたため、プロセス再起動やマルチインスタンス
//! 構成(ロードバランサ配下に複数プロセス)で機能しないという限界が
//! あった(ユーザー指摘、2026-09-13)。`sessions`テーブル(DB永続化、
//! `aruaru-db`は複数プロセスから共有接続できる)へ置き換え、ログイン
//! (`register`・`oauth::callback`)成功時に`Set-Cookie`でセッションIDを
//! 発行する。`authenticate`は`Authorization: Bearer`ヘッダ(既存のAPI
//! クライアント向け経路、後方互換で維持)→無ければ`Cookie`ヘッダの
//! セッションIDの順で認証を試みる。OAuthのstate/PKCEも同様にDBの
//! `oauth_pending_states`テーブルへ移した(`oauth.rs`参照)。

use std::sync::Arc;

use aruaru_db_connector::AruaruDb;
use hyper::header::{HeaderValue, COOKIE, SET_COOKIE};
use open_runo_poem_compat::hyper_compat::json_response;
use open_runo_poem_compat::{Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::json_body::read_json_body;

pub const ROLES: &[&str] = &["seller", "buyer", "job_seeker", "recruiter"];

const SESSION_COOKIE_NAME: &str = "aruaru_session";
/// セッションの有効期間(秒)。30日。
const SESSION_MAX_AGE_SECONDS: i64 = 60 * 60 * 24 * 30;

pub async fn ensure_table(db: &AruaruDb) -> anyhow::Result<()> {
    db.execute(
        "CREATE TABLE IF NOT EXISTS users (\
            id TEXT PRIMARY KEY, \
            name TEXT, \
            role TEXT, \
            token TEXT UNIQUE, \
            email TEXT, \
            oauth_provider TEXT, \
            oauth_subject TEXT\
        )",
        &[],
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    // 既存テーブル(OAuth新設前に作成済みのもの)への追従。Postgresの
    // `ADD COLUMN IF NOT EXISTS`は冪等なので複数回実行しても安全。
    for stmt in [
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS email TEXT",
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS oauth_provider TEXT",
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS oauth_subject TEXT",
    ] {
        db.execute(stmt, &[]).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    // Cookieセッション(DB永続化、プロセス再起動・マルチインスタンス
    // 構成でも機能する——`aruaru-db`への接続を持つプロセスなら誰でも
    // 同じセッションを検証できる)。
    db.execute(
        "CREATE TABLE IF NOT EXISTS sessions (\
            session_id TEXT PRIMARY KEY, \
            user_token TEXT, \
            created_at BIGINT\
        )",
        &[],
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

fn now_unix() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// ログイン成功時にセッションを新規発行し、`Set-Cookie`ヘッダを付けた
/// レスポンスを返す。`user_token`は`users.token`(既存のBearerトークンと
/// 同じ値)——セッションは「このBearerトークンをこのCookieで代理する」
/// という薄いマッピングに過ぎない設計(認可ロジック自体は変えない)。
pub async fn issue_session(db: &AruaruDb, user_token: &str, mut resp: Response) -> Response {
    let session_id = crate::ids::make_token();
    if let Err(e) = db
        .execute(
            "INSERT INTO sessions (session_id, user_token, created_at) VALUES ($1, $2, $3)",
            &[&session_id, &user_token, &now_unix()],
        )
        .await
    {
        return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()}));
    }
    if let Err(e) = db.commit("session issued").await {
        return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()}));
    }

    let cookie = format!(
        "{SESSION_COOKIE_NAME}={session_id}; Path=/; HttpOnly; SameSite=Lax; Max-Age={SESSION_MAX_AGE_SECONDS}"
    );
    if let Ok(value) = HeaderValue::from_str(&cookie) {
        resp.headers_mut().insert(SET_COOKIE, value);
    }
    resp
}

fn read_session_cookie(req: &Request) -> Option<String> {
    let header = req.headers().get(COOKIE)?.to_str().ok()?;
    for pair in header.split(';') {
        let pair = pair.trim();
        if let Some(value) = pair.strip_prefix(&format!("{SESSION_COOKIE_NAME}=")) {
            return Some(value.to_string());
        }
    }
    None
}

/// セッションIDから`users.token`を引く(期限切れの検証も行う——
/// `created_at`+`SESSION_MAX_AGE_SECONDS`を過ぎたセッションは無効)。
async fn resolve_session_token(db: &AruaruDb, session_id: &str) -> Result<Option<String>, String> {
    let rows = db
        .query("SELECT user_token, created_at FROM sessions WHERE session_id = $1", &[&session_id])
        .await
        .map_err(|e| e.to_string())?;
    let Some(row) = rows.into_iter().next() else { return Ok(None) };
    let user_token: String = row.get(0);
    let created_at: i64 = row.get(1);
    if now_unix() - created_at > SESSION_MAX_AGE_SECONDS {
        return Ok(None);
    }
    Ok(Some(user_token))
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    name: String,
    role: String,
}

#[derive(Debug, Serialize)]
struct RegisterResponse {
    id: String,
    name: String,
    role: String,
    token: String,
}

fn validate_register(body: &RegisterRequest) -> Option<Response> {
    if body.name.trim().is_empty() {
        return Some(json_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            &json!({ "error": "field \"name\" must not be empty" }),
        ));
    }
    if !ROLES.contains(&body.role.as_str()) {
        return Some(json_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            &json!({ "error": format!("unknown role: \"{}\" (expected one of {ROLES:?})", body.role) }),
        ));
    }
    None
}

pub async fn register(req: Request, db: Arc<AruaruDb>) -> Response {
    let body = match read_json_body::<RegisterRequest>(req).await {
        Ok(b) => b,
        Err(resp) => return resp,
    };
    if let Some(resp) = validate_register(&body) {
        return resp;
    }

    let id = crate::ids::make_id(&[&body.name, &body.role], "user");
    let token = crate::ids::make_token();

    if let Err(e) = db
        .execute(
            "INSERT INTO users (id, name, role, token) VALUES ($1, $2, $3, $4)",
            &[&id, &body.name, &body.role, &token],
        )
        .await
    {
        return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()}));
    }
    if let Err(e) = db.commit("user registration").await {
        return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()}));
    }

    let resp =
        json_response(StatusCode::OK, &RegisterResponse { id, name: body.name, role: body.role, token: token.clone() });
    issue_session(&db, &token, resp).await
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub name: String,
    pub role: String,
}

/// `Authorization: Bearer <token>`ヘッダ、無ければ`Cookie`ヘッダの
/// セッションからユーザーを解決する(どちらもヘッダを読むだけなので
/// リクエストボディはまだ消費しない——呼び出し側は認証確認後に
/// `read_json_body`でボディを読む)。API クライアント(Bearer)・
/// ブラウザ(Cookieセッション)の両方に対応する設計。
pub async fn authenticate(req: &Request, db: &AruaruDb) -> Result<AuthUser, Response> {
    let bearer_token = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::to_string);

    let token = match bearer_token {
        Some(t) => t,
        None => match read_session_cookie(req) {
            Some(session_id) => match resolve_session_token(db, &session_id).await {
                Ok(Some(t)) => t,
                Ok(None) => {
                    return Err(json_response(
                        StatusCode::UNAUTHORIZED,
                        &json!({"error": "session expired or not found"}),
                    ))
                }
                Err(e) => return Err(json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e}))),
            },
            None => {
                return Err(json_response(
                    StatusCode::UNAUTHORIZED,
                    &json!({"error": "missing Authorization header or session cookie"}),
                ))
            }
        },
    };

    match db.query("SELECT name, role FROM users WHERE token = $1", &[&token]).await {
        Ok(rows) => match rows.into_iter().next() {
            Some(r) => Ok(AuthUser { name: r.get(0), role: r.get(1) }),
            None => Err(json_response(StatusCode::UNAUTHORIZED, &json!({"error": "invalid token"}))),
        },
        Err(e) => Err(json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()}))),
    }
}

/// `user`が`role`を持つことを確認する。持たなければ403。
pub fn require_role(user: &AuthUser, role: &str) -> Option<Response> {
    if user.role == role {
        None
    } else {
        Some(json_response(
            StatusCode::FORBIDDEN,
            &json!({ "error": format!("this action requires role \"{role}\", but the authenticated user has role \"{}\"", user.role) }),
        ))
    }
}

/// なりすまし防止: リクエストボディ内の名前フィールド(例:
/// `seller_name`)が認証済みユーザー本人の名前と一致することを確認する。
/// 一致しなければ403(他人の名前を騙って出品/注文/レビューを作成できて
/// しまう実用性の穴を作らない)。
pub fn require_name_matches(user: &AuthUser, field_name: &str, claimed_name: &str) -> Option<Response> {
    if user.name == claimed_name {
        None
    } else {
        Some(json_response(
            StatusCode::FORBIDDEN,
            &json!({ "error": format!("field \"{field_name}\" must match the authenticated user's own name") }),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_register_rejects_empty_name() {
        let body = RegisterRequest { name: "  ".to_string(), role: "seller".to_string() };
        assert!(validate_register(&body).is_some());
    }

    #[test]
    fn validate_register_rejects_unknown_role() {
        let body = RegisterRequest { name: "山田太郎".to_string(), role: "admin".to_string() };
        assert!(validate_register(&body).is_some());
    }

    #[test]
    fn validate_register_accepts_every_known_role() {
        for role in ROLES {
            let body = RegisterRequest { name: "山田太郎".to_string(), role: role.to_string() };
            assert!(validate_register(&body).is_none(), "role {role} must be accepted");
        }
    }

    #[test]
    fn require_role_rejects_mismatched_role() {
        let user = AuthUser { name: "山田太郎".to_string(), role: "buyer".to_string() };
        assert!(require_role(&user, "seller").is_some());
    }

    #[test]
    fn require_role_accepts_matching_role() {
        let user = AuthUser { name: "山田太郎".to_string(), role: "seller".to_string() };
        assert!(require_role(&user, "seller").is_none());
    }

    #[test]
    fn require_name_matches_rejects_impersonation() {
        let user = AuthUser { name: "山田太郎".to_string(), role: "seller".to_string() };
        assert!(require_name_matches(&user, "seller_name", "鈴木一郎").is_some());
    }

    #[test]
    fn require_name_matches_accepts_own_name() {
        let user = AuthUser { name: "山田太郎".to_string(), role: "seller".to_string() };
        assert!(require_name_matches(&user, "seller_name", "山田太郎").is_none());
    }

    #[test]
    fn session_cookie_name_and_max_age_are_sane() {
        // Cookie文字列組み立てロジック自体はDB接続が要るため統合テスト
        // 側の検証範囲だが、定数の整合性(名前が空でない、期間が正の値)
        // だけはここで裏取りできる。
        assert!(!SESSION_COOKIE_NAME.is_empty());
        assert!(SESSION_MAX_AGE_SECONDS > 0);
    }

    #[test]
    fn now_unix_is_monotonically_reasonable() {
        let a = now_unix();
        let b = now_unix();
        assert!(b >= a);
        assert!(a > 0);
    }
}
