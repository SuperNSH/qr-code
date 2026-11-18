//! Making QR codes, through `qrcode`.

use qrcode::{
    types::QrError as EncodeError,
    {QrCode, Version},
};

use crate::{
    error::{ErrorKind, QrError},
    options::{EccLevel, GenerateOptions, QrMatrix},
};

/// The largest quiet zone `generate` accepts: qrcode's renderer draws (size + 2 * quiet_zone)
/// squared modules, so past this the border alone ties up the caller for hours.
const MAX_QUIET_ZONE: u32 = 100;

/// The QR code for `data`.
///
/// Empty data is refused with `ERR_QR_INVALID_ARGUMENT`; data no QR code can hold with
/// `ERR_QR_PAYLOAD_TOO_LARGE`. With no `ec_level` asked for, the payload's length picks one:
/// up to 255 bytes High, up to 1071 Medium, longer Low — smaller payloads get the strongest
/// correction, and past those sizes the stronger levels stop fitting the versions that still
/// scan reliably.
pub fn generate(data: &[u8], options: &GenerateOptions) -> Result<QrMatrix, QrError> {
    if data.is_empty() {
        return Err(QrError::new(ErrorKind::InvalidArgument));
    }
    if options.quiet_zone > MAX_QUIET_ZONE {
        return Err(QrError::new(ErrorKind::InvalidArgument));
    }
    let ec_level = options.ec_level.unwrap_or_else(|| auto_ecc(data.len()));
    let code =
        QrCode::with_error_correction_level(data, ec_level.to_qrcode()).map_err(encode_error)?;
    let version = match code.version() {
        Version::Normal(version) => version as u8,
        // with_error_correction_level builds normal codes only; a micro one would be a bug in
        // the encoder, and this call reports it instead of panicking.
        Version::Micro(_) => return Err(QrError::new(ErrorKind::Backend)),
    };
    let modules: Vec<bool> = code
        .to_colors()
        .into_iter()
        .map(|color| color == qrcode::Color::Dark)
        .collect();
    Ok(QrMatrix::new(
        code.width(),
        modules,
        ec_level,
        version,
        options.quiet_zone,
    ))
}

/// The correction a payload of `len` bytes gets when none was asked for.
pub(crate) fn auto_ecc(len: usize) -> EccLevel {
    if len <= 255 {
        EccLevel::High
    } else if len <= 1071 {
        EccLevel::Medium
    } else {
        EccLevel::Low
    }
}

/// What an encoder refusal reports: too much data is the caller's payload; anything else is
/// the encoder failing on input it should take.
fn encode_error(error: EncodeError) -> QrError {
    match error {
        EncodeError::DataTooLong => QrError::new(ErrorKind::PayloadTooLarge),
        _ => QrError::new(ErrorKind::Backend),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::EccLevel;

    fn matrix_for(payload: &[u8]) -> QrMatrix {
        generate(payload, &GenerateOptions::default()).expect("encodes")
    }

    #[test]
    fn empty_data_is_refused() {
        let error = generate(&[], &GenerateOptions::default()).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidArgument);
    }

    #[test]
    fn data_no_code_can_hold_is_too_large() {
        // 3000 bytes pick Low, whose largest code holds 2953.
        let error = generate(&[7; 3000], &GenerateOptions::default()).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::PayloadTooLarge);
    }

    #[test]
    fn the_level_picks_at_the_255_and_1071_boundaries() {
        assert_eq!(matrix_for(&[1; 255]).ec_level, EccLevel::High);
        assert_eq!(matrix_for(&[1; 256]).ec_level, EccLevel::Medium);
        assert_eq!(matrix_for(&[1; 1071]).ec_level, EccLevel::Medium);
        assert_eq!(matrix_for(&[1; 1072]).ec_level, EccLevel::Low);
        assert_eq!(auto_ecc(255), EccLevel::High);
        assert_eq!(auto_ecc(1071), EccLevel::Medium);
        assert_eq!(auto_ecc(1072), EccLevel::Low);
    }

    #[test]
    fn an_asked_for_level_is_kept() {
        let options = GenerateOptions {
            ec_level: Some(EccLevel::Quartile),
            ..GenerateOptions::default()
        };
        let matrix = generate(b"explicit", &options).expect("encodes");
        assert_eq!(matrix.ec_level, EccLevel::Quartile);
    }

    #[test]
    fn the_matrix_matches_its_version() {
        let matrix = matrix_for(b"A");
        // A version-1 code is 21 modules across.
        assert_eq!(matrix.version, 1);
        assert_eq!(matrix.size, 21);
        assert_eq!(matrix.modules.len(), 21 * 21);
    }

    #[test]
    fn the_svg_shapes_the_code_with_its_quiet_zone() {
        let matrix = matrix_for(b"SVG");
        let svg = matrix.to_svg();
        assert!(svg.starts_with("<?xml"), "{svg}");
        assert!(svg.contains("<svg"), "{svg}");
        assert!(
            svg.contains("fill=\"#000\""),
            "the dark modules draw in black"
        );
        let side = matrix.size + 8;
        assert!(svg.contains(&format!("width=\"{side}\"")), "{svg}");
        assert!(svg.contains("</svg>"), "{svg}");
    }

    #[test]
    fn a_huge_quiet_zone_is_refused() {
        let huge = GenerateOptions {
            quiet_zone: MAX_QUIET_ZONE + 1,
            ..GenerateOptions::default()
        };
        let error = generate(b"border", &huge).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidArgument);

        let sane = GenerateOptions {
            quiet_zone: MAX_QUIET_ZONE,
            ..GenerateOptions::default()
        };
        assert!(generate(b"border", &sane).is_ok());
    }

    #[test]
    fn a_zero_quiet_zone_draws_no_border() {
        let options = GenerateOptions {
            quiet_zone: 0,
            ..GenerateOptions::default()
        };
        let matrix = generate(b"no border", &options).expect("encodes");
        let svg = matrix.to_svg();
        assert!(svg.contains(&format!("width=\"{}\"", matrix.size)), "{svg}");
    }
}
