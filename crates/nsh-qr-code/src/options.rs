//! The types the crate's calls take and return: correction levels, options, and results.

/// A QR code's error correction level.
///
/// `qrcode` names these L/M/Q/H; `rqrr` reports the raw format-bits level, where 0 is M,
/// 1 is L, 2 is H and 3 is Q. The conversions live here so both spell one name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EccLevel {
    /// Corrects up to 7% of the modules.
    Low,
    /// Corrects up to 15%.
    Medium,
    /// Corrects up to 25%.
    Quartile,
    /// Corrects up to 30%.
    High,
}

impl EccLevel {
    /// The level behind `rqrr`'s `ecc_level` number (0 M, 1 L, 2 H, 3 Q), or `None` when the
    /// bits name no level.
    pub(crate) fn from_rqrr(level: u16) -> Option<Self> {
        match level {
            0 => Some(Self::Medium),
            1 => Some(Self::Low),
            2 => Some(Self::High),
            3 => Some(Self::Quartile),
            _ => None,
        }
    }

    /// The level as `qrcode` names it.
    pub(crate) fn to_qrcode(self) -> qrcode::EcLevel {
        match self {
            Self::Low => qrcode::EcLevel::L,
            Self::Medium => qrcode::EcLevel::M,
            Self::Quartile => qrcode::EcLevel::Q,
            Self::High => qrcode::EcLevel::H,
        }
    }
}

/// The options of one scan. Start from [`ScanOptions::default`] and set what you need.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScanOptions {
    /// The longest the scan waits before it resolves with `ERR_QR_TIMEOUT`. Off by default;
    /// `Some(Duration::ZERO)` is refused as an invalid argument, and on wasm32 the option is
    /// accepted and unused, as the web has no timer runtime.
    pub timeout: Option<std::time::Duration>,
}

/// The options of one generation. Start from [`GenerateOptions::default`] and set what you need.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerateOptions {
    /// The correction level. `None` picks by payload length: the strongest that keeps the
    /// code small enough to scan (see [`crate::generate`]).
    pub ec_level: Option<EccLevel>,
    /// The white border around the code, in modules. The QR spec asks for 4; at most 100 is
    /// accepted, or `generate` refuses the call.
    pub quiet_zone: u32,
}

impl Default for GenerateOptions {
    fn default() -> Self {
        Self {
            ec_level: None,
            quiet_zone: 4,
        }
    }
}

/// One QR code found in a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedQr {
    /// The payload as text, with invalid UTF-8 replaced; `bytes` keeps what was encoded.
    pub text: String,
    /// The payload as it was encoded.
    pub bytes: Vec<u8>,
    /// The code's correction level.
    pub ec_level: EccLevel,
    /// The QR version, 1 to 40.
    pub version: u8,
    /// The code's corners in frame pixels: top-left, top-right, bottom-right, bottom-left.
    pub corners: [(usize, usize); 4],
}

/// A generated QR code: the modules to draw, row-major, dark first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QrMatrix {
    /// The code's width and height in modules.
    pub size: usize,
    /// One entry per module: `true` is dark.
    pub modules: Vec<bool>,
    /// The code's correction level.
    pub ec_level: EccLevel,
    /// The QR version, 1 to 40.
    pub version: u8,
    /// The border [`Self::to_svg`] draws, kept from the generation's options.
    quiet_zone: u32,
}

impl QrMatrix {
    pub(crate) fn new(
        size: usize,
        modules: Vec<bool>,
        ec_level: EccLevel,
        version: u8,
        quiet_zone: u32,
    ) -> Self {
        Self {
            size,
            modules,
            ec_level,
            version,
            quiet_zone,
        }
    }

    /// The code as an SVG string: dark modules on white, with the quiet zone border.
    pub fn to_svg(&self) -> String {
        let colors: Vec<qrcode::Color> = self
            .modules
            .iter()
            .map(|dark| {
                if *dark {
                    qrcode::Color::Dark
                } else {
                    qrcode::Color::Light
                }
            })
            .collect();
        // One module per unit, so the SVG scales to whatever size the app draws it at.
        qrcode::render::Renderer::<qrcode::render::svg::Color>::new(
            &colors,
            self.size,
            self.quiet_zone,
        )
        .module_dimensions(1, 1)
        .build()
    }
}
