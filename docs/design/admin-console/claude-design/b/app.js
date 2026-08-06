/* AnxietyWatch2 Service Console — Direction B
   Dependency-free progressive enhancement.
   Every value this file touches is already server-rendered in the HTML.
   With JS disabled the page is static but complete and correct. */
(function () {
  "use strict";

  /* ---- 1. Ages advance on screen ---------------------------------------
     A page opened at 23:00 and read at 03:30 must not still say "40s ago". */
  var UNITS = [[86400,"d"],[3600,"h"],[60,"m"],[1,"s"]];
  function age(sec) {
    if (sec < 0) sec = 0;
    if (sec < 60) return Math.floor(sec) + "s";
    for (var i = 0; i < UNITS.length; i++) {
      if (sec >= UNITS[i][0]) {
        var whole = Math.floor(sec / UNITS[i][0]);
        var rem = Math.floor((sec % UNITS[i][0]) / (UNITS[i + 1] ? UNITS[i + 1][0] : 1));
        return UNITS[i + 1] && rem && whole < 10
          ? whole + UNITS[i][1] + " " + rem + UNITS[i + 1][1]
          : whole + UNITS[i][1];
      }
    }
    return "0s";
  }

  /* data-age-s is the age at render time, as the server computed it.
     data-since is the absolute observation time, kept for semantics and for
     the title attribute. We advance from the rendered age so a static mockup
     stays coherent, which is also what a server-rendered page wants. */
  var loadedAt = Date.now();
  var els = [].slice.call(document.querySelectorAll("[data-age-s], [data-since]"));
  function tickAges() {
    var elapsed = (Date.now() - loadedAt) / 1000;
    els.forEach(function (el) {
      var base = parseFloat(el.getAttribute("data-age-s"));
      if (isNaN(base)) {
        var t = Date.parse(el.getAttribute("data-since"));
        if (isNaN(t)) return;
        base = (loadedAt - t) / 1000;
      }
      var sec = base + elapsed;
      var target = el.querySelector(".fresh__age") || el;
      target.textContent = age(sec) + " ago";
      var limit = parseInt(el.getAttribute("data-expect-s") || "0", 10);
      if (limit) {
        var over = sec > limit;
        el.classList.toggle("fresh--over", over);
        /* A status whose own check is older than its expected interval
           degrades to unknown automatically. */
        var owner = el.closest("[data-degrades]");
        if (owner && over) owner.setAttribute("data-degraded", "true");
      }
    });
  }
  tickAges();
  setInterval(tickAges, 1000);

  /* ---- 2. Console clock ------------------------------------------------- */
  var clock = document.querySelector("[data-clock]");
  function tickClock() {
    if (!clock) return;
    var d = new Date();
    clock.textContent = d.toISOString().slice(11, 19) + "Z";
  }
  tickClock(); setInterval(tickClock, 1000);

  /* ---- 3. Document-level feed freshness ----------------------------------
     When the feed dies the DOCUMENT degrades, as an overlay. Card state
     underneath is left exactly as last observed. */
  var feedAt = Date.now() - (parseInt(document.body.getAttribute("data-feed-offset-s") || "0", 10) * 1000);
  var FEED_LIMIT = 90; /* seconds */
  var feedAgeEls = [].slice.call(document.querySelectorAll("[data-feed-age]"));
  function tickFeed() {
    var sec = (Date.now() - feedAt) / 1000;
    feedAgeEls.forEach(function (el) { el.textContent = age(sec); });
    document.body.setAttribute("data-feed", sec > FEED_LIMIT ? "stale" : "live");
  }
  tickFeed(); setInterval(tickFeed, 1000);

  document.addEventListener("click", function (e) {
    var t = e.target.closest("[data-action]");
    if (!t) return;
    var act = t.getAttribute("data-action");

    if (act === "refresh") { feedAt = Date.now(); tickFeed(); }

    /* Demo affordance: kill the feed so the degraded document state can be
       inspected without waiting for a real outage. */
    if (act === "kill-feed") { feedAt = Date.now() - 1080 * 1000; tickFeed(); }

    if (act === "open") {
      var m = document.getElementById(t.getAttribute("data-target"));
      if (m) { m.hidden = false; var f = m.querySelector("input,button,textarea"); if (f) f.focus(); }
    }
    if (act === "close") {
      var host = t.closest("[role='dialog']") || t.closest(".scrim");
      if (host) host.hidden = true;
    }
  });

  document.addEventListener("keydown", function (e) {
    if (e.key !== "Escape") return;
    [].forEach.call(document.querySelectorAll(".scrim:not([hidden])"), function (s) {
      if (!s.hasAttribute("data-no-escape")) s.hidden = true;
    });
  });

  /* ---- 4. Destructive confirmation: the object's name must be typed ------ */
  [].forEach.call(document.querySelectorAll("[data-confirm-phrase]"), function (form) {
    var phrase = form.getAttribute("data-confirm-phrase");
    var input = form.querySelector("[data-confirm-input]");
    var go = form.querySelector("[data-confirm-go]");
    if (!input || !go) return;
    go.disabled = true;
    input.addEventListener("input", function () {
      go.disabled = input.value.trim() !== phrase;
    });
  });

  /* ---- 5. One-time secret: reveal is deliberate, and it does not linger -- */
  [].forEach.call(document.querySelectorAll("[data-secret]"), function (box) {
    var val = box.querySelector(".secret__value");
    var btn = box.querySelector("[data-secret-reveal]");
    var timer = box.querySelector("[data-secret-timer]");
    var left = 30, iv = null;
    if (!val || !btn) return;
    btn.addEventListener("click", function () {
      var hidden = val.getAttribute("data-hidden") === "true";
      val.setAttribute("data-hidden", hidden ? "false" : "true");
      btn.textContent = hidden ? "Hide" : "Reveal";
      if (iv) { clearInterval(iv); iv = null; }
      if (hidden) {
        left = 30;
        iv = setInterval(function () {
          left--;
          if (timer) timer.textContent = "Auto-hides in " + left + "s";
          if (left <= 0) {
            clearInterval(iv); iv = null;
            val.setAttribute("data-hidden", "true");
            btn.textContent = "Reveal";
            if (timer) timer.textContent = "Hidden. Reveal again if you still have it unsaved.";
          }
        }, 1000);
      } else if (timer) { timer.textContent = "Hidden."; }
    });
  });

  /* ---- 6. Dirty-state tracking for ordinary forms ------------------------ */
  [].forEach.call(document.querySelectorAll("form[data-dirty-track]"), function (form) {
    var bar = form.querySelector(".savebar");
    var state = form.querySelector(".savebar__state");
    var initial = new FormData(form);
    function serialise(fd) { var o = []; fd.forEach(function (v, k) { o.push(k + "=" + v); }); return o.join("&"); }
    var base = serialise(initial);
    function check() {
      var dirty = serialise(new FormData(form)) !== base;
      if (bar) bar.setAttribute("data-dirty", dirty ? "true" : "false");
      if (state) state.textContent = dirty
        ? "Unsaved changes on this form."
        : "No changes since last save.";
      [].forEach.call(form.querySelectorAll("[data-needs-dirty]"), function (b) { b.disabled = !dirty; });
    }
    form.addEventListener("input", check);
    form.addEventListener("change", check);
    check();
    window.addEventListener("beforeunload", function (e) {
      if (bar && bar.getAttribute("data-dirty") === "true") { e.preventDefault(); e.returnValue = ""; }
    });
  });
})();
