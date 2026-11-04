//! Bundled UI fonts (ATTRIBUTION.md), all SIL OFL 1.1 from the Fontsource
//! 5.3.0 packages: IBM Plex Sans Arabic for the Arabic UI and Plus Jakarta
//! Sans for medical terms and other Latin text. Embedded as `data:` URLs, so
//! every device shows the same faces offline.
// adapted from kabar-app main: crates/app/src/fonts.rs

macro_rules! font {
    ($path:literal) => {
        include_bytes!(concat!("../../../assets/fonts/", $path))
    };
}

/// Latin basic and extended ranges, as Fontsource splits them.
const LATIN: &str = "U+0000-00FF,U+0131,U+0152-0153,U+02BB-02BC,U+02C6,U+02DA,U+02DC,U+0304,U+0308,U+0329,U+2000-206F,U+20AC,U+2122,U+2191,U+2193,U+2212,U+2215,U+FEFF,U+FFFD";
const LATIN_EXT: &str = "U+0100-02BA,U+02BD-02C5,U+02C7-02CC,U+02CE-02D7,U+02DD-02FF,U+0304,U+0308,U+0329,U+1D00-1DBF,U+1E00-1E9F,U+1EF2-1EFF,U+2020,U+20A0-20AB,U+20AD-20C0,U+2113,U+2C60-2C7F,U+A720-A7FF";
const ARABIC: &str = "U+0600-06FF,U+0750-077F,U+0870-088E,U+0890-0891,U+0897-08E1,U+08E3-08FF,U+200C-200E,U+2010-2011,U+204F,U+2E41,U+FB50-FDFF,U+FE70-FE74,U+FE76-FEFC";

/// (family, weight, unicode-range, woff2 bytes)
const FACES: [(&str, &str, &str, &[u8]); 5] = [
    (
        "Plus Jakarta Sans",
        "200 800",
        LATIN,
        font!("plus-jakarta-sans/plus-jakarta-sans-latin-wght-normal.woff2"),
    ),
    (
        "Plus Jakarta Sans",
        "200 800",
        LATIN_EXT,
        font!("plus-jakarta-sans/plus-jakarta-sans-latin-ext-wght-normal.woff2"),
    ),
    (
        "IBM Plex Sans Arabic",
        "400",
        ARABIC,
        font!("ibm-plex-sans-arabic/ibm-plex-sans-arabic-arabic-400-normal.woff2"),
    ),
    (
        "IBM Plex Sans Arabic",
        "600",
        ARABIC,
        font!("ibm-plex-sans-arabic/ibm-plex-sans-arabic-arabic-600-normal.woff2"),
    ),
    (
        "IBM Plex Sans Arabic",
        "700",
        ARABIC,
        font!("ibm-plex-sans-arabic/ibm-plex-sans-arabic-arabic-700-normal.woff2"),
    ),
];

/// Whether this build bundles the Arabic faces. Not on desktop Linux: there
/// WebKitGTK draws words carrying tashkeel smaller and above the baseline
/// when an Arabic web font has a `unicode-range` (Kabar #227), so Arabic
/// falls back to the system's Noto Arabic. Android and the Mac keep Plex.
const ARABIC_FACES: bool = !cfg!(target_os = "linux");

/// The `@font-face` rules, built once.
pub fn css() -> &'static str {
    static CSS: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    CSS.get_or_init(|| {
        FACES
            .iter()
            .filter(|(_, _, range, _)| ARABIC_FACES || *range != ARABIC)
            .map(|(family, weight, range, bytes)| {
                format!(
                    "@font-face {{ font-family: \"{family}\"; font-style: normal; font-display: swap; font-weight: {weight}; \
                     src: url(data:font/woff2;base64,{}) format(\"woff2\"); unicode-range: {range}; }}\n",
                    base64(bytes)
                )
            })
            .collect()
    })
}

/// Standard base64 with padding (small enough not to need a crate).
fn base64(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = u32::from(b[0]) << 16 | u32::from(b[1]) << 8 | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(ABC[(n >> (18 - 6 * i) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_face_is_woff2() {
        for (family, _, _, bytes) in FACES {
            assert_eq!(&bytes[..4], b"wOF2", "{family}");
        }
        let arabic = if ARABIC_FACES { 3 } else { 0 };
        assert_eq!(css().matches("@font-face").count(), 2 + arabic);
    }

    #[test]
    fn base64_matches_rfc_4648_vectors() {
        let cases = [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foobar", "Zm9vYmFy")];
        for (plain, encoded) in cases {
            assert_eq!(base64(plain.as_bytes()), encoded);
        }
    }
}
