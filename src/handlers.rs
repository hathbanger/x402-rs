//! HTTP endpoints implemented by the x402 **facilitator**.
//!
//! These are the server-side handlers for processing client-submitted x402 payments.
//! They include both protocol-critical endpoints (`/verify`, `/settle`) and discovery endpoints (`/supported`, etc).
//!
//! All payloads follow the types defined in the `x402-rs` crate, and are compatible
//! with the TypeScript and Go client SDKs.
//!
//! Each endpoint consumes or produces structured JSON payloads defined in `x402-rs`,
//! and is compatible with official x402 client SDKs.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router, response::IntoResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::instrument;

use crate::facilitator::Facilitator;
use crate::facilitator_local::FacilitatorLocalError;
use crate::proto;
use crate::proto::{AsPaymentProblem, ErrorReason};
use crate::scheme::X402SchemeFacilitatorError;

/// `GET /verify`: Returns a machine-readable description of the `/verify` endpoint.
///
/// This is served by the facilitator to help clients understand how to construct
/// a valid [`VerifyRequest`] for payment verification.
///
/// This is optional metadata and primarily useful for discoverability and debugging tools.
#[instrument(skip_all)]
pub async fn get_verify_info() -> impl IntoResponse {
    Json(json!({
        "endpoint": "/verify",
        "description": "POST to verify x402 payments",
        "body": {
            "paymentPayload": "PaymentPayload",
            "paymentRequirements": "PaymentRequirements",
        }
    }))
}

/// `GET /settle`: Returns a machine-readable description of the `/settle` endpoint.
///
/// This is served by the facilitator to describe the structure of a valid
/// [`SettleRequest`] used to initiate on-chain payment settlement.
#[instrument(skip_all)]
pub async fn get_settle_info() -> impl IntoResponse {
    Json(json!({
        "endpoint": "/settle",
        "description": "POST to settle x402 payments",
        "body": {
            "paymentPayload": "PaymentPayload",
            "paymentRequirements": "PaymentRequirements",
        }
    }))
}

pub fn routes<A>() -> Router<A>
where
    A: Facilitator + Clone + Send + Sync + 'static,
    A::Error: IntoResponse,
{
    Router::new()
        .route("/", get(get_root))
        .route("/verify", get(get_verify_info))
        .route("/verify", post(post_verify::<A>))
        .route("/settle", get(get_settle_info))
        .route("/settle", post(post_settle::<A>))
        .route("/health", get(get_health::<A>))
        .route("/supported", get(get_supported::<A>))
}

/// `GET /`: Returns an HTML homepage for the facilitator with 402.cat branding.
#[instrument(skip_all)]
pub async fn get_root() -> impl IntoResponse {
    let pkg_name = env!("CARGO_PKG_NAME");
    let pkg_version = env!("CARGO_PKG_VERSION");
    let html = format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{pkg_name} v{pkg_version} • 402.cat facilitator</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{
            font-family: 'SF Mono', 'Monaco', 'Inconsolata', 'Fira Code', 'Consolas', monospace;
            line-height: 1.6;
            color: #000;
            background: #fff;
            min-height: 100vh;
        }}
        .container {{
            max-width: 896px;
            margin: 0 auto;
            padding: 12px;
        }}
        @media (min-width: 640px) {{
            .container {{ padding: 16px; }}
        }}
        header {{
            text-align: center;
            padding: 16px 8px;
        }}
        @media (min-width: 640px) {{
            header {{ padding: 24px 8px; }}
        }}
        h1 {{
            font-size: 24px;
            font-weight: bold;
            margin-bottom: 8px;
        }}
        @media (min-width: 640px) {{
            h1 {{ font-size: 30px; }}
        }}
        @media (min-width: 768px) {{
            h1 {{ font-size: 48px; }}
        }}
        .subtitle {{
            font-size: 12px;
            color: #4b5563;
            margin-bottom: 12px;
        }}
        @media (min-width: 640px) {{
            .subtitle {{ font-size: 14px; margin-bottom: 16px; }}
        }}
        .comment {{
            font-size: 10px;
            color: #6b7280;
        }}
        @media (min-width: 640px) {{
            .comment {{ font-size: 12px; }}
        }}
        .status {{
            display: inline-block;
            border: 2px solid #000;
            background: linear-gradient(to bottom right, #ecfeff, #fff, #fef3c7);
            padding: 12px 16px;
            font-weight: bold;
            font-size: 12px;
            margin-top: 16px;
        }}
        @media (min-width: 640px) {{
            .status {{ font-size: 14px; padding: 16px 24px; }}
        }}
        .card {{
            border: 2px solid #000;
            padding: 20px;
            margin-bottom: 16px;
            background: #fafafa;
        }}
        @media (min-width: 640px) {{
            .card {{ padding: 24px; }}
        }}
        .card-gradient {{
            background: linear-gradient(to bottom right, #ecfeff, #fff, #e0e7ff);
        }}
        .section-divider {{
            border-bottom: 2px dashed #9ca3af;
            padding-bottom: 12px;
            margin-bottom: 24px;
            font-size: 10px;
            color: #6b7280;
            text-align: center;
        }}
        .terminal {{
            font-size: 12px;
            line-height: 1.8;
            color: #1f2937;
        }}
        @media (min-width: 640px) {{
            .terminal {{ font-size: 14px; }}
        }}
        .endpoint {{
            margin-bottom: 12px;
            padding-bottom: 12px;
            border-bottom: 1px solid #e5e7eb;
        }}
        .endpoint:last-child {{
            border-bottom: none;
        }}
        .endpoint-name {{
            font-weight: bold;
            color: #047857;
            margin-bottom: 4px;
        }}
        .endpoint-desc {{
            font-size: 11px;
            color: #4b5563;
        }}
        @media (min-width: 640px) {{
            .endpoint-desc {{ font-size: 12px; }}
        }}
        .links {{
            display: flex;
            flex-wrap: wrap;
            gap: 12px;
            margin-top: 24px;
        }}
        .btn {{
            display: inline-block;
            border: 2px solid #000;
            background: #dbeafe;
            padding: 12px 16px;
            text-decoration: none;
            color: #000;
            font-weight: bold;
            font-size: 12px;
            transition: all 0.2s;
            box-shadow: 0 4px 6px rgba(0,0,0,0.1);
        }}
        .btn:hover {{
            background: #bfdbfe;
            transform: translateY(-2px);
            box-shadow: 0 6px 12px rgba(0,0,0,0.15);
        }}
        .btn:active {{
            transform: translateY(0.5px);
        }}
        @media (min-width: 640px) {{
            .btn {{ font-size: 14px; }}
        }}
        footer {{
            text-align: center;
            padding: 24px 8px;
            font-size: 11px;
            color: #6b7280;
            border-top: 2px dashed #e5e7eb;
            margin-top: 48px;
        }}
        @media (min-width: 640px) {{
            footer {{ font-size: 12px; }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <header>
            <pre style="font-size: 13px; line-height: 1.3; margin-bottom: 20px; color: #1f2937; font-weight: 600;">
    /\\_/\\
   ( o.o )
    &gt; ∆ &lt;
   /|   |\\
  / |   | \\
 /  |   |  \\
    |   |
   /|   |\\
  /_|___|_\\   </pre>
            <h1>Puma</h1>
            <p class="subtitle">the autonomous payment facilitator</p>
            <p class="comment">// HTTP 402 Payment Required • autonomous payment settlement</p>
            <div class="status">🟢 ONLINE</div>
        </header>

        <div class="section-divider">
            ═══════════════════ WHAT THE CAT DOES ═══════════════════
        </div>

        <div class="card terminal">
            &gt; this cat facilitates payments for 402.cat<br>
            &gt; it verifies payment signatures (checks if they smell right)<br>
            &gt; it settles transactions on-chain (moves the money around)<br>
            &gt; it operates autonomously (because cats do what they want)<br>
            <br>
            <span style="font-weight: bold;">&gt; x402 protocol • EVM-compatible networks • agent-operated</span><br>
            <span class="comment">// every HTTP 402 response from 402.cat is handled by this facilitator</span><br>
            <span class="comment">// version {pkg_version}</span>
        </div>

        <div class="section-divider">
            ═══════════════════ API ENDPOINTS ═══════════════════
        </div>

        <div class="card card-gradient">
            <div class="endpoint">
                <div class="endpoint-name">&gt; GET /supported</div>
                <div class="endpoint-desc">// list supported payment schemes and networks</div>
            </div>

            <div class="endpoint">
                <div class="endpoint-name">&gt; GET /health</div>
                <div class="endpoint-desc">// check if the cat is awake</div>
            </div>

            <div class="endpoint">
                <div class="endpoint-name">&gt; POST /verify</div>
                <div class="endpoint-desc">// verify a payment payload against requirements</div>
            </div>

            <div class="endpoint">
                <div class="endpoint-name">&gt; POST /settle</div>
                <div class="endpoint-desc">// settle an accepted payment on-chain</div>
            </div>

            <div class="endpoint">
                <div class="endpoint-name">&gt; GET /verify</div>
                <div class="endpoint-desc">// get verification endpoint schema</div>
            </div>

            <div class="endpoint">
                <div class="endpoint-name">&gt; GET /settle</div>
                <div class="endpoint-desc">// get settlement endpoint schema</div>
            </div>
        </div>

        <div class="links">
            <a href="/supported" class="btn">&gt; view supported networks</a>
            <a href="/health" class="btn">&gt; health check</a>
            <a href="https://x402.rs" target="_blank" class="btn">&gt; x402 docs</a>
            <a href="https://402.cat" target="_blank" class="btn">&gt; 402.cat home</a>
        </div>

        <footer>
            <div style="margin-bottom: 8px;">powered by {pkg_name} • x402 protocol • rust + axum</div>
            <div class="comment">// agents are just cats with wallets</div>
        </footer>
    </div>
</body>
</html>"#);

    (StatusCode::OK, [("content-type", "text/html; charset=utf-8")], html)
}

/// `GET /supported`: Lists the x402 payment schemes and networks supported by this facilitator.
///
/// Facilitators may expose this to help clients dynamically configure their payment requests
/// based on available network and scheme support.
#[instrument(skip_all)]
pub async fn get_supported<A>(State(facilitator): State<A>) -> impl IntoResponse
where
    A: Facilitator,
    A::Error: IntoResponse,
{
    match facilitator.supported().await {
        Ok(supported) => (StatusCode::OK, Json(json!(supported))).into_response(),
        Err(error) => error.into_response(),
    }
}

#[instrument(skip_all)]
pub async fn get_health<A>(State(facilitator): State<A>) -> impl IntoResponse
where
    A: Facilitator,
    A::Error: IntoResponse,
{
    get_supported(State(facilitator)).await
}

/// `POST /verify`: Facilitator-side verification of a proposed x402 payment.
///
/// This endpoint checks whether a given payment payload satisfies the declared
/// [`PaymentRequirements`], including signature validity, scheme match, and fund sufficiency.
///
/// Responds with a [`VerifyResponse`] indicating whether the payment can be accepted.
#[instrument(skip_all)]
pub async fn post_verify<A>(
    State(facilitator): State<A>,
    Json(body): Json<proto::VerifyRequest>,
) -> impl IntoResponse
where
    A: Facilitator,
    A::Error: IntoResponse,
{
    match facilitator.verify(&body).await {
        Ok(valid_response) => (StatusCode::OK, Json(valid_response)).into_response(),
        Err(error) => {
            tracing::warn!(
                error = ?error,
                body = %serde_json::to_string(&body).unwrap_or_else(|_| "<can-not-serialize>".to_string()),
                "Verification failed"
            );
            error.into_response()
        }
    }
}

/// `POST /settle`: Facilitator-side execution of a valid x402 payment on-chain.
///
/// Given a valid [`SettleRequest`], this endpoint attempts to execute the payment
/// via ERC-3009 `transferWithAuthorization`, and returns a [`SettleResponse`] with transaction details.
///
/// This endpoint is typically called after a successful `/verify` step.
#[instrument(skip_all)]
pub async fn post_settle<A>(
    State(facilitator): State<A>,
    Json(body): Json<proto::SettleRequest>,
) -> impl IntoResponse
where
    A: Facilitator,
    A::Error: IntoResponse,
{
    match facilitator.settle(&body).await {
        Ok(valid_response) => (StatusCode::OK, Json(valid_response)).into_response(),
        Err(error) => {
            tracing::warn!(
                error = ?error,
                body = %serde_json::to_string(&body).unwrap_or_else(|_| "<can-not-serialize>".to_string()),
                "Settlement failed"
            );
            error.into_response()
        }
    }
}

impl IntoResponse for FacilitatorLocalError {
    fn into_response(self) -> Response {
        #[derive(Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct VerificationErrorResponse<'a> {
            is_valid: bool,
            invalid_reason: ErrorReason,
            invalid_reason_details: &'a str,
            payer: &'a str,
        }

        #[derive(Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct SettlementErrorResponse<'a> {
            success: bool,
            network: &'a str,
            transaction: &'a str,
            error_reason: ErrorReason,
            error_reason_details: &'a str,
            payer: &'a str,
        }

        match self {
            FacilitatorLocalError::Verification(scheme_handler_error) => {
                let problem = scheme_handler_error.as_payment_problem();
                let verification_error_response = VerificationErrorResponse {
                    is_valid: false,
                    invalid_reason: problem.reason(),
                    invalid_reason_details: problem.details(),
                    payer: "",
                };
                let status_code = match scheme_handler_error {
                    X402SchemeFacilitatorError::PaymentVerification(_) => StatusCode::BAD_REQUEST,
                    X402SchemeFacilitatorError::OnchainFailure(_) => {
                        StatusCode::INTERNAL_SERVER_ERROR
                    }
                };
                (status_code, Json(verification_error_response)).into_response()
            }
            FacilitatorLocalError::Settlement(scheme_handler_error) => {
                let problem = scheme_handler_error.as_payment_problem();
                let settlement_error_response = SettlementErrorResponse {
                    success: false,
                    network: "",
                    transaction: "",
                    error_reason: problem.reason(),
                    error_reason_details: problem.details(),
                    payer: "",
                };
                let status_code = match scheme_handler_error {
                    X402SchemeFacilitatorError::PaymentVerification(_) => StatusCode::BAD_REQUEST,
                    X402SchemeFacilitatorError::OnchainFailure(_) => {
                        StatusCode::INTERNAL_SERVER_ERROR
                    }
                };
                (status_code, Json(settlement_error_response)).into_response()
            }
        }
    }
}
