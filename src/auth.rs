//! 認証: ロール別のBearerトークン認証。
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

use std::sync::Arc;

use aruaru_db_connector::AruaruDb;
use open_runo_poem_compat::hyper_compat::json_response;
use open_runo_poem_compat::{Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::json_body::read_json_body;

pub const ROLES: &[&str] = &["seller", "buyer", "job_seeker", "recruiter"];

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
    Ok(())
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

    json_response(StatusCode::OK, &RegisterResponse { id, name: body.name, role: body.role, token })
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub name: String,
    pub role: String,
}

/// `Authorization: Bearer <token>`ヘッダからユーザーを解決する。
/// ヘッダを読むだけなのでリクエストボディはまだ消費しない
/// (呼び出し側は認証確認後に`read_json_body`でボディを読む)。
pub async fn authenticate(req: &Request, db: &AruaruDb) -> Result<AuthUser, Response> {
    let token = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::to_string);

    let Some(token) = token else {
        return Err(json_response(StatusCode::UNAUTHORIZED, &json!({"error": "missing Authorization header"})));
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
}
