//! Whether the connection costs money: automatic update downloads wait for
//! Wi-Fi. Android asks ConnectivityManager (mobile data or Data Saver; an
//! unknown answer counts as metered). The Mac is never metered.
// adapted from kabar-app main: crates/app/src/platform/mod.rs (metered_network)

pub fn metered() -> bool {
    #[cfg(target_os = "android")]
    {
        super::install::network_metered().unwrap_or(true)
    }
    #[cfg(not(target_os = "android"))]
    {
        false
    }
}
