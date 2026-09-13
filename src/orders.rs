//! 注文(Order)。出品(Service)への注文と、その状態遷移
//! (`pending`→`completed`/`cancelled`)を管理する。レビュー投稿の前提
//! (「注文が完了した本人だけが投稿できる」)はこのモジュールが提供する
//! `order_is_completed_by`が担う(`reviews.rs`から呼ばれる)。

use std::sync::Arc;

use aruaru_db_connector::AruaruDb;
use open_runo_poem_compat::hyper_compat::json_response;
use open_runo_poem_compat::{PathParams, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::json_body::read_json_body;

pub async fn ensure_table(db: &AruaruDb) -> anyhow::Result<()> {
    db.execute(
        "CREATE TABLE IF NOT EXISTS orders (\
            id TEXT PRIMARY KEY, \
            service_id TEXT, \
            buyer_name TEXT, \
            status TEXT\
        )",
        &[],
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct CreateOrderRequest {
    buyer_name: String,
    #[serde(default = "default_commit_message")]
    message: String,
}

fn default_commit_message() -> String {
    "new order".to_string()
}

#[derive(Debug, Serialize)]
struct CreateOrderResponse {
    id: String,
    commit_id: String,
}

fn validate_create(body: &CreateOrderRequest) -> Option<Response> {
    if body.buyer_name.trim().is_empty() {
        return Some(json_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            &json!({ "error": "field \"buyer_name\" must not be empty" }),
        ));
    }
    None
}

/// 有効な状態遷移(pendingからのみcompleted/cancelledへ進める、
/// 完了/取消済みからの再遷移は許可しない——一度確定した注文の状態を
/// 後から書き換えられてしまう実用性の穴を作らない)。
const VALID_TRANSITIONS: &[(&str, &str)] = &[("pending", "completed"), ("pending", "cancelled")];

pub async fn create_order(req: Request, params: PathParams, db: Arc<AruaruDb>) -> Response {
    let user = match crate::auth::authenticate(&req, &db).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    if let Some(resp) = crate::auth::require_role(&user, "buyer") {
        return resp;
    }

    let Some(service_id) = params.get("id").map(str::to_string) else {
        return json_response(StatusCode::BAD_REQUEST, &json!({"error": "missing service id"}));
    };

    match db.query("SELECT id FROM services WHERE id = $1", &[&service_id]).await {
        Ok(rows) if rows.is_empty() => {
            return json_response(StatusCode::NOT_FOUND, &json!({"error": "service not found"}))
        }
        Err(e) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()})),
        Ok(_) => {}
    }

    let body = match read_json_body::<CreateOrderRequest>(req).await {
        Ok(b) => b,
        Err(resp) => return resp,
    };
    if let Some(resp) = validate_create(&body) {
        return resp;
    }
    if let Some(resp) = crate::auth::require_name_matches(&user, "buyer_name", &body.buyer_name) {
        return resp;
    }

    let id = crate::ids::make_id(&[&service_id, &body.buyer_name], "order");

    if let Err(e) = db
        .execute(
            "INSERT INTO orders (id, service_id, buyer_name, status) VALUES ($1, $2, $3, 'pending')",
            &[&id, &service_id, &body.buyer_name],
        )
        .await
    {
        return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()}));
    }

    let commit_id = match db.commit(&body.message).await {
        Ok(cid) => cid,
        Err(e) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()})),
    };

    json_response(StatusCode::OK, &CreateOrderResponse { id, commit_id })
}

#[derive(Debug, Deserialize)]
pub struct TransitionOrderRequest {
    status: String,
    #[serde(default = "default_transition_message")]
    message: String,
}

fn default_transition_message() -> String {
    "order status transition".to_string()
}

pub async fn transition_order(req: Request, params: PathParams, db: Arc<AruaruDb>) -> Response {
    let Some(order_id) = params.get("id") else {
        return json_response(StatusCode::BAD_REQUEST, &json!({"error": "missing order id"}));
    };
    let body = match read_json_body::<TransitionOrderRequest>(req).await {
        Ok(b) => b,
        Err(resp) => return resp,
    };

    let current_status = match db.query("SELECT status FROM orders WHERE id = $1", &[&order_id]).await {
        Ok(rows) => match rows.into_iter().next() {
            Some(r) => r.get::<_, String>(0),
            None => return json_response(StatusCode::NOT_FOUND, &json!({"error": "order not found"})),
        },
        Err(e) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()})),
    };

    if !VALID_TRANSITIONS.contains(&(current_status.as_str(), body.status.as_str())) {
        return json_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            &json!({ "error": format!("cannot transition from \"{current_status}\" to \"{}\"", body.status) }),
        );
    }

    if let Err(e) = db.execute("UPDATE orders SET status = $1 WHERE id = $2", &[&body.status, &order_id]).await {
        return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()}));
    }

    let commit_id = match db.commit(&body.message).await {
        Ok(cid) => cid,
        Err(e) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()})),
    };

    json_response(StatusCode::OK, &json!({ "id": order_id, "status": body.status, "commit_id": commit_id }))
}

/// このモジュールの主目的: `reviews.rs`が「レビュアーが、当該出品への
/// 完了済み注文の買い手本人か」を確認するために呼ぶ。完了済みの注文が
/// 1件も無ければ`false`(まだ`false`と`存在しない`を区別する必要は無い
/// ——呼び出し側は等しく「投稿不可」として扱う)。
pub async fn order_is_completed_by(db: &AruaruDb, service_id: &str, buyer_name: &str) -> Result<bool, String> {
    db.query(
        "SELECT 1 FROM orders WHERE service_id = $1 AND buyer_name = $2 AND status = 'completed' LIMIT 1",
        &[&service_id, &buyer_name],
    )
    .await
    .map(|rows| !rows.is_empty())
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_create_rejects_empty_buyer_name() {
        let body = CreateOrderRequest { buyer_name: "  ".to_string(), message: default_commit_message() };
        assert!(validate_create(&body).is_some());
    }

    #[test]
    fn validate_create_accepts_non_empty_buyer_name() {
        let body = CreateOrderRequest { buyer_name: "山田太郎".to_string(), message: default_commit_message() };
        assert!(validate_create(&body).is_none());
    }

    #[test]
    fn pending_to_completed_and_cancelled_are_the_only_valid_transitions() {
        assert!(VALID_TRANSITIONS.contains(&("pending", "completed")));
        assert!(VALID_TRANSITIONS.contains(&("pending", "cancelled")));
        assert!(!VALID_TRANSITIONS.contains(&("completed", "pending")));
        assert!(!VALID_TRANSITIONS.contains(&("cancelled", "completed")));
        assert!(!VALID_TRANSITIONS.contains(&("completed", "cancelled")));
    }
}
