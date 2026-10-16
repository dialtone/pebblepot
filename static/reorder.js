// ✏️ Edit mode: drag a row by its ⠿ handle to move it within its section. Pointer events, so it
// works with a finger on the iPad and a phone as well as a mouse. On drop, the section's new order
// (the rows on screen, so a searched list moves only what's shown) is posted; the reply redraws it.
(function () {
  var drag = null;
  function ids(rows) {
    return Array.prototype.map.call(rows.querySelectorAll(":scope > [data-item]"), function (r) {
      return r.getAttribute("data-item");
    }).join(",");
  }
  document.addEventListener("pointerdown", function (e) {
    var handle = e.target.closest("[data-drag-handle]");
    if (!handle || drag) return;
    var row = handle.closest("[data-item]");
    e.preventDefault();
    handle.setPointerCapture(e.pointerId);
    drag = { row: row, rows: row.parentElement, handle: handle, before: ids(row.parentElement) };
    row.classList.add("dragging");
  });
  document.addEventListener("pointermove", function (e) {
    if (!drag) return;
    e.preventDefault();
    var under = document.elementFromPoint(e.clientX, e.clientY);
    var over = under && under.closest("[data-item]");
    if (!over || over === drag.row || over.parentElement !== drag.rows) return;
    var rows = Array.prototype.slice.call(drag.rows.children);
    // Moving down puts it after the row it's over; moving up, before it. Works for the 2-column grid too.
    drag.rows.insertBefore(drag.row, rows.indexOf(over) > rows.indexOf(drag.row) ? over.nextSibling : over);
  });
  function drop() {
    if (!drag) return;
    var d = drag;
    drag = null;
    d.row.classList.remove("dragging");
    var after = ids(d.rows);
    if (after === d.before) return;
    var search = document.querySelector("[data-search]");
    htmx.ajax("POST", d.handle.getAttribute("data-reorder-path"), {
      source: d.rows, swap: "none",
      values: { section: d.handle.getAttribute("data-reorder-section"), order: after, q: search ? search.value : "" }
    });
  }
  document.addEventListener("pointerup", drop);
  document.addEventListener("pointercancel", drop);
  // Don't let a live refresh redraw a section while a row is being dragged in it.
  document.addEventListener("htmx:oobBeforeSwap", function (e) {
    if (drag && e.detail.target === drag.rows) e.detail.shouldSwap = false;
  });
})();
