//! aruaru.pro: コンコナラ型スキルマーケットプレイス+求人サイト統合。
//!
//! **現状(2026-09-13、リポジトリ新設)**: RPoem(`open-runo-poem-compat`)+
//! aruaru-db(`aruaru_db_connector`)の結線を`job-site`
//! (`F:\runo\repository\job-site`)から踏襲した最小の起動骨格のみ。
//! `GET /healthz`・`GET /categories`(カテゴリマスタ一覧)のみ実装。
//! 出品・注文・エスクロー・Stripe Connect決済・レビュー・チャット等は
//! 未着手(詳細は`CLAUDE.md`の「次にすべきこと」参照)。

mod categories;

use std::sync::Arc;

use aruaru_db_connector::AruaruDb;
use open_runo_poem_compat::hyper_compat::json_response;
use open_runo_poem_compat::{get, handler_fn, Request, Response, Route, Server, StatusCode, TcpListener};
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

    let app = Route::new()
        .at(
            "/healthz",
            get(handler_fn(|_req, _p| Box::pin(async { json_response(StatusCode::OK, &json!({"ok": true})) }))),
        )
        .at(
            "/categories",
            get(handler_fn(|_req: Request, _p| Box::pin(async { list_categories().await }))),
        );

    let bind_addr: std::net::SocketAddr =
        std::env::var("ARUARU_PRO_BIND").unwrap_or_else(|_| "0.0.0.0:8081".to_string()).parse()?;
    tracing::info!("aruaru.pro listening on {bind_addr}");
    let (_addr, handle) = Server::new(TcpListener::bind(bind_addr)).run(app).await?;
    handle.await?;

    // db は将来のエンドポイント(出品/注文/レビュー)実装まで保持する
    // (現状は接続確認のみで未使用、警告回避のため明示的にdrop)。
    drop(db);
    Ok(())
}

async fn list_categories() -> Response {
    json_response(StatusCode::OK, &json!({ "categories": categories::CATEGORIES }))
}
