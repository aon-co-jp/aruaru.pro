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

/// 必須フィールドの空文字チェック+価格の妥当性チェック。
/// (RS-JSONの型検証は必須フィールド欠如は検出するが、空文字や負の
/// 価格はすり抜けるため、`job-site`の`validate_upsert`と同じ理由で
/// 別途弾く)。
fn validate_upsert(body: &UpsertServiceRequest) -> Option<Response> {
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
    if !crate::categories::CATEGORIES.iter().any(|c| c.name == body.category) {
        return Some(json_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            &json!({ "error": format!("unknown category: \"{}\"", body.category) }),
        ));
    }
    None
}

pub async fn upsert_service(req: Request, db: Arc<AruaruDb>) -> Response {
    let body = match read_json_body::<UpsertServiceRequest>(req).await {
        Ok(b) => b,
        Err(resp) => return resp,
    };
    if let Some(resp) = validate_upsert(&body) {
        return resp;
    }

    // **job-siteのslugifyパターンを転用しない理由**: job-siteのslugify
    // (英数字以外を`-`に潰す)は日本語の出品者名・タイトルに対しては
    // 全て空文字になってしまい、異なる出品が同じid(空文字同士の連結)
    // へ衝突する——このサービスは日本語が主言語のため、素朴なslugify
    // 転用は実際にデータを壊す。ハッシュを必ず付与して一意性を保証し、
    // ASCII成分がある場合のみ読みやすいslugを前置する。
    let id = make_service_id(&body.seller_name, &body.title);

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

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// 出品idを、`seller_name`+`title`のハッシュを必ず付与して生成する。
/// ASCII成分があれば読みやすさのためslugを前置するが、一意性の保証は
/// 常にハッシュ側が担う(日本語名同士がslugify後にどちらも空文字へ
/// 潰れて衝突する、というjob-site由来の実バグを踏まない設計)。
pub fn make_service_id(seller_name: &str, title: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    seller_name.hash(&mut hasher);
    0u8.hash(&mut hasher); // 区切り: "ab"+"c" と "a"+"bc" のハッシュ衝突を避ける
    title.hash(&mut hasher);
    let hash = hasher.finish();

    let seller_slug = slugify(seller_name);
    let title_slug = slugify(title);
    if seller_slug.is_empty() && title_slug.is_empty() {
        format!("svc-{hash:016x}")
    } else {
        format!("{seller_slug}--{title_slug}-{hash:016x}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_service_id_does_not_collide_for_different_japanese_names() {
        // job-site由来のslugify転用がそのまま日本語名に効いていたら、
        // どちらも空文字同士の連結で衝突していたはずの組み合わせ。
        let id_a = make_service_id("鈴木一郎", "ロゴ制作");
        let id_b = make_service_id("佐藤花子", "ロゴ制作");
        assert_ne!(id_a, id_b, "different Japanese seller names must not collide");
    }

    #[test]
    fn make_service_id_is_deterministic_for_the_same_input() {
        let id_a = make_service_id("鈴木一郎", "ロゴ制作");
        let id_b = make_service_id("鈴木一郎", "ロゴ制作");
        assert_eq!(id_a, id_b);
    }

    #[test]
    fn make_service_id_prefixes_a_readable_slug_when_ascii_is_present() {
        let id = make_service_id("Taro Suzuki", "Logo Design");
        assert!(id.starts_with("taro-suzuki--logo-design-"), "got: {id}");
    }

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
        assert!(validate_upsert(&body).is_some());
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
        assert!(validate_upsert(&body).is_some());
    }

    #[test]
    fn validate_upsert_rejects_unknown_category() {
        let body = UpsertServiceRequest {
            seller_name: "鈴木一郎".to_string(),
            category: "存在しないカテゴリ".to_string(),
            title: "ロゴ制作".to_string(),
            description: "説明".to_string(),
            price_yen: 1000,
            message: default_commit_message(),
        };
        assert!(validate_upsert(&body).is_some());
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
        assert!(validate_upsert(&body).is_none());
    }
}
