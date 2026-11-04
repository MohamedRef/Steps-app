//! Layout by window width, never by device name (Material window classes).
// adapted from kabar-app main: crates/core/src/layout.rs

/// Compact under 600dp (the S24 Ultra upright), medium under 840dp, expanded
/// from 840dp (the Tab S11, the Mac window).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidthClass {
    Compact,
    Medium,
    Expanded,
}

impl WidthClass {
    pub fn from_dp(width: f64) -> Self {
        if width < 600.0 {
            Self::Compact
        } else if width < 840.0 {
            Self::Medium
        } else {
            Self::Expanded
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Medium => "medium",
            Self::Expanded => "expanded",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breakpoints() {
        assert_eq!(WidthClass::from_dp(412.0), WidthClass::Compact);
        assert_eq!(WidthClass::from_dp(600.0), WidthClass::Medium);
        assert_eq!(WidthClass::from_dp(839.9), WidthClass::Medium);
        assert_eq!(WidthClass::from_dp(1100.0), WidthClass::Expanded);
    }
}
