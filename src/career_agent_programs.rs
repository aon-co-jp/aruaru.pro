//! IT研修付き転職エージェントプログラム(CareerAgentProgram)。
//!
//! ユーザー指示(2026-09-13)「正社員求人や無料のIT研修付き無料の
//! 転職エージェントサービスコーナーも作って」に基づき新設。
//!
//! **`job_listings`との違い**: `job_listings`は「勤務先が出す求人票」
//! (勤務地・時給・雇用形態)であり、正社員求人自体は
//! `employment_type = "正社員"`で既にカバーしている。一方、本モジュール
//! が扱うのは「転職エージェント(採用支援企業)が提供する、無料IT研修
//! +就職支援」という異なるサービス形態——出品者は勤務先ではなく
//! エージェント企業、利用者は求人に応募するのではなく研修プログラムに
//! 申し込む。フィールドが根本的に異なるため専用モデルとする
//! (`services`・`job_listings`と同じ設計判断)。
//!
//! **料金体系についての正直な開示**: 「無料のIT研修付き無料の転職
//! エージェント」という表現から、利用者(研修を受ける側)への課金は
//! 想定していない(`is_free_for_trainee`は現状常に`true`として運用する
//! 想定、フィールドとして持たせているのは将来有料コースが追加される
//! 可能性への備え)。エージェント企業からの成約報酬等の収益モデルは
//! このプロジェクトのスコープ外(`CLAUDE.md`の「次にすべきこと」参照)。

use std::sync::Arc;

use aruaru_db_connector::AruaruDb;
use open_runo_poem_compat::hyper_compat::json_response;
use open_runo_poem_compat::{PathParams, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::json_body::read_json_body;

pub async fn ensure_table(db: &AruaruDb) -> anyhow::Result<()> {
    db.execute(
        "CREATE TABLE IF NOT EXISTS career_agent_programs (\
            id TEXT PRIMARY KEY, \
            agent_name TEXT, \
            program_name TEXT, \
            description TEXT, \
            training_weeks INTEGER, \
            target_job_type TEXT, \
            is_free_for_trainee BOOLEAN\
        )",
        &[],
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct UpsertProgramRequest {
    agent_name: String,
    program_name: String,
    description: String,
    training_weeks: i32,
    target_job_type: String,
    #[serde(default = "default_is_free")]
    is_free_for_trainee: bool,
    #[serde(default = "default_commit_message")]
    message: String,
}

fn default_is_free() -> bool {
    true
}

fn default_commit_message() -> String {
    "career agent program update".to_string()
}

#[derive(Debug, Serialize)]
struct UpsertProgramResponse {
    id: String,
    commit_id: String,
}

fn validate_upsert(body: &UpsertProgramRequest) -> Option<Response> {
    let bad_field = if body.agent_name.trim().is_empty() {
        Some("agent_name")
    } else if body.program_name.trim().is_empty() {
        Some("program_name")
    } else if body.description.trim().is_empty() {
        Some("description")
    } else if body.target_job_type.trim().is_empty() {
        Some("target_job_type")
    } else {
        None
    };
    if let Some(field) = bad_field {
        return Some(json_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            &json!({ "error": format!("field \"{field}\" must not be empty") }),
        ));
    }
    if body.training_weeks <= 0 {
        return Some(json_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            &json!({ "error": "training_weeks must be a positive integer" }),
        ));
    }
    None
}

pub async fn upsert_program(req: Request, db: Arc<AruaruDb>) -> Response {
    let user = match crate::auth::authenticate(&req, &db).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    // エージェント企業も「求人を仲介する」という点で採用担当
    // (`recruiter`)ロールの一種として扱う(専用ロールを新設すると
    // authロール一覧が肥大化するため、既存の役割分担に収める判断)。
    if let Some(resp) = crate::auth::require_role(&user, "recruiter") {
        return resp;
    }

    let body = match read_json_body::<UpsertProgramRequest>(req).await {
        Ok(b) => b,
        Err(resp) => return resp,
    };
    if let Some(resp) = validate_upsert(&body) {
        return resp;
    }
    if let Some(resp) = crate::auth::require_name_matches(&user, "agent_name", &body.agent_name) {
        return resp;
    }

    let id = crate::ids::make_id(&[&body.agent_name, &body.program_name], "program");

    if let Err(e) = db
        .execute(
            "INSERT INTO career_agent_programs \
             (id, agent_name, program_name, description, training_weeks, target_job_type, is_free_for_trainee) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) \
             ON CONFLICT (id) DO UPDATE SET agent_name = EXCLUDED.agent_name, \
             program_name = EXCLUDED.program_name, description = EXCLUDED.description, \
             training_weeks = EXCLUDED.training_weeks, target_job_type = EXCLUDED.target_job_type, \
             is_free_for_trainee = EXCLUDED.is_free_for_trainee",
            &[
                &id,
                &body.agent_name,
                &body.program_name,
                &body.description,
                &body.training_weeks,
                &body.target_job_type,
                &body.is_free_for_trainee,
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

    json_response(StatusCode::OK, &UpsertProgramResponse { id, commit_id })
}

pub async fn list_programs(db: Arc<AruaruDb>) -> Response {
    match db
        .query(
            "SELECT id, agent_name, program_name, description, training_weeks, target_job_type, \
             is_free_for_trainee FROM career_agent_programs",
            &[],
        )
        .await
    {
        Ok(rows) => {
            let programs: Vec<_> = rows.iter().map(row_to_json).collect();
            json_response(StatusCode::OK, &json!({"career_agent_programs": programs}))
        }
        Err(e) => json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()})),
    }
}

pub async fn get_program(_req: Request, params: PathParams, db: Arc<AruaruDb>) -> Response {
    let Some(id) = params.get("id") else {
        return json_response(StatusCode::BAD_REQUEST, &json!({"error": "missing id"}));
    };
    match db
        .query(
            "SELECT id, agent_name, program_name, description, training_weeks, target_job_type, \
             is_free_for_trainee FROM career_agent_programs WHERE id = $1",
            &[&id],
        )
        .await
    {
        Ok(rows) => match rows.into_iter().next() {
            Some(r) => json_response(StatusCode::OK, &row_to_json(&r)),
            None => json_response(StatusCode::NOT_FOUND, &json!({"error": "program not found"})),
        },
        Err(e) => json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()})),
    }
}

fn row_to_json(r: &tokio_postgres::Row) -> serde_json::Value {
    json!({
        "id": r.get::<_, String>(0),
        "agent_name": r.get::<_, String>(1),
        "program_name": r.get::<_, String>(2),
        "description": r.get::<_, String>(3),
        "training_weeks": r.get::<_, i32>(4),
        "target_job_type": r.get::<_, String>(5),
        "is_free_for_trainee": r.get::<_, bool>(6),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn well_formed_request() -> UpsertProgramRequest {
        UpsertProgramRequest {
            agent_name: "株式会社キャリアエージェント".to_string(),
            program_name: "未経験からのITエンジニア転職コース".to_string(),
            description: "12週間の実務研修後に提携企業へ紹介".to_string(),
            training_weeks: 12,
            target_job_type: "Webエンジニア".to_string(),
            is_free_for_trainee: true,
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
        body.program_name = "  ".to_string();
        assert!(validate_upsert(&body).is_some());
    }

    #[test]
    fn validate_upsert_rejects_non_positive_training_weeks() {
        let mut body = well_formed_request();
        body.training_weeks = 0;
        assert!(validate_upsert(&body).is_some());
    }

    #[test]
    fn is_free_for_trainee_defaults_to_true() {
        assert!(default_is_free());
    }
}
