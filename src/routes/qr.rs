use axum::{
    extract::{Path, State},
    http::header,
    response::IntoResponse,
};
use image::Luma;
use qrcode::QrCode;

use crate::error::AppError;
use crate::state::AppState;

/// GET /api/v1/qr/:data — generate QR code as PNG (optimized for ESP32 OLED)
pub async fn qr_png(Path(data): Path<String>) -> Result<impl IntoResponse, AppError> {
    let code = QrCode::new(data.as_bytes())
        .map_err(|e| AppError::BadRequest(format!("invalid QR data: {}", e)))?;

    let image = code.render::<Luma<u8>>().quiet_zone(false).build();

    let mut buf = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("PNG encode error: {}", e)))?;

    Ok(([(header::CONTENT_TYPE, "image/png")], buf.into_inner()))
}

/// GET /lnurlp/:wallet_id/qr — QR code for LNURL-pay endpoint
pub async fn lnurl_qr(
    State(state): State<AppState>,
    Path(wallet_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    // Verify wallet exists
    crate::db::wallets::get_by_id(&state.db, &wallet_id)
        .await
        .map_err(|_| AppError::NotFound("wallet not found".to_string()))?;

    let lnurl_url = format!(
        "http://{}:{}/lnurlp/{}",
        state.config.host, state.config.port, wallet_id
    );

    // Encode as LNURL (bech32)
    let lnurl_encoded = encode_lnurl(&lnurl_url);

    let code = QrCode::new(lnurl_encoded.to_uppercase().as_bytes())
        .map_err(|e| AppError::Internal(anyhow::anyhow!("QR error: {}", e)))?;

    let image = code.render::<Luma<u8>>().quiet_zone(false).build();

    let mut buf = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("PNG encode error: {}", e)))?;

    Ok(([(header::CONTENT_TYPE, "image/png")], buf.into_inner()))
}

fn encode_lnurl(url: &str) -> String {
    use bech32::{Bech32, Hrp};
    let hrp = Hrp::parse("lnurl").expect("valid hrp");
    bech32::encode::<Bech32>(hrp, url.as_bytes()).unwrap_or_default()
}
