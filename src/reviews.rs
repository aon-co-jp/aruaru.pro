//! レビュー(Review)。特定の出品(`service_id`)に対する評価。
//! **現状のスコープ(第一段)**: 注文(Order)との紐付けは未着手——本来は
//! 「注文が完了したレビュアーだけが投稿できる」という制約を持つべきだが、
//! 注文モデル自体がまだ無いため、この制約はまだ実装していない
//! (`CLAUDE.md`の「次にすべきこと」参照)。

use std::sync::Arc;

use aruaru_db_connector::AruaruDb;
use open_runo_poem_compat::hyper_compat::json_response;
use open_runo_poem_compat::{PathParams, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::json_body::read_json_body;

pub async fn ensure_table(db: &AruaruDb) -> anyhow::Result<()> {
    db.execute(
        "CREATE TABLE IF NOT EXISTS reviews (\
            id TEXT PRIMARY KEY, \
            service_id TEXT, \
            reviewer_name TEXT, \
            rating INTEGER, \
            comment TEXT\
        )",
        &[],
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct CreateReviewRequest {
    reviewer_name: String,
    rating: i32,
    comment: String,
    #[serde(default = "default_commit_message")]
    message: String,
}

fn default_commit_message() -> String {
    "new review".to_string()
}

#[derive(Debug, Serialize)]
struct CreateReviewResponse {
    id: String,
    commit_id: String,
}

fn validate(body: &CreateReviewRequest) -> Option<Response> {
    if body.reviewer_name.trim().is_empty() {
        return Some(json_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            &json!({ "error": "field \"reviewer_name\" must not be empty" }),
        ));
    }
    if !(1..=5).contains(&body.rating) {
        return Some(json_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            &json!({ "error": "rating must be between 1 and 5" }),
        ));
    }
    None
}

pub async fn create_review(req: Request, params: PathParams, db: Arc<AruaruDb>) -> Response {
    let Some(service_id) = params.get("id").map(str::to_string) else {
        return json_response(StatusCode::BAD_REQUEST, &json!({"error": "missing service id"}));
    };

    // レビュー対象の出品が実在するかを先に確認する(存在しないservice_id
    // へレビューを紐付けてしまう実用性の穴を作らない)。
    match db.query("SELECT id FROM services WHERE id = $1", &[&service_id]).await {
        Ok(rows) if rows.is_empty() => {
            return json_response(StatusCode::NOT_FOUND, &json!({"error": "service not found"}))
        }
        Err(e) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()})),
        Ok(_) => {}
    }

    let body = match read_json_body::<CreateReviewRequest>(req).await {
        Ok(b) => b,
        Err(resp) => return resp,
    };
    if let Some(resp) = validate(&body) {
        return resp;
    }

    let id = format!("{service_id}--review-{}", uuid_like());

    if let Err(e) = db
        .execute(
            "INSERT INTO reviews (id, service_id, reviewer_name, rating, comment) VALUES ($1, $2, $3, $4, $5)",
            &[&id, &service_id, &body.reviewer_name, &body.rating, &body.comment],
        )
        .await
    {
        return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()}));
    }

    let commit_id = match db.commit(&body.message).await {
        Ok(cid) => cid,
        Err(e) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()})),
    };

    json_response(StatusCode::OK, &CreateReviewResponse { id, commit_id })
}

pub async fn list_reviews(_req: Request, params: PathParams, db: Arc<AruaruDb>) -> Response {
    let Some(service_id) = params.get("id") else {
        return json_response(StatusCode::BAD_REQUEST, &json!({"error": "missing service id"}));
    };
    match db
        .query(
            "SELECT id, reviewer_name, rating, comment FROM reviews WHERE service_id = $1",
            &[&service_id],
        )
        .await
    {
        Ok(rows) => {
            let reviews: Vec<_> = rows
                .iter()
                .map(|r| {
                    json!({
                        "id": r.get::<_, String>(0),
                        "reviewer_name": r.get::<_, String>(1),
                        "rating": r.get::<_, i32>(2),
                        "comment": r.get::<_, String>(3),
                    })
                })
                .collect();
            json_response(StatusCode::OK, &json!({"reviews": reviews}))
        }
        Err(e) => json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()})),
    }
}

/// レビューid用の簡易な一意サフィックス。外部の`uuid`クレートに依存
/// せず、システム時刻(ナノ秒)を16進で使う(衝突耐性は本物のUUIDに劣るが、
/// 同一service_id内での連番的な用途には十分——本格的な対策が必要なら
/// 次段階で`uuid`クレート導入を検討)。
fn uuid_like() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("{nanos:x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_empty_reviewer_name() {
        let body = CreateReviewRequest {
            reviewer_name: "  ".to_string(),
            rating: 5,
            comment: "良かったです".to_string(),
            message: default_commit_message(),
        };
        assert!(validate(&body).is_some());
    }

    #[test]
    fn validate_rejects_out_of_range_rating() {
        for rating in [0, 6, -1] {
            let body = CreateReviewRequest {
                reviewer_name: "山田太郎".to_string(),
                rating,
                comment: "コメント".to_string(),
                message: default_commit_message(),
            };
            assert!(validate(&body).is_some(), "rating {rating} must be rejected");
        }
    }

    #[test]
    fn validate_accepts_a_well_formed_request() {
        let body = CreateReviewRequest {
            reviewer_name: "山田太郎".to_string(),
            rating: 5,
            comment: "良かったです".to_string(),
            message: default_commit_message(),
        };
        assert!(validate(&body).is_none());
    }
}
