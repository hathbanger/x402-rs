//! HTTP endpoints implemented by the x402 **facilitator**.

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

#[instrument(skip_all)]
pub async fn get_verify_info() -> impl IntoResponse {
    Json(json!({"endpoint": "/verify", "description": "POST to verify x402 payments"}))
}

#[instrument(skip_all)]
pub async fn get_settle_info() -> impl IntoResponse {
    Json(json!({"endpoint": "/settle", "description": "POST to settle x402 payments"}))
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

/// `GET /`: Returns an HTML homepage
#[instrument(skip_all)]
pub async fn get_root() -> impl IntoResponse {
    let pkg_name = env!("CARGO_PKG_NAME");
    let pkg_version = env!("CARGO_PKG_VERSION");
    
    // Read the HTML from external file for easier editing
    let html_content = include_str!("homepage.html");
    let html = html_content.replace("{pkg_name}", pkg_name).replace("{pkg_version}", pkg_version);

    (StatusCode::OK, [("content-type", "text/html; charset=utf-8")], html)
}

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
            tracing::warn!(error = ?error, "Verification failed");
            error.into_response()
        }
    }
}

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
            tracing::warn!(error = ?error, "Settlement failed");
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
                    X402SchemeFacilitatorError::OnchainFailure(_) => StatusCode::INTERNAL_SERVER_ERROR,
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
                    X402SchemeFacilitatorError::OnchainFailure(_) => StatusCode::INTERNAL_SERVER_ERROR,
                };
                (status_code, Json(settlement_error_response)).into_response()
            }
        }
    }
}
