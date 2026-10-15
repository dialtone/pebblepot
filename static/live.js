// Live sync. The sse extension refetches the board on `changed` (see #live in board.html) and
// re-opens a stream that failed with backoff. This adds: a refetch whenever the stream (re)opens
// and when the page comes back (iPad wake), and a fresh stream on wake if the old one is closed.
(function () {
  var source;
  function live() { return document.getElementById("live"); }
  function refresh() { if (live()) htmx.trigger(live(), "refresh"); }
  function wake() {
    if (source && source.readyState === EventSource.CLOSED && live()) {
      htmx.swap(live(), live().outerHTML, { swapStyle: "outerHTML" }); // reconnects, then refreshes
    } else refresh();
  }
  document.addEventListener("htmx:sseOpen", function (e) { source = e.detail.source; refresh(); });
  document.addEventListener("htmx:sseError", function (e) { source = e.detail.source; });
  document.addEventListener("visibilitychange", function () { if (document.visibilityState === "visible") wake(); });
  window.addEventListener("pageshow", function (e) { if (e.persisted) wake(); });
  // A refetch mostly brings back the pots and rows already shown: skip those swaps so nothing flickers.
  function squash(html) { return html.replace(/>\s+</g, "><").trim(); }
  document.addEventListener("htmx:oobBeforeSwap", function (e) {
    var now = e.detail.target, next = e.detail.fragment.firstElementChild;
    if (now.matches("#pots, .rows") && next && squash(next.outerHTML) === squash(now.outerHTML)) e.detail.shouldSwap = false;
  });
})();
