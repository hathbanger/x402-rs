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
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}

        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
            background: #fafafa;
            color: #1a1a1a;
            line-height: 1.6;
            padding: 20px;
        }}

        .container {{
            max-width: 1000px;
            margin: 0 auto;
        }}

        .header {{
            text-align: center;
            padding: 40px 0 60px 0;
        }}

        .cat-ascii {{
            font-family: 'Courier New', monospace;
            font-size: 16px;
            line-height: 1.2;
            color: #1a1a1a;
            margin: 20px 0;
        }}

        .title {{
            font-size: 36px;
            font-weight: 700;
            margin: 20px 0 10px 0;
        }}

        .subtitle {{
            font-size: 14px;
            color: #666;
            margin-bottom: 8px;
        }}

        .meta {{
            font-size: 12px;
            color: #999;
            font-family: 'Courier New', monospace;
        }}

        .status-badge {{
            display: inline-flex;
            align-items: center;
            gap: 8px;
            padding: 8px 16px;
            background: #e8f5e9;
            border: 1px solid #9ee2b0;
            border-radius: 8px;
            font-size: 14px;
            font-weight: 600;
            color: #1e7e34;
            margin-top: 20px;
        }}

        .section-title {{
            font-size: 24px;
            font-weight: 700;
            margin: 40px 0 20px 0;
        }}

        .description-card {{
            background: white;
            border: 1px solid #e5e5e5;
            border-radius: 8px;
            padding: 24px;
            margin-bottom: 30px;
        }}

        .description-card p {{
            font-size: 14px;
            color: #666;
            margin-bottom: 12px;
            line-height: 1.8;
        }}

        .description-card p:last-child {{
            margin-bottom: 0;
        }}

        .highlight {{
            font-weight: 600;
            color: #1a1a1a;
        }}

        .category-section {{
            margin-bottom: 40px;
        }}

        .category-header {{
            display: flex;
            align-items: center;
            gap: 12px;
            margin-bottom: 16px;
            padding-bottom: 12px;
            border-bottom: 2px solid #f0f0f0;
        }}

        .category-name {{
            font-size: 18px;
            font-weight: 600;
            color: #1a1a1a;
        }}

        .category-count {{
            font-size: 12px;
            color: #999;
            background: #f5f5f5;
            padding: 4px 10px;
            border-radius: 12px;
        }}

        .endpoint-list {{
            display: flex;
            flex-direction: column;
            gap: 12px;
        }}

        .endpoint-item {{
            background: white;
            border: 1px solid #e5e5e5;
            border-radius: 8px;
            padding: 16px 20px;
            transition: all 0.2s;
        }}

        .endpoint-item:hover {{
            border-color: #d0d0d0;
            box-shadow: 0 2px 8px rgba(0, 0, 0, 0.04);
        }}

        .endpoint-header {{
            display: flex;
            align-items: center;
            gap: 12px;
            margin-bottom: 8px;
        }}

        .method-badge {{
            padding: 4px 10px;
            border-radius: 4px;
            font-size: 11px;
            font-weight: 600;
            font-family: 'Courier New', monospace;
            min-width: 45px;
            text-align: center;
        }}

        .method-get {{
            background: #d4f4dd;
            color: #1e7e34;
            border: 1px solid #9ee2b0;
        }}

        .method-post {{
            background: #d6e9ff;
            color: #0052cc;
            border: 1px solid #a3cfff;
        }}

        .endpoint-path {{
            font-family: 'Courier New', monospace;
            font-size: 14px;
            font-weight: 500;
            color: #1a1a1a;
            flex: 1;
        }}

        .endpoint-desc {{
            font-size: 13px;
            color: #666;
            padding-left: 57px;
        }}

        .action-buttons {{
            display: flex;
            gap: 12px;
            flex-wrap: wrap;
            margin-top: 40px;
        }}

        .btn {{
            padding: 12px 24px;
            background: #1a1a1a;
            color: white;
            border: none;
            border-radius: 8px;
            font-size: 14px;
            font-weight: 600;
            text-decoration: none;
            cursor: pointer;
            transition: all 0.2s;
            display: inline-flex;
            align-items: center;
            gap: 8px;
        }}

        .btn:hover {{
            background: #333;
            transform: translateY(-1px);
            box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
        }}

        .btn:active {{
            transform: translateY(0);
        }}

        .btn-secondary {{
            background: white;
            color: #1a1a1a;
            border: 1px solid #e0e0e0;
        }}

        .btn-secondary:hover {{
            background: #fafafa;
            border-color: #1a1a1a;
        }}

        footer {{
            text-align: center;
            padding: 40px 20px;
            margin-top: 60px;
            border-top: 1px solid #e5e5e5;
        }}

        .footer-text {{
            font-size: 12px;
            color: #999;
            margin-bottom: 8px;
        }}

        .footer-tagline {{
            font-size: 11px;
            color: #ccc;
            font-family: 'Courier New', monospace;
        }}

        @media (max-width: 768px) {{
            .title {{
                font-size: 28px;
            }}

            .endpoint-desc {{
                padding-left: 0;
                margin-top: 4px;
            }}

            .action-buttons {{
                flex-direction: column;
            }}

            .btn {{
                width: 100%;
                justify-content: center;
            }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <pre class="cat-ascii">  /\\_/\\
 ( o.o )
  &gt; ^ &lt;</pre>
            <h1 class="title" id="site-title">facilitator.402.cat</h1>
            <p class="subtitle">autonomous payment facilitator</p>
            <p class="meta">// x402 protocol • base / base-sepolia • payment settlement</p>
            <div class="status-badge">
                <span>🟢</span>
                <span>ONLINE</span>
            </div>
        </div>

        <div class="description-card">
            <p>This facilitator handles <span class="highlight">x402 protocol payments</span> for the 402.cat ecosystem.</p>
            <p>It <span class="highlight">verifies payment signatures</span>, ensures they meet requirements, and <span class="highlight">settles transactions on-chain</span> using EVM-compatible networks.</p>
            <p class="meta">version {pkg_version} • rust + axum • agent-operated</p>
        </div>

        <h2 class="section-title">Endpoints</h2>

        <div class="category-section">
            <div class="category-header">
                <div class="category-name">Discovery & Health</div>
                <div class="category-count">2 endpoints</div>
            </div>
            <div class="endpoint-list">
                <div class="endpoint-item">
                    <div class="endpoint-header">
                        <span class="method-badge method-get">GET</span>
                        <span class="endpoint-path">/supported</span>
                    </div>
                    <div class="endpoint-desc">List supported payment schemes and networks</div>
                </div>
                <div class="endpoint-item">
                    <div class="endpoint-header">
                        <span class="method-badge method-get">GET</span>
                        <span class="endpoint-path">/health</span>
                    </div>
                    <div class="endpoint-desc">Health check endpoint</div>
                </div>
            </div>
        </div>

        <div class="category-section">
            <div class="category-header">
                <div class="category-name">Payment Processing</div>
                <div class="category-count">2 endpoints</div>
            </div>
            <div class="endpoint-list">
                <div class="endpoint-item">
                    <div class="endpoint-header">
                        <span class="method-badge method-post">POST</span>
                        <span class="endpoint-path">/verify</span>
                    </div>
                    <div class="endpoint-desc">Verify a payment payload against requirements</div>
                </div>
                <div class="endpoint-item">
                    <div class="endpoint-header">
                        <span class="method-badge method-post">POST</span>
                        <span class="endpoint-path">/settle</span>
                    </div>
                    <div class="endpoint-desc">Settle an accepted payment on-chain</div>
                </div>
            </div>
        </div>

        <div class="category-section">
            <div class="category-header">
                <div class="category-name">Schema & Documentation</div>
                <div class="category-count">2 endpoints</div>
            </div>
            <div class="endpoint-list">
                <div class="endpoint-item">
                    <div class="endpoint-header">
                        <span class="method-badge method-get">GET</span>
                        <span class="endpoint-path">/verify</span>
                    </div>
                    <div class="endpoint-desc">Get verification endpoint schema</div>
                </div>
                <div class="endpoint-item">
                    <div class="endpoint-header">
                        <span class="method-badge method-get">GET</span>
                        <span class="endpoint-path">/settle</span>
                    </div>
                    <div class="endpoint-desc">Get settlement endpoint schema</div>
                </div>
            </div>
        </div>

        <div class="action-buttons">
            <a href="/supported" class="btn">View Supported Networks</a>
            <a href="/health" class="btn btn-secondary">Health Check</a>
            <a href="https://x402.rs" target="_blank" class="btn btn-secondary">x402 Docs</a>
            <a href="https://402.cat" target="_blank" class="btn btn-secondary">402.cat Home</a>
        </div>

        <footer>
            <div class="footer-text">powered by {pkg_name} • x402 protocol</div>
            <div class="footer-tagline">// agents are just cats with wallets</div>
        </footer>
    </div>
    <script>
        // Set dynamic title based on hostname
        document.getElementById('site-title').textContent = window.location.hostname;
        document.title = window.location.hostname + ' • x402 facilitator';
    </script>
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
