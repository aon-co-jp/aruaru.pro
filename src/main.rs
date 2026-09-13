//! aruaru.pro: コンコナラ型スキルマーケットプレイス+求人サイト統合。
//!
//! **現状(2026-09-13、リポジトリ新設)**: RPoem(`open-runo-poem-compat`)+
//! aruaru-db(`aruaru_db_connector`)の結線を`job-site`
//! (`F:\runo\repository\job-site`)から踏襲した最小の起動骨格のみ。
//! `GET /healthz`・`GET /categories`(カテゴリマスタ一覧)のみ実装。
//! 出品・注文・エスクロー・Stripe Connect決済・レビュー・チャット等は
//! 未着手(詳細は`CLAUDE.md`の「次にすべきこと」参照)。

mod categories;
mod ids;
mod json_body;
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

    let db_services_upsert = db.clone();
    let db_services_list = db.clone();
    let db_services_get = db.clone();
    let db_reviews_create = db.clone();
    let db_reviews_list = db.clone();
    let db_orders_create = db.clone();
    let db_orders_transition = db.clone();
    let db_stripe_onboarding = db.clone();
    let db_stripe_checkout = db.clone();

    let app = Route::new()
        .at(
            "/healthz",
            get(handler_fn(|_req, _p| Box::pin(async { json_response(StatusCode::OK, &json!({"ok": true})) }))),
        )
        .at(
            "/categories",
            get(handler_fn(|_req: Request, _p| Box::pin(async { list_categories().await }))),
        )
        .at(
            "/",
            get(handler_fn(|_req: Request, _p| Box::pin(async { render_categories_page().await }))),
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
        );

    let bind_addr: std::net::SocketAddr =
        std::env::var("ARUARU_PRO_BIND").unwrap_or_else(|_| "0.0.0.0:8081".to_string()).parse()?;
    tracing::info!("aruaru.pro listening on {bind_addr}");
    let (_addr, handle) = Server::new(TcpListener::bind(bind_addr)).run(app).await?;
    handle.await?;
    Ok(())
}

async fn list_categories() -> Response {
    json_response(StatusCode::OK, &json!({ "categories": categories::CATEGORIES }))
}

/// RS-React(`App::mount`+`render_to_html`)でカテゴリ一覧をSSRする
/// 最小のUI(詳細は`page`モジュール参照)。
async fn render_categories_page() -> Response {
    html_response(StatusCode::OK, page::render_categories_page())
}
