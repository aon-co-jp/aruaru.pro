//! 求人・アルバイト情報(JobListing)。出品(Service)とは別のエンティティ
//! ——スキル出品(価格・カテゴリ)とは異なるフィールド(勤務地・時給・
//! 雇用形態)を持つため、`services`テーブルを再利用せず専用モデルにする
//! (`CLAUDE.md`のスコープ節参照: 「求人・アルバイト情報」はcoconalaには
//! 無いトップレベルカテゴリとして追加された)。

use std::sync::Arc;

use aruaru_db_connector::AruaruDb;
use open_runo_poem_compat::hyper_compat::json_response;
use open_runo_poem_compat::{PathParams, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::json_body::read_json_body;

/// 雇用形態。固定の一覧から選ばせる(自由記述だと表記揺れ——「アルバイト」
/// 「バイト」「Part-time」等——で検索/絞り込みが機能しなくなるため)。
const EMPLOYMENT_TYPES: &[&str] = &["正社員", "契約社員", "アルバイト", "パート", "業務委託", "派遣"];

pub async fn ensure_table(db: &AruaruDb) -> anyhow::Result<()> {
    db.execute(
        "CREATE TABLE IF NOT EXISTS job_listings (\
            id TEXT PRIMARY KEY, \
            employer_name TEXT, \
            title TEXT, \
            description TEXT, \
            location TEXT, \
            employment_type TEXT, \
            hourly_wage_yen INTEGER\
        )",
        &[],
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct UpsertJobListingRequest {
    employer_name: String,
    title: String,
    description: String,
    location: String,
    employment_type: String,
    hourly_wage_yen: i32,
    #[serde(default = "default_commit_message")]
    message: String,
}

fn default_commit_message() -> String {
    "job listing update".to_string()
}

#[derive(Debug, Serialize)]
struct UpsertJobListingResponse {
    id: String,
    commit_id: String,
}

fn validate_upsert(body: &UpsertJobListingRequest) -> Option<Response> {
    let bad_field = if body.employer_name.trim().is_empty() {
        Some("employer_name")
    } else if body.title.trim().is_empty() {
        Some("title")
    } else if body.description.trim().is_empty() {
        Some("description")
    } else if body.location.trim().is_empty() {
        Some("location")
    } else {
        None
    };
    if let Some(field) = bad_field {
        return Some(json_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            &json!({ "error": format!("field \"{field}\" must not be empty") }),
        ));
    }
    if !EMPLOYMENT_TYPES.contains(&body.employment_type.as_str()) {
        return Some(json_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            &json!({ "error": format!("unknown employment_type: \"{}\" (expected one of {EMPLOYMENT_TYPES:?})", body.employment_type) }),
        ));
    }
    if body.hourly_wage_yen <= 0 {
        return Some(json_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            &json!({ "error": "hourly_wage_yen must be a positive integer" }),
        ));
    }
    None
}

pub async fn upsert_job_listing(req: Request, db: Arc<AruaruDb>) -> Response {
    let user = match crate::auth::authenticate(&req, &db).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    if let Some(resp) = crate::auth::require_role(&user, "recruiter") {
        return resp;
    }

    let body = match read_json_body::<UpsertJobListingRequest>(req).await {
        Ok(b) => b,
        Err(resp) => return resp,
    };
    if let Some(resp) = validate_upsert(&body) {
        return resp;
    }
    if let Some(resp) = crate::auth::require_name_matches(&user, "employer_name", &body.employer_name) {
        return resp;
    }

    let id = crate::ids::make_id(&[&body.employer_name, &body.title], "job");

    if let Err(e) = db
        .execute(
            "INSERT INTO job_listings \
             (id, employer_name, title, description, location, employment_type, hourly_wage_yen) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) \
             ON CONFLICT (id) DO UPDATE SET employer_name = EXCLUDED.employer_name, \
             title = EXCLUDED.title, description = EXCLUDED.description, \
             location = EXCLUDED.location, employment_type = EXCLUDED.employment_type, \
             hourly_wage_yen = EXCLUDED.hourly_wage_yen",
            &[
                &id,
                &body.employer_name,
                &body.title,
                &body.description,
                &body.location,
                &body.employment_type,
                &body.hourly_wage_yen,
            ],
        )
        .await
    {
        return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()}));
    }

    let commit_id = match db.commit(&body.message).await {
        Ok(cid) => cid,
        Err(e) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()})),
    };

    json_response(StatusCode::OK, &UpsertJobListingResponse { id, commit_id })
}

pub async fn list_job_listings(db: Arc<AruaruDb>) -> Response {
    match db
        .query(
            "SELECT id, employer_name, title, description, location, employment_type, hourly_wage_yen \
             FROM job_listings",
            &[],
        )
        .await
    {
        Ok(rows) => {
            let jobs: Vec<_> = rows.iter().map(row_to_json).collect();
            json_response(StatusCode::OK, &json!({"job_listings": jobs}))
        }
        Err(e) => json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()})),
    }
}

pub async fn get_job_listing(_req: Request, params: PathParams, db: Arc<AruaruDb>) -> Response {
    let Some(id) = params.get("id") else {
        return json_response(StatusCode::BAD_REQUEST, &json!({"error": "missing id"}));
    };
    match db
        .query(
            "SELECT id, employer_name, title, description, location, employment_type, hourly_wage_yen \
             FROM job_listings WHERE id = $1",
            &[&id],
        )
        .await
    {
        Ok(rows) => match rows.into_iter().next() {
            Some(r) => json_response(StatusCode::OK, &row_to_json(&r)),
            None => json_response(StatusCode::NOT_FOUND, &json!({"error": "job listing not found"})),
        },
        Err(e) => json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()})),
    }
}

fn row_to_json(r: &tokio_postgres::Row) -> serde_json::Value {
    json!({
        "id": r.get::<_, String>(0),
        "employer_name": r.get::<_, String>(1),
        "title": r.get::<_, String>(2),
        "description": r.get::<_, String>(3),
        "location": r.get::<_, String>(4),
        "employment_type": r.get::<_, String>(5),
        "hourly_wage_yen": r.get::<_, i32>(6),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn well_formed_request() -> UpsertJobListingRequest {
        UpsertJobListingRequest {
            employer_name: "株式会社サンプル".to_string(),
            title: "接客スタッフ".to_string(),
            description: "カフェでの接客業務".to_string(),
            location: "東京都渋谷区".to_string(),
            employment_type: "アルバイト".to_string(),
            hourly_wage_yen: 1200,
            message: default_commit_message(),
        }
    }

    #[test]
    fn validate_upsert_accepts_a_well_formed_request() {
        assert!(validate_upsert(&well_formed_request()).is_none());
    }

    #[test]
    fn validate_upsert_rejects_empty_required_fields() {
        let mut body = well_formed_request();
        body.location = "  ".to_string();
        assert!(validate_upsert(&body).is_some());
    }

    #[test]
    fn validate_upsert_rejects_unknown_employment_type() {
        let mut body = well_formed_request();
        body.employment_type = "フリーランス".to_string();
        assert!(validate_upsert(&body).is_some());
    }

    #[test]
    fn validate_upsert_rejects_non_positive_wage() {
        let mut body = well_formed_request();
        body.hourly_wage_yen = 0;
        assert!(validate_upsert(&body).is_some());
    }

    #[test]
    fn validate_upsert_accepts_every_known_employment_type() {
        for employment_type in EMPLOYMENT_TYPES {
            let mut body = well_formed_request();
            body.employment_type = employment_type.to_string();
            assert!(validate_upsert(&body).is_none(), "employment_type {employment_type} must be accepted");
        }
    }
}
