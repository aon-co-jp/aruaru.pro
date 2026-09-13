//! 出品(Service)のCRUD。`job-site`のjobs実装パターン(RS-JSONでの
//! デコード、slugifyしたidの生成、`AruaruDb::commit`によるGit-on-SQLの
//! バージョン管理)をそのまま踏襲する。
//!
//! **現状のスコープ(第一段)**: 出品の作成/一覧/単体取得のみ。
//! 決済(Stripe Connect)・注文(Order)・出品者の認証/プロフィールとの
//! 紐付けは未着手(`CLAUDE.md`の「次にすべきこと」参照)。

use std::sync::Arc;

use aruaru_db_connector::AruaruDb;
use open_runo_poem_compat::hyper_compat::json_response;
use open_runo_poem_compat::{PathParams, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::json_body::read_json_body;

pub async fn ensure_table(db: &AruaruDb) -> anyhow::Result<()> {
    db.execute(
        "CREATE TABLE IF NOT EXISTS services (\
            id TEXT PRIMARY KEY, \
            seller_name TEXT, \
            category TEXT, \
            title TEXT, \
            description TEXT, \
            price_yen INTEGER\
        )",
        &[],
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct UpsertServiceRequest {
    seller_name: String,
    category: String,
    title: String,
    description: String,
    price_yen: i32,
    #[serde(default = "default_commit_message")]
    message: String,
}

fn default_commit_message() -> String {
    "service listing update".to_string()
}

#[derive(Debug, Serialize)]
struct UpsertServiceResponse {
    id: String,
    commit_id: String,
}

/// 必須フィールドの空文字チェック+価格の妥当性チェック(カテゴリの
/// 実在チェックは含まない——コンパイル時定数だけでは、DBへ後から
/// 追加されたカテゴリを弾いてしまうため、`upsert_service`側で別途
/// `category_is_known`(同期/高速)→DBフォールバックの2段で確認する)。
/// (RS-JSONの型検証は必須フィールド欠如は検出するが、空文字や負の
/// 価格はすり抜けるため、`job-site`の`validate_upsert`と同じ理由で
/// 別途弾く)。
fn validate_upsert_fields(body: &UpsertServiceRequest) -> Option<Response> {
    let bad_field = if body.seller_name.trim().is_empty() {
        Some("seller_name")
    } else if body.title.trim().is_empty() {
        Some("title")
    } else if body.description.trim().is_empty() {
        Some("description")
    } else if body.category.trim().is_empty() {
        Some("category")
    } else {
        None
    };
    if let Some(field) = bad_field {
        return Some(json_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            &json!({ "error": format!("field \"{field}\" must not be empty") }),
        ));
    }
    if body.price_yen <= 0 {
        return Some(json_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            &json!({ "error": "price_yen must be a positive integer" }),
        ));
    }
    None
}

/// コンパイル時定数(`categories::CATEGORIES`)にカテゴリが存在するかを
/// I/O無しで確認する高速パス。DBには存在するが定数には無い(=後から
/// 追加された)カテゴリは`false`を返す——呼び出し側はその場合のみDBへ
/// フォールバックする(`categories::category_exists_in_db`)。
fn category_is_known_statically(category: &str) -> bool {
    crate::categories::CATEGORIES.iter().any(|c| c.name == category)
}

pub async fn upsert_service(req: Request, db: Arc<AruaruDb>) -> Response {
    let user = match crate::auth::authenticate(&req, &db).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    if let Some(resp) = crate::auth::require_role(&user, "seller") {
        return resp;
    }

    let body = match read_json_body::<UpsertServiceRequest>(req).await {
        Ok(b) => b,
        Err(resp) => return resp,
    };
    if let Some(resp) = validate_upsert_fields(&body) {
        return resp;
    }
    if let Some(resp) = crate::auth::require_name_matches(&user, "seller_name", &body.seller_name) {
        return resp;
    }
    if !category_is_known_statically(&body.category) {
        match crate::categories::category_exists_in_db(&db, &body.category).await {
            Ok(true) => {}
            Ok(false) => {
                return json_response(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    &json!({ "error": format!("unknown category: \"{}\"", body.category) }),
                )
            }
            Err(e) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e})),
        }
    }

    // id生成の日本語衝突対策は`crate::ids::make_id`参照。
    let id = crate::ids::make_id(&[&body.seller_name, &body.title], "svc");

    if let Err(e) = db
        .execute(
            "INSERT INTO services (id, seller_name, category, title, description, price_yen) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             ON CONFLICT (id) DO UPDATE SET seller_name = EXCLUDED.seller_name, \
             category = EXCLUDED.category, title = EXCLUDED.title, \
             description = EXCLUDED.description, price_yen = EXCLUDED.price_yen",
            &[&id, &body.seller_name, &body.category, &body.title, &body.description, &body.price_yen],
        )
        .await
    {
        return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()}));
    }

    let commit_id = match db.commit(&body.message).await {
        Ok(cid) => cid,
        Err(e) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()})),
    };

    json_response(StatusCode::OK, &UpsertServiceResponse { id, commit_id })
}

pub async fn list_services(db: Arc<AruaruDb>) -> Response {
    match db.query("SELECT id, seller_name, category, title, description, price_yen FROM services", &[]).await {
        Ok(rows) => {
            let services: Vec<_> = rows.iter().map(row_to_json).collect();
            json_response(StatusCode::OK, &json!({"services": services}))
        }
        Err(e) => json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()})),
    }
}

pub async fn get_service(_req: Request, params: PathParams, db: Arc<AruaruDb>) -> Response {
    let Some(id) = params.get("id") else {
        return json_response(StatusCode::BAD_REQUEST, &json!({"error": "missing id"}));
    };
    match db
        .query("SELECT id, seller_name, category, title, description, price_yen FROM services WHERE id = $1", &[&id])
        .await
    {
        Ok(rows) => match rows.into_iter().next() {
            Some(r) => json_response(StatusCode::OK, &row_to_json(&r)),
            None => json_response(StatusCode::NOT_FOUND, &json!({"error": "service not found"})),
        },
        Err(e) => json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()})),
    }
}

fn row_to_json(r: &tokio_postgres::Row) -> serde_json::Value {
    json!({
        "id": r.get::<_, String>(0),
        "seller_name": r.get::<_, String>(1),
        "category": r.get::<_, String>(2),
        "title": r.get::<_, String>(3),
        "description": r.get::<_, String>(4),
        "price_yen": r.get::<_, i32>(5),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // id生成のテストは`crate::ids`側に集約済み(共通ヘルパー化に伴い移動)。

    #[test]
    fn validate_upsert_rejects_empty_required_fields() {
        let body = UpsertServiceRequest {
            seller_name: "  ".to_string(),
            category: "デザイン制作".to_string(),
            title: "ロゴ制作".to_string(),
            description: "説明".to_string(),
            price_yen: 1000,
            message: default_commit_message(),
        };
        assert!(validate_upsert_fields(&body).is_some());
    }

    #[test]
    fn validate_upsert_rejects_non_positive_price() {
        let body = UpsertServiceRequest {
            seller_name: "鈴木一郎".to_string(),
            category: "デザイン制作".to_string(),
            title: "ロゴ制作".to_string(),
            description: "説明".to_string(),
            price_yen: 0,
            message: default_commit_message(),
        };
        assert!(validate_upsert_fields(&body).is_some());
    }

    #[test]
    fn category_is_known_statically_rejects_unknown_category() {
        assert!(!category_is_known_statically("存在しないカテゴリ"));
    }

    #[test]
    fn category_is_known_statically_accepts_every_seeded_category() {
        for cat in crate::categories::CATEGORIES {
            assert!(category_is_known_statically(cat.name), "category {} must be known statically", cat.name);
        }
    }

    #[test]
    fn validate_upsert_accepts_a_well_formed_request() {
        let body = UpsertServiceRequest {
            seller_name: "鈴木一郎".to_string(),
            category: "デザイン制作".to_string(),
            title: "ロゴ制作".to_string(),
            description: "説明".to_string(),
            price_yen: 5000,
            message: default_commit_message(),
        };
        assert!(validate_upsert_fields(&body).is_none());
    }
}
