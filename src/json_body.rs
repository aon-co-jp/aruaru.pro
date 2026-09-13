//! リクエストボディをRS-JSON(`rust-json`)経由でデコードするヘルパー。
//! `job-site`の`read_json_body`と同じパターン(aruaru-db本体が
//! `serde_json`直呼びから`rust-json`へ移行した既存方針に揃える)。

use http_body_util::BodyExt;
use open_runo_poem_compat::hyper_compat::json_response;
use open_runo_poem_compat::{Request, Response, StatusCode};
use serde_json::json;

pub async fn read_json_body<T: serde::de::DeserializeOwned>(req: Request) -> Result<T, Response> {
    let bytes = match req.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => {
            return Err(json_response(StatusCode::BAD_REQUEST, &json!({ "error": "failed to read request body" })))
        }
    };
    rust_json::from_slice_strict::<T>(&bytes)
        .map_err(|e| json_response(StatusCode::BAD_REQUEST, &json!({ "error": format!("invalid JSON body: {e}") })))
}
