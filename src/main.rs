//! aruaru.pro: コンコナラ型スキルマーケットプレイス+求人サイト統合。
//!
//! **現状(2026-09-13、リポジトリ新設)**: RPoem(`open-runo-poem-compat`)+
//! aruaru-db(`aruaru_db_connector`)の結線を`job-site`
//! (`F:\runo\repository\job-site`)から踏襲した最小の起動骨格のみ。
//! `GET /healthz`・`GET /categories`(カテゴリマスタ一覧)のみ実装。
//! 出品・注文・エスクロー・Stripe Connect決済・レビュー・チャット等は
//! 未着手(詳細は`CLAUDE.md`の「次にすべきこと」参照)。

mod auth;
mod career_agent_programs;
mod categories;
mod ids;
mod job_listings;
mod json_body;
mod oauth;
mod orders;
mod page;
mod reviews;
mod services;
mod stripe_connect;

use std::sync::Arc;

use aruaru_db_connector::AruaruDb;
use open_runo_poem_compat::hyper_compat::{html_response, json_response};
use open_runo_poem_compat::{get, handler_fn, post, Request, Response, Route, Server, StatusCode, TcpListener};
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let dsn = std::env::var("ARUARU_PRO_DSN")
        .unwrap_or_else(|_| "host=127.0.0.1 port=5433 user=app password=secret dbname=app".to_string());

    let db = Arc::new(
        AruaruDb::connect(&dsn)
            .await
            .map_err(|e| anyhow::anyhow!("aruaru-db connect failed: {e}"))?,
    );
    services::ensure_table(&db).await?;
    reviews::ensure_table(&db).await?;
    orders::ensure_table(&db).await?;
    stripe_connect::ensure_table(&db).await?;
    job_listings::ensure_table(&db).await?;
    auth::ensure_table(&db).await?;
    categories::ensure_table_and_seed(&db).await?;
    career_agent_programs::ensure_table(&db).await?;

    let db_auth_register = db.clone();
    let db_oauth_callback = db.clone();
    let db_categories_list = db.clone();
    let db_categories_page = db.clone();
    let db_programs_upsert = db.clone();
    let db_programs_list = db.clone();
    let db_programs_get = db.clone();
    let db_services_upsert = db.clone();
    let db_services_list = db.clone();
    let db_services_get = db.clone();
    let db_reviews_create = db.clone();
    let db_reviews_list = db.clone();
    let db_orders_create = db.clone();
    let db_orders_transition = db.clone();
    let db_stripe_onboarding = db.clone();
    let db_stripe_checkout = db.clone();
    let db_jobs_upsert = db.clone();
    let db_jobs_list = db.clone();
    let db_jobs_get = db.clone();

    let app = Route::new()
        .at(
            "/healthz",
            get(handler_fn(|_req, _p| Box::pin(async { json_response(StatusCode::OK, &json!({"ok": true})) }))),
        )
        .at(
            "/categories",
            get(handler_fn(move |_req: Request, _p| {
                let db = db_categories_list.clone();
                Box::pin(async move { list_categories(db).await })
            })),
        )
        .at(
            "/",
            get(handler_fn(move |_req: Request, _p| {
                let db = db_categories_page.clone();
                Box::pin(async move { render_categories_page(db).await })
            })),
        )
        .at(
            "/services",
            post(handler_fn(move |req, _p| {
                let db = db_services_upsert.clone();
                Box::pin(async move { services::upsert_service(req, db).await })
            }))
            .get(handler_fn(move |_req, _p| {
                let db = db_services_list.clone();
                Box::pin(async move { services::list_services(db).await })
            })),
        )
        .at(
            "/services/:id",
            get(handler_fn(move |req, params| {
                let db = db_services_get.clone();
                Box::pin(async move { services::get_service(req, params.into(), db).await })
            })),
        )
        .at(
            "/services/:id/reviews",
            post(handler_fn(move |req, params| {
                let db = db_reviews_create.clone();
                Box::pin(async move { reviews::create_review(req, params.into(), db).await })
            }))
            .get(handler_fn(move |req, params| {
                let db = db_reviews_list.clone();
                Box::pin(async move { reviews::list_reviews(req, params.into(), db).await })
            })),
        )
        .at(
            "/services/:id/orders",
            post(handler_fn(move |req, params| {
                let db = db_orders_create.clone();
                Box::pin(async move { orders::create_order(req, params.into(), db).await })
            })),
        )
        .at(
            "/orders/:id/transition",
            post(handler_fn(move |req, params| {
                let db = db_orders_transition.clone();
                Box::pin(async move { orders::transition_order(req, params.into(), db).await })
            })),
        )
        .at(
            "/sellers/:seller_name/stripe/onboarding",
            post(handler_fn(move |_req, params| {
                let db = db_stripe_onboarding.clone();
                Box::pin(async move { stripe_connect::create_onboarding_link(params.into(), db).await })
            })),
        )
        .at(
            "/orders/:order_id/checkout",
            post(handler_fn(move |_req, params| {
                let db = db_stripe_checkout.clone();
                Box::pin(async move { stripe_connect::create_checkout_session(params.into(), db).await })
            })),
        )
        .at(
            "/jobs",
            post(handler_fn(move |req, _p| {
                let db = db_jobs_upsert.clone();
                Box::pin(async move { job_listings::upsert_job_listing(req, db).await })
            }))
            .get(handler_fn(move |_req, _p| {
                let db = db_jobs_list.clone();
                Box::pin(async move { job_listings::list_job_listings(db).await })
            })),
        )
        .at(
            "/jobs/:id",
            get(handler_fn(move |req, params| {
                let db = db_jobs_get.clone();
                Box::pin(async move { job_listings::get_job_listing(req, params.into(), db).await })
            })),
        )
        .at(
            "/auth/register",
            post(handler_fn(move |req, _p| {
                let db = db_auth_register.clone();
                Box::pin(async move { auth::register(req, db).await })
            })),
        )
        .at(
            "/auth/oauth/:provider/start",
            get(handler_fn(move |req, params| Box::pin(async move { oauth::start(req, params.into()).await }))),
        )
        .at(
            "/auth/oauth/:provider/callback",
            get(handler_fn(move |req, params| {
                let db = db_oauth_callback.clone();
                Box::pin(async move { oauth::callback(req, params.into(), db).await })
            })),
        )
        .at(
            "/career-agent-programs",
            post(handler_fn(move |req, _p| {
                let db = db_programs_upsert.clone();
                Box::pin(async move { career_agent_programs::upsert_program(req, db).await })
            }))
            .get(handler_fn(move |_req, _p| {
                let db = db_programs_list.clone();
                Box::pin(async move { career_agent_programs::list_programs(db).await })
            })),
        )
        .at(
            "/career-agent-programs/:id",
            get(handler_fn(move |req, params| {
                let db = db_programs_get.clone();
                Box::pin(async move { career_agent_programs::get_program(req, params.into(), db).await })
            })),
        );

    let bind_addr: std::net::SocketAddr =
        std::env::var("ARUARU_PRO_BIND").unwrap_or_else(|_| "0.0.0.0:8081".to_string()).parse()?;
    tracing::info!("aruaru.pro listening on {bind_addr}");
    let (_addr, handle) = Server::new(TcpListener::bind(bind_addr)).run(app).await?;
    handle.await?;
    Ok(())
}

/// カテゴリ一覧(DB上の`categories`テーブルから読む、
/// `categories::load_category_tree`参照)。
async fn list_categories(db: Arc<AruaruDb>) -> Response {
    match categories::load_category_tree(&db).await {
        Ok(tree) => json_response(StatusCode::OK, &json!({ "categories": tree })),
        Err(e) => json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e})),
    }
}

/// RS-React(`App::mount`+`render_to_html`)でカテゴリ一覧をSSRする
/// 最小のUI(詳細は`page`モジュール参照)。
async fn render_categories_page(db: Arc<AruaruDb>) -> Response {
    html_response(StatusCode::OK, page::render_categories_page(&db).await)
}
