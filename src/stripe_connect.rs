//! Stripe Connectによる決済(出品者オンボーディング+プラットフォーム
//! 手数料付きのチェックアウト)。独自の資金移動システムは実装しない
//! (`CLAUDE.md`参照——法令対応の観点から既存決済プラットフォームに委ねる
//! 方針)。
//!
//! **現状のスコープ(第一段)**: Express Connectアカウントのオンボーディング
//! リンク発行、Checkout Session(`application_fee_amount`+
//! `transfer_data.destination`によるプラットフォーム手数料+出品者への
//! 自動振込)の作成のみ。Webhook(決済完了通知→注文の`completed`遷移)は
//! 未着手(`CLAUDE.md`の「次にすべきこと」参照)。
//!
//! **検証状況の正直な開示**: 実際のStripe API呼び出し(`Account::create`・
//! `AccountLink::create`・`CheckoutSession::create`)は、このセッションの
//! 環境にStripeのテストAPIキーが無いため実機検証していない。金額計算
//! (`platform_fee_yen`)とリクエストパラメータの構築ロジックのみユニット
//! テストで確認済み——ネットワーク呼び出し自体はコンパイルが通ることまでの
//! 保証。実際に使う前に、テストモードのAPIキーで最小限の実地確認が必要。

use std::sync::Arc;

use aruaru_db_connector::AruaruDb;
use open_runo_poem_compat::hyper_compat::json_response;
use open_runo_poem_compat::{PathParams, Response, StatusCode};
use serde_json::json;
use stripe::{
    Account, AccountLink, AccountLinkType, AccountType, CheckoutSession, CreateAccount, CreateAccountCapabilities,
    CreateAccountCapabilitiesCardPayments, CreateAccountCapabilitiesTransfers, CreateAccountLink,
    CreateCheckoutSession, CreateCheckoutSessionLineItems, CreateCheckoutSessionLineItemsPriceData,
    CreateCheckoutSessionLineItemsPriceDataProductData, CreateCheckoutSessionPaymentIntentData,
    CreateCheckoutSessionPaymentIntentDataTransferData, Currency,
};

/// プラットフォーム手数料率(コンコナラ等の既存サービスより大幅に安く
/// 提供する、というユーザー方針に基づく初期値。実際の値は事業判断で
/// 後から調整可能な単一定数として持つ)。
const PLATFORM_FEE_PERCENT: i64 = 5;

/// 出品者のStripe Connectアカウントidを保持するテーブル。認証機構が
/// 無い現段階では、`seller_name`をそのまま鍵として使う簡易な設計
/// (認証実装(「次にすべきこと」#4)の際にユーザーidへ差し替える想定)。
pub async fn ensure_table(db: &AruaruDb) -> anyhow::Result<()> {
    db.execute(
        "CREATE TABLE IF NOT EXISTS seller_stripe_accounts (\
            seller_name TEXT PRIMARY KEY, \
            stripe_account_id TEXT\
        )",
        &[],
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

fn stripe_client() -> Result<stripe::Client, Response> {
    match std::env::var("STRIPE_SECRET_KEY") {
        Ok(key) => Ok(stripe::Client::new(key)),
        Err(_) => Err(json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &json!({ "error": "STRIPE_SECRET_KEY is not configured" }),
        )),
    }
}

/// 出品者(`seller_name`)向けにExpress Connectアカウントを作成し
/// (既に作成済みならそれを再利用)、Connect Onboardingへのリンクを返す。
pub async fn create_onboarding_link(params: PathParams, db: Arc<AruaruDb>) -> Response {
    let Some(seller_name) = params.get("seller_name").map(str::to_string) else {
        return json_response(StatusCode::BAD_REQUEST, &json!({"error": "missing seller_name"}));
    };
    let client = match stripe_client() {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let existing = db
        .query("SELECT stripe_account_id FROM seller_stripe_accounts WHERE seller_name = $1", &[&seller_name])
        .await;
    let account_id = match existing {
        Ok(rows) if !rows.is_empty() => rows[0].get::<_, String>(0).parse().ok(),
        Ok(_) => None,
        Err(e) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()})),
    };

    let account_id = match account_id {
        Some(id) => id,
        None => {
            let mut create = CreateAccount::new();
            create.type_ = Some(AccountType::Express);
            create.country = Some("JP");
            create.capabilities = Some(CreateAccountCapabilities {
                card_payments: Some(CreateAccountCapabilitiesCardPayments { requested: Some(true) }),
                transfers: Some(CreateAccountCapabilitiesTransfers { requested: Some(true) }),
                ..Default::default()
            });
            let account = match Account::create(&client, create).await {
                Ok(a) => a,
                Err(e) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()})),
            };
            if let Err(e) = db
                .execute(
                    "INSERT INTO seller_stripe_accounts (seller_name, stripe_account_id) VALUES ($1, $2) \
                     ON CONFLICT (seller_name) DO UPDATE SET stripe_account_id = EXCLUDED.stripe_account_id",
                    &[&seller_name, &account.id.as_str()],
                )
                .await
            {
                return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()}));
            }
            if let Err(e) = db.commit("seller stripe onboarding started").await {
                return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()}));
            }
            account.id
        }
    };

    let base_url = std::env::var("ARUARU_PRO_PUBLIC_URL").unwrap_or_else(|_| "https://aruaru.pro".to_string());
    let refresh_url = format!("{base_url}/sellers/{seller_name}/stripe/onboarding");
    let return_url = format!("{base_url}/sellers/{seller_name}/stripe/onboarding/complete");
    let mut link_params = CreateAccountLink::new(account_id, AccountLinkType::AccountOnboarding);
    link_params.refresh_url = Some(&refresh_url);
    link_params.return_url = Some(&return_url);

    match AccountLink::create(&client, link_params).await {
        Ok(link) => json_response(StatusCode::OK, &json!({ "onboarding_url": link.url })),
        Err(e) => json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()})),
    }
}

/// `price_yen`(出品価格、円)からプラットフォーム手数料(円)を計算する。
/// 切り下げ(端数は出品者側に有利になるよう、プラットフォームが多く
/// 取らない方向へ丸める)。
pub fn platform_fee_yen(price_yen: i64) -> i64 {
    (price_yen * PLATFORM_FEE_PERCENT) / 100
}

/// 指定した出品(`service_id`)+注文(`order_id`)に対するCheckout Session
/// を作成し、決済ページのURLを返す。出品者がまだConnectオンボーディング
/// を完了していない場合はエラーを返す(`stripe_account_id`が無い)。
pub async fn create_checkout_session(params: PathParams, db: Arc<AruaruDb>) -> Response {
    let Some(order_id) = params.get("order_id").map(str::to_string) else {
        return json_response(StatusCode::BAD_REQUEST, &json!({"error": "missing order_id"}));
    };

    let order_row = match db.query("SELECT service_id FROM orders WHERE id = $1", &[&order_id]).await {
        Ok(rows) => match rows.into_iter().next() {
            Some(r) => r,
            None => return json_response(StatusCode::NOT_FOUND, &json!({"error": "order not found"})),
        },
        Err(e) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()})),
    };
    let service_id: String = order_row.get(0);

    let service_row = match db
        .query("SELECT seller_name, title, price_yen FROM services WHERE id = $1", &[&service_id])
        .await
    {
        Ok(rows) => match rows.into_iter().next() {
            Some(r) => r,
            None => return json_response(StatusCode::NOT_FOUND, &json!({"error": "service not found"})),
        },
        Err(e) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()})),
    };
    let seller_name: String = service_row.get(0);
    let title: String = service_row.get(1);
    let price_yen: i32 = service_row.get(2);

    let seller_account_id = match db
        .query("SELECT stripe_account_id FROM seller_stripe_accounts WHERE seller_name = $1", &[&seller_name])
        .await
    {
        Ok(rows) => match rows.into_iter().next() {
            Some(r) => r.get::<_, String>(0),
            None => {
                return json_response(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    &json!({ "error": "seller has not completed Stripe onboarding yet" }),
                )
            }
        },
        Err(e) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()})),
    };

    let client = match stripe_client() {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let base_url = std::env::var("ARUARU_PRO_PUBLIC_URL").unwrap_or_else(|_| "https://aruaru.pro".to_string());
    let success_url = format!("{base_url}/orders/{order_id}/checkout/success");
    let cancel_url = format!("{base_url}/orders/{order_id}/checkout/cancel");

    let mut session_params = CreateCheckoutSession::new();
    session_params.mode = Some(stripe::CheckoutSessionMode::Payment);
    session_params.success_url = Some(&success_url);
    session_params.cancel_url = Some(&cancel_url);
    session_params.line_items = Some(vec![CreateCheckoutSessionLineItems {
        quantity: Some(1),
        price_data: Some(CreateCheckoutSessionLineItemsPriceData {
            currency: Currency::JPY,
            unit_amount: Some(price_yen as i64),
            product_data: Some(CreateCheckoutSessionLineItemsPriceDataProductData {
                name: title,
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }]);
    session_params.payment_intent_data = Some(CreateCheckoutSessionPaymentIntentData {
        application_fee_amount: Some(platform_fee_yen(price_yen as i64)),
        transfer_data: Some(CreateCheckoutSessionPaymentIntentDataTransferData {
            destination: seller_account_id,
            ..Default::default()
        }),
        ..Default::default()
    });

    match CheckoutSession::create(&client, session_params).await {
        Ok(session) => json_response(StatusCode::OK, &json!({ "checkout_url": session.url })),
        Err(e) => json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()})),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_fee_is_five_percent_and_rounds_down() {
        assert_eq!(platform_fee_yen(10_000), 500);
        // 999 * 5 / 100 = 49.95 -> 49(切り下げ、出品者に有利な方向)
        assert_eq!(platform_fee_yen(999), 49);
    }

    #[test]
    fn platform_fee_is_far_below_typical_marketplace_fees() {
        // コンコナラ等の既存サービス(数十%規模)より大幅に安くする、
        // というユーザー方針を数値で裏取りする(20%を上回らないことを
        // 確認——具体的な他社料率はここでは断定しない)。
        let price = 10_000;
        assert!(platform_fee_yen(price) < price / 5);
    }
}
