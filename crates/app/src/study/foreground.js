// One bridge for the app's lifetime, not one per Study mount. Android's
// WebView reports visibility when the Activity resumes; pageshow also covers
// a page coming back from the browser's cache. Neither touches the item on
// screen, the scroll position or Back.
// Adapted from batna-app main: crates/app/src/study/foreground.js.
window.__stepsStudyForegroundSend = (message) => dioxus.send(message);
if (!window.__stepsStudyForegroundInit) {
  window.__stepsStudyForegroundInit = true;
  const visible = () => {
    if (document.visibilityState === "visible") {
      window.__stepsStudyForegroundSend("visible");
    }
  };
  document.addEventListener("visibilitychange", visible);
  window.addEventListener("pageshow", visible);
}
// Keep the eval channel open for the app's lifetime.
await new Promise(() => {});
