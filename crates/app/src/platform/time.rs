//! Timers that work on every target: tokio's on the Mac and Android, the
//! browser's (`setTimeout`) on the web, where tokio has no timer.

use std::future::Future;
use std::time::Duration;

/// Waits `d` without blocking the UI.
pub async fn sleep(d: Duration) {
    #[cfg(not(target_arch = "wasm32"))]
    tokio::time::sleep(d).await;
    #[cfg(target_arch = "wasm32")]
    gloo_timers::future::sleep(d).await;
}

/// `fut`'s output, or `Err(Elapsed)` after `limit`.
#[cfg_attr(target_os = "android", allow(dead_code))] // Android speaks through JNI
pub async fn timeout<F: Future>(limit: Duration, fut: F) -> Result<F::Output, Elapsed> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::time::timeout(limit, fut).await.map_err(|_| Elapsed)
    }
    #[cfg(target_arch = "wasm32")]
    {
        use futures_util::future::{Either, select};
        let fut = std::pin::pin!(fut);
        let timer = std::pin::pin!(sleep(limit));
        match select(fut, timer).await {
            Either::Left((out, _)) => Ok(out),
            Either::Right(_) => Err(Elapsed),
        }
    }
}

/// The time limit ran out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(target_os = "android", allow(dead_code))]
pub struct Elapsed;

/// Runs blocking work off the UI thread where there are threads; the web has
/// one thread, so it runs in place there.
pub async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::task::spawn_blocking(f).await.map_err(|e| e.to_string())
    }
    #[cfg(target_arch = "wasm32")]
    {
        Ok(f())
    }
}
