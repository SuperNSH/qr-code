//! The scanner as TSX sees it: `NativeQrCode`, a class whose async members reject with a
//! `code` on failure. Results are plain values; frames and the camera stay on the Rust side.

use std::time::Duration;

use nsh_core::prelude::*;

use crate::{
    DecodedQr, EccLevel, ErrorKind, Frame, GenerateOptions, PermissionState, QrError, QrScanner,
    ScanOptions, decode_frame, generate,
};

/// The options of one generation, from TSX. Unset fields keep the defaults.
#[derive(JsObject, Clone, Debug, Default)]
pub struct NativeGenerateOptions {
    /// `"low"`, `"medium"`, `"quartile"` or `"high"`. Unset picks by payload length.
    pub ec_level: Option<String>,
    /// The white border around the code, in modules. Defaults to 4.
    pub quiet_zone: Option<u32>,
}

/// The options of one scan, from TSX. Unset fields keep the defaults.
#[derive(JsObject, Clone, Debug, Default)]
pub struct NativeScanOptions {
    /// The longest the scan waits for a code, in milliseconds. Off unless set; unused on the web.
    pub timeout_ms: Option<u64>,
}

/// One QR code TSX reads: its text and bytes, the code's own facts, and its corners.
#[derive(JsObject, Clone, Debug, PartialEq)]
pub struct NativeDecodedQr {
    /// The payload as text, with invalid UTF-8 replaced.
    pub text: String,
    /// The payload as raw bytes.
    pub bytes: Vec<u8>,
    /// `"low"`, `"medium"`, `"quartile"` or `"high"`.
    pub ec_level: String,
    /// The QR version, 1 to 40.
    pub version: u32,
    /// The four corners' x,y coordinates in frame pixels: top-left, top-right, bottom-right,
    /// bottom-left.
    pub corners: Vec<u32>,
}

/// A QR code handle. Create one and keep it; every handle shares the platform's scan gate.
#[class]
pub struct NativeQrCode {
    scanner: QrScanner,
}

#[class]
impl NativeQrCode {
    /// The platform's scanner.
    pub fn new() -> NativeQrCode {
        NativeQrCode {
            scanner: QrScanner::new(),
        }
    }

    /// The QR codes in one grayscale frame: `luma` holds one byte per pixel, row-major,
    /// 0 black and 255 white.
    pub async fn decode_image(
        this: Instance<Self>,
        width: u32,
        height: u32,
        luma: Vec<u8>,
    ) -> Result<Vec<NativeDecodedQr>, JsError> {
        // An instance member like its siblings; the frame carries everything it needs.
        let _ = this;
        let frame = Frame::new(width as usize, height as usize, luma).map_err(into_js)?;
        let found = decode_frame(&frame).map_err(into_js)?;
        Ok(found.into_iter().map(native_qr).collect())
    }

    /// The QR code for `data`, as an SVG string.
    pub async fn generate(
        this: Instance<Self>,
        data: Vec<u8>,
        options: Option<NativeGenerateOptions>,
    ) -> Result<String, JsError> {
        // An instance member like its siblings; the payload carries everything it needs.
        let _ = this;
        let options = to_generate_options(options.unwrap_or_default())?;
        let matrix = generate(&data, &options).map_err(into_js)?;
        Ok(matrix.to_svg())
    }

    /// The camera permission as it stands: `"undetermined"`, `"granted"` or `"denied"`.
    pub async fn camera_permission(this: Instance<Self>) -> Result<String, JsError> {
        let scanner = this.with(|native| native.scanner.clone());
        let state = scanner.camera_permission().await.map_err(into_js)?;
        Ok(permission_name(state).to_owned())
    }

    /// Asks for the camera permission and reports how it ended.
    pub async fn request_camera_permission(this: Instance<Self>) -> Result<String, JsError> {
        let scanner = this.with(|native| native.scanner.clone());
        let state = scanner.request_camera_permission().await.map_err(into_js)?;
        Ok(permission_name(state).to_owned())
    }

    /// Scans camera frames until one carries a QR code. One scan runs at a time.
    pub async fn scan(
        this: Instance<Self>,
        options: Option<NativeScanOptions>,
    ) -> Result<NativeDecodedQr, JsError> {
        let scanner = this.with(|native| native.scanner.clone());
        let options = to_scan_options(options.unwrap_or_default());
        let decoded = scanner.scan(&options).await.map_err(into_js)?;
        Ok(native_qr(decoded))
    }

    /// Stops the running scan; it resolves with `ERR_QR_CANCELLED`. Returns `true` when one ran.
    pub async fn stop(this: Instance<Self>) -> Result<bool, JsError> {
        let scanner = this.with(|native| native.scanner.clone());
        scanner.stop().await.map_err(into_js)
    }
}

impl Default for NativeQrCode {
    fn default() -> Self {
        Self::new()
    }
}

/// The error a rejection carries: its message, and the `code` apps match on.
fn into_js(error: QrError) -> JsError {
    JsError::Error(error.kind().message().to_owned()).with_code(error.code())
}

fn invalid(message: &str) -> JsError {
    JsError::Error(message.to_owned()).with_code(ErrorKind::InvalidArgument.code())
}

fn permission_name(state: PermissionState) -> &'static str {
    match state {
        PermissionState::Undetermined => "undetermined",
        PermissionState::Granted => "granted",
        PermissionState::Denied => "denied",
    }
}

fn ec_level_name(level: EccLevel) -> &'static str {
    match level {
        EccLevel::Low => "low",
        EccLevel::Medium => "medium",
        EccLevel::Quartile => "quartile",
        EccLevel::High => "high",
    }
}

fn ec_level_of(name: &str) -> Option<EccLevel> {
    match name {
        "low" => Some(EccLevel::Low),
        "medium" => Some(EccLevel::Medium),
        "quartile" => Some(EccLevel::Quartile),
        "high" => Some(EccLevel::High),
        _ => None,
    }
}

/// One decoded code as TSX sees it: the corners flattened to x,y pairs.
fn native_qr(decoded: DecodedQr) -> NativeDecodedQr {
    NativeDecodedQr {
        text: decoded.text,
        bytes: decoded.bytes,
        ec_level: ec_level_name(decoded.ec_level).to_owned(),
        version: u32::from(decoded.version),
        corners: decoded
            .corners
            .iter()
            .flat_map(|&(x, y)| [x as u32, y as u32])
            .collect(),
    }
}

/// Turns the TSX options into the crate's, refusing a level it does not know.
fn to_generate_options(options: NativeGenerateOptions) -> Result<GenerateOptions, JsError> {
    let ec_level = match options.ec_level.as_deref() {
        None => None,
        Some(name) => Some(ec_level_of(name).ok_or_else(|| {
            invalid("ecLevel must be \"low\", \"medium\", \"quartile\" or \"high\"")
        })?),
    };
    Ok(GenerateOptions {
        ec_level,
        quiet_zone: options.quiet_zone.unwrap_or(4),
    })
}

fn to_scan_options(options: NativeScanOptions) -> ScanOptions {
    ScanOptions {
        timeout: options.timeout_ms.map(Duration::from_millis),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_permission_has_the_name_tsx_uses() {
        assert_eq!(
            permission_name(PermissionState::Undetermined),
            "undetermined"
        );
        assert_eq!(permission_name(PermissionState::Granted), "granted");
        assert_eq!(permission_name(PermissionState::Denied), "denied");
    }

    #[test]
    fn each_level_has_the_name_tsx_uses_and_back() {
        for name in ["low", "medium", "quartile", "high"] {
            assert_eq!(ec_level_name(ec_level_of(name).expect("known")), name);
        }
        assert!(ec_level_of("ultra").is_none());
    }

    #[test]
    fn an_unknown_level_is_refused_and_a_known_one_is_kept() {
        let refused = NativeGenerateOptions {
            ec_level: Some("ultra".to_owned()),
            ..NativeGenerateOptions::default()
        };
        assert!(to_generate_options(refused).is_err());

        let high = NativeGenerateOptions {
            ec_level: Some("high".to_owned()),
            quiet_zone: Some(2),
        };
        let options = to_generate_options(high).expect("known level");
        assert_eq!(
            options,
            GenerateOptions {
                ec_level: Some(EccLevel::High),
                quiet_zone: 2,
            }
        );
    }

    #[test]
    fn unset_options_keep_the_defaults() {
        assert_eq!(
            to_generate_options(NativeGenerateOptions::default()).ok(),
            Some(GenerateOptions::default())
        );
        assert_eq!(
            to_scan_options(NativeScanOptions::default()),
            ScanOptions::default()
        );
    }

    #[test]
    fn the_corners_flatten_to_xy_pairs() {
        let native = native_qr(DecodedQr {
            text: "corner".to_owned(),
            bytes: b"corner".to_vec(),
            ec_level: EccLevel::High,
            version: 1,
            corners: [(1, 2), (3, 4), (5, 6), (7, 8)],
        });
        assert_eq!(native.corners, vec![1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(native.ec_level, "high");
        assert_eq!(native.version, 1);
    }
}
