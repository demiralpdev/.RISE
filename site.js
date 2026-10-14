/* ═══════════════════════════════════════════════════════════════════════════
   RISE — Multi-Page Router + Smooth Watercolor-Wash Eye Mask Generator
   + Overhead Swaying Leaf Shadows & Dual-Deck Seamless Video Loops
   Untouched: verify.js handles all cryptographic verification on #verify
   ═══════════════════════════════════════════════════════════════════════════ */
(function () {
  "use strict";

  /* ── 1. Live Clock ── */
  var clockEl = document.getElementById("clock");
  function updateClock() {
    if (!clockEl) return;
    var d = new Date();
    var p = function (n) {
      return n < 10 ? "0" + n : "" + n;
    };
    clockEl.textContent =
      p(d.getHours()) + ":" + p(d.getMinutes()) + ":" + p(d.getSeconds());
  }
  updateClock();
  setInterval(updateClock, 1000);

  /* ── 2. Multi-Page View Router + 1st vs 2nd+ Visit Logic ── */
  var VISIT_KEY = "rise_visit_count";
  var MODE_OVERRIDE_KEY = "rise_visit_mode_override";
  var views = ["showcase", "workspace", "download", "install"];

  var visitPill = document.getElementById("visit-pill");
  var routeToast = document.getElementById("route-toast");
  var toastGoShowcase = document.getElementById("toast-go-showcase");
  var toastClose = document.getElementById("toast-close");

  function setActiveView(viewName, pushHash) {
    if (views.indexOf(viewName) === -1) viewName = "showcase";

    views.forEach(function (v) {
      var sec = document.getElementById("view-" + v);
      if (sec) sec.classList.toggle("active", v === viewName);
    });

    var navTabs = document.querySelectorAll("[data-nav-view]");
    navTabs.forEach(function (btn) {
      if (btn.classList.contains("nav-tab")) {
        btn.classList.toggle(
          "active",
          btn.getAttribute("data-nav-view") === viewName
        );
      }
    });

    if (pushHash && history.replaceState) {
      var url = new URL(window.location.href);
      url.searchParams.set("view", viewName);
      history.replaceState(null, "", url.toString());
    }

    window.scrollTo({ top: 0, behavior: "smooth" });
  }

  function updateVisitBadge(isReturningMode) {
    if (!visitPill) return;
    if (isReturningMode) {
      visitPill.textContent = "2nd+ Visit: Verify";
    } else {
      visitPill.textContent = "1st Visit: Intro";
    }
  }

  function initVisitRouting() {
    var params = new URLSearchParams(window.location.search);
    var explicitView = params.get("view");
    var hash = (window.location.hash || "").replace("#", "");

    var rawCount = parseInt(localStorage.getItem(VISIT_KEY) || "0", 10);
    if (isNaN(rawCount)) rawCount = 0;
    var override = localStorage.getItem(MODE_OVERRIDE_KEY);

    localStorage.setItem(VISIT_KEY, String(rawCount + 1));

    var hashMap = {
      verify: "workspace",
      workspace: "workspace",
      download: "download",
      install: "install",
      showcase: "showcase"
    };

    if (explicitView && views.indexOf(explicitView) !== -1) {
      setActiveView(explicitView, false);
      updateVisitBadge(rawCount >= 1);
      return;
    }

    if (hash && hashMap[hash]) {
      setActiveView(hashMap[hash], false);
      updateVisitBadge(rawCount >= 1);
      return;
    }

    var shouldRouteToWorkspace =
      override === "returning" || (override !== "first" && rawCount >= 1);

    updateVisitBadge(shouldRouteToWorkspace);

    if (shouldRouteToWorkspace) {
      setActiveView("workspace", false);
      if (routeToast) routeToast.classList.add("show");
    } else {
      setActiveView("showcase", false);
    }
  }

  document.addEventListener("click", function (e) {
    var trigger = e.target.closest("[data-nav-view]");
    if (!trigger) return;
    var targetView = trigger.getAttribute("data-nav-view");
    if (!targetView) return;
    e.preventDefault();
    if (routeToast) routeToast.classList.remove("show");
    setActiveView(targetView, true);
  });

  if (toastGoShowcase) {
    toastGoShowcase.addEventListener("click", function () {
      if (routeToast) routeToast.classList.remove("show");
      setActiveView("showcase", true);
    });
  }

  if (toastClose) {
    toastClose.addEventListener("click", function () {
      if (routeToast) routeToast.classList.remove("show");
    });
  }

  if (visitPill) {
    visitPill.addEventListener("click", function () {
      var currentOverride = localStorage.getItem(MODE_OVERRIDE_KEY);
      if (currentOverride === "first") {
        localStorage.setItem(MODE_OVERRIDE_KEY, "returning");
        updateVisitBadge(true);
        setActiveView("workspace", true);
        if (routeToast) routeToast.classList.add("show");
      } else {
        localStorage.setItem(MODE_OVERRIDE_KEY, "first");
        localStorage.setItem(VISIT_KEY, "1");
        updateVisitBadge(false);
        if (routeToast) routeToast.classList.remove("show");
        setActiveView("showcase", true);
      }
    });
  }

  initVisitRouting();

  /* ── 3. Smooth Watercolor-Wash Eye Mask Generator (Zero Speckles/Dots!) ──
     Builds a genuine wet-on-wet watercolor wash silhouette using overlapping
     soft-feathered brush blooms and smooth organic lobes (no sandy pixel noise). */
  function applyWatercolorEyeMask() {
    var bleedEl = document.getElementById("eye-bleed-bg");
    if (!bleedEl) return;

    var w = 960;
    var h = 640;
    var canvas = document.createElement("canvas");
    canvas.width = w;
    canvas.height = h;
    var ctx = canvas.getContext("2d");
    if (!ctx) return;

    ctx.clearRect(0, 0, w, h);

    /* Helper to paint a soft elliptical watercolor brush bloom */
    function paintBloom(cx, cy, rx, ry, rotDeg, innerStop, peakAlpha) {
      ctx.save();
      ctx.translate(cx, cy);
      ctx.rotate((rotDeg * Math.PI) / 180);
      ctx.scale(rx, ry);
      var grad = ctx.createRadialGradient(0, 0, innerStop, 0, 0, 1.0);
      grad.addColorStop(0, "rgba(0,0,0," + peakAlpha + ")");
      grad.addColorStop(0.55, "rgba(0,0,0," + peakAlpha * 0.72 + ")");
      grad.addColorStop(0.82, "rgba(0,0,0," + peakAlpha * 0.25 + ")");
      grad.addColorStop(1.0, "rgba(0,0,0,0)");
      ctx.fillStyle = grad;
      ctx.beginPath();
      ctx.arc(0, 0, 1.0, 0, Math.PI * 2);
      ctx.fill();
      ctx.restore();
    }

    /* 1. Core solid eye & eyelid wash (keeps iris, eyelashes, and eyelids 100% crisp) */
    paintBloom(w * 0.50, h * 0.50, w * 0.37, h * 0.33, -2, 0.58, 1.0);
    paintBloom(w * 0.48, h * 0.49, w * 0.40, h * 0.29, 3, 0.50, 1.0);

    /* 2. Surrounding organic watercolor brush washes (brow, temple, tear duct, lower cheek)
       Matching the soft cloud-like watercolor splashes in Screenshot-2026-10-14-at-11.20.48.png */
    var washes = [
      /* Upper brow left & center-left washes */
      [0.34, 0.28, 0.16, 0.14, -18, 0.15, 0.78],
      [0.45, 0.22, 0.18, 0.13, 8, 0.12, 0.82],
      [0.57, 0.23, 0.17, 0.14, 14, 0.15, 0.85],
      [0.68, 0.29, 0.16, 0.14, 24, 0.12, 0.76],

      /* Left tear-duct & inner corner watercolor bleed */
      [0.21, 0.44, 0.15, 0.14, -12, 0.10, 0.74],
      [0.17, 0.54, 0.14, 0.12, 15, 0.08, 0.62],
      [0.25, 0.62, 0.16, 0.13, -8, 0.12, 0.72],

      /* Right outer corner & temple watercolor bleed */
      [0.77, 0.42, 0.16, 0.14, 18, 0.12, 0.80],
      [0.82, 0.52, 0.15, 0.13, -10, 0.10, 0.68],
      [0.75, 0.62, 0.16, 0.14, 12, 0.10, 0.74],

      /* Lower eyelid & cheekbone soft watercolor washes */
      [0.36, 0.72, 0.17, 0.14, 10, 0.12, 0.76],
      [0.48, 0.76, 0.19, 0.14, -6, 0.12, 0.80],
      [0.61, 0.74, 0.17, 0.14, -14, 0.12, 0.75],

      /* Outer delicate translucent watercolor mist lobes */
      [0.29, 0.34, 0.14, 0.11, -25, 0.0, 0.48],
      [0.73, 0.34, 0.13, 0.11, 28, 0.0, 0.46],
      [0.42, 0.81, 0.14, 0.10, 6, 0.0, 0.44],
      [0.56, 0.80, 0.15, 0.10, -8, 0.0, 0.42]
    ];

    for (var i = 0; i < washes.length; i++) {
      var b = washes[i];
      paintBloom(w * b[0], h * b[1], w * b[2], h * b[3], b[4], b[5], b[6]);
    }

    var dataUrl = canvas.toDataURL("image/png");
    bleedEl.style.webkitMaskImage = "url(" + dataUrl + ")";
    bleedEl.style.maskImage = "url(" + dataUrl + ")";
  }

  applyWatercolorEyeMask();

  /* ── 4. Reusable Zero-White-Flash Dual-Deck Video Loop ── */
  function initOpaqueDualDeckLoop(idA, idB, dissolveSec) {
    var deckA = document.getElementById(idA);
    var deckB = document.getElementById(idB);
    if (!deckA || !deckB) return;

    deckA.muted = true;
    deckB.muted = true;
    deckA.playsInline = true;
    deckB.playsInline = true;

    var activeDeck = deckA;
    var standbyDeck = deckB;
    var isDissolving = false;

    activeDeck.classList.add("is-base");
    var pInit = activeDeck.play();
    if (pInit && pInit.catch) pInit.catch(function () {});

    var startDissolve = function () {
      if (isDissolving) return;
      isDissolving = true;

      var incoming = standbyDeck;
      var outgoing = activeDeck;

      try {
        incoming.currentTime = 0;
      } catch (_) {}

      var pIn = incoming.play();
      if (pIn && pIn.catch) pIn.catch(function () {});

      outgoing.classList.remove("is-fading-in");
      outgoing.classList.add("is-base");

      incoming.classList.remove("is-base");
      void incoming.offsetWidth;
      incoming.classList.add("is-fading-in");

      activeDeck = incoming;
      standbyDeck = outgoing;

      setTimeout(function () {
        incoming.classList.remove("is-fading-in");
        incoming.classList.add("is-base");
        outgoing.classList.remove("is-base", "is-fading-in");
        try {
          outgoing.pause();
          outgoing.currentTime = 0;
        } catch (_) {}
        isDissolving = false;
      }, Math.round(dissolveSec * 1000) + 40);
    };

    var monitor = function () {
      if (activeDeck && !activeDeck.paused && !isDissolving) {
        var dur = activeDeck.duration;
        var t = activeDeck.currentTime || 0;
        if (dur && !isNaN(dur) && dur > 2 && t >= dur - dissolveSec) {
          startDissolve();
        }
      }
      requestAnimationFrame(monitor);
    };

    [deckA, deckB].forEach(function (v) {
      v.addEventListener("ended", function () {
        if (v === activeDeck) startDissolve();
      });
    });

    requestAnimationFrame(monitor);
  }

  /* Initialize seamless loops for both the Eye video (./hero-eye.mp4) and Leaf Shadows video (./leaf-shadows.mp4) */
  initOpaqueDualDeckLoop("hero-eye-video-a", "hero-eye-video-b", 1.15);
  initOpaqueDualDeckLoop("leaf-video-a", "leaf-video-b", 1.3);

  /* ── 5. Sample Manifest Loader for Untouched verify.js ── */
  var sampleManifests = [
    {
      filename: "IST_4082.rise.json",
      payload: {
        rise_version: "0.2.5",
        format: "image/rise-jpeg-jumbf",
        asset_title: "IST_4082.rise",
        captured_at: "2026-10-14T08:42:19Z",
        hardware_attestation: "StrongBox_Keymaster_4.1",
        merkle_root_sha256:
          "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        signature_scheme: "ES256+ML-DSA-65",
        rfc3161_tsa: "https://freetsa.org/tsr"
      }
    },
    {
      filename: "ARC_4089.rise.json",
      payload: {
        rise_version: "0.2.5",
        format: "image/rise-jpeg-jumbf",
        asset_title: "ARC_4089.rise",
        captured_at: "2026-10-14T11:15:04Z",
        hardware_attestation: "Apple_Secure_Enclave_T2",
        merkle_root_sha256:
          "8f434346648f6b96df89dda901c5176b10a6d83961dd3c1ac88b59b2dc327aa4",
        signature_scheme: "ES256+ML-DSA-65",
        rfc3161_tsa: "https://freetsa.org/tsr"
      }
    },
    {
      filename: "ALP_4104.rise.json",
      payload: {
        rise_version: "0.2.5",
        format: "video/rvid-mp4-uuid",
        asset_title: "ALP_4104.rise",
        captured_at: "2026-10-14T16:08:51Z",
        hardware_attestation: "TPM_2.0_VBS",
        merkle_root_sha256:
          "2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae",
        signature_scheme: "ES256+ML-DSA-65",
        rfc3161_tsa: "https://freetsa.org/tsr"
      }
    }
  ];

  function loadSampleIntoVerifier(index) {
    var item = sampleManifests[index] || sampleManifests[0];
    setActiveView("workspace", true);

    var fileIn = document.getElementById("fileIn");
    if (!fileIn || typeof DataTransfer === "undefined") return;

    var jsonText = JSON.stringify(item.payload, null, 2);
    var file = new File([jsonText], item.filename, {
      type: "application/json"
    });
    var dt = new DataTransfer();
    dt.items.add(file);
    fileIn.files = dt.files;
    fileIn.dispatchEvent(new Event("change", { bubbles: true }));
  }

  var sampleBtns = document.querySelectorAll("[data-sample-idx]");
  sampleBtns.forEach(function (btn) {
    btn.addEventListener("click", function () {
      var idx = parseInt(btn.getAttribute("data-sample-idx") || "0", 10);
      loadSampleIntoVerifier(idx);
    });
  });

  /* ── 6. Accessible OS Install Tabs ── */
  var tabButtons = Array.prototype.slice.call(
    document.querySelectorAll('[role="tab"][data-tab]')
  );
  var tabPanels = Array.prototype.slice.call(
    document.querySelectorAll('[role="tabpanel"]')
  );

  function activateTab(btn, focusTab) {
    var target = btn.getAttribute("data-tab");
    tabButtons.forEach(function (b) {
      var active = b === btn;
      b.setAttribute("aria-selected", active ? "true" : "false");
      b.setAttribute("tabindex", active ? "0" : "-1");
      if (active && focusTab) b.focus();
    });
    tabPanels.forEach(function (p) {
      p.classList.toggle("active", p.id === "panel-" + target);
    });
  }

  tabButtons.forEach(function (btn, idx) {
    btn.addEventListener("click", function () {
      activateTab(btn, false);
    });
    btn.addEventListener("keydown", function (e) {
      var nextIdx = null;
      if (e.key === "ArrowRight") nextIdx = (idx + 1) % tabButtons.length;
      else if (e.key === "ArrowLeft")
        nextIdx = (idx - 1 + tabButtons.length) % tabButtons.length;
      else if (e.key === "Home") nextIdx = 0;
      else if (e.key === "End") nextIdx = tabButtons.length - 1;
      if (nextIdx !== null) {
        e.preventDefault();
        activateTab(tabButtons[nextIdx], true);
      }
    });
  });

  /* ── 7. Copy Buttons ── */
  var copyBtns = document.querySelectorAll("[data-copy]");
  copyBtns.forEach(function (btn) {
    btn.addEventListener("click", function () {
      var id = btn.getAttribute("data-copy");
      var el = document.getElementById(id);
      if (!el) return;
      var text = el.innerText || el.textContent || "";
      if (navigator.clipboard && navigator.clipboard.writeText) {
        navigator.clipboard.writeText(text).then(function () {
          var orig = btn.textContent;
          btn.textContent = "Copied";
          setTimeout(function () {
            btn.textContent = orig;
          }, 1400);
        });
      }
    });
  });
})();
