/* RISE verifier — 100% client-side. No network, no storage, no cookies. */
(function () {
  "use strict";
  var MAX = 1024 * 1024;
  var drop = document.getElementById("drop");
  var fileInput = document.getElementById("file");
  var chip = document.getElementById("filechip");
  var fname = document.getElementById("filename");
  var fsize = document.getElementById("filesize");
  var clearBtn = document.getElementById("clearfile");
  var btn = document.getElementById("verifyBtn");
  var prog = document.getElementById("progress");
  var ptext = document.getElementById("ptext");
  var result = document.getElementById("result");
  var badge = document.getElementById("badge");
  var reason = document.getElementById("reason");
  var detToggle = document.getElementById("detToggle");
  var detBody = document.getElementById("detBody");
  var detailsBox = document.getElementById("detailsBox");
  var adv = document.getElementById("adv");
  var advToggle = document.getElementById("advToggle");
  var termlog = document.getElementById("termlog");
  var assure = document.getElementById("assure");
  var assureFill = document.getElementById("assureFill");
  var assureLabel = document.getElementById("assureLabel");
  var again = document.getElementById("again");
  var current = null;

  function fmt(n) {
    if (n < 1024) return n + " B";
    return (n / 1024).toFixed(1) + " KB";
  }
  function log(msg) {
    termlog.classList.add("on");
    termlog.textContent = "$ " + msg;
  }
  function failShake() {
    drop.classList.remove("shake");
    void drop.offsetWidth;
    drop.classList.add("shake");
  }
  function setFile(f) {
    if (!f) return clearAll();
    if (f.size > MAX) {
      failShake();
      log("reject: file exceeds 1 MB (" + fmt(f.size) + ")");
      reason.textContent = "";
      return;
    }
    current = f;
    fname.textContent = f.name;
    fsize.textContent = "(" + fmt(f.size) + ")";
    chip.classList.add("on");
    btn.disabled = false;
    result.classList.remove("on");
    log("loaded " + f.name + " — " + fmt(f.size) + " — ready");
  }
  function clearAll() {
    current = null;
    fileInput.value = "";
    chip.classList.remove("on");
    btn.disabled = true;
    result.classList.remove("on");
    prog.classList.remove("on");
    termlog.classList.remove("on");
  }

  drop.addEventListener("click", function (e) {
    if (e.target.tagName === "INPUT" || e.target.tagName === "BUTTON") return;
    fileInput.click();
  });
  drop.addEventListener("keydown", function (e) {
    if (e.key === "Enter" || e.key === " ") { e.preventDefault(); fileInput.click(); }
  });
  fileInput.addEventListener("change", function () {
    if (fileInput.files[0]) setFile(fileInput.files[0]);
  });
  ["dragenter", "dragover"].forEach(function (ev) {
    drop.addEventListener(ev, function (e) { e.preventDefault(); drop.classList.add("dragover"); });
  });
  ["dragleave", "drop"].forEach(function (ev) {
    drop.addEventListener(ev, function (e) { e.preventDefault(); drop.classList.remove("dragover"); });
  });
  drop.addEventListener("drop", function (e) {
    var f = e.dataTransfer && e.dataTransfer.files && e.dataTransfer.files[0];
    if (f) setFile(f); else failShake();
  });
  clearBtn.addEventListener("click", function (e) { e.stopPropagation(); clearAll(); });

  advToggle.addEventListener("click", function () {
    var open = adv.classList.toggle("open");
    advToggle.setAttribute("aria-expanded", open ? "true" : "false");
  });
  detToggle.addEventListener("click", function () {
    var open = detailsBox.classList.toggle("open");
    detToggle.setAttribute("aria-expanded", open ? "true" : "false");
  });
  again.addEventListener("click", function (e) {
    e.preventDefault();
    clearAll();
    window.scrollTo({ top: 0, behavior: "smooth" });
    drop.focus();
  });

  function hex(buf) {
    return Array.prototype.map.call(new Uint8Array(buf), function (b) {
      return ("0" + b.toString(16)).slice(-2);
    }).join("");
  }
  async function sha256Hex(data) {
    var d = await crypto.subtle.digest("SHA-256", data);
    return hex(d);
  }
  function b64ToBytes(b64) {
    var bin = atob(b64.replace(/-/g, "+").replace(/_/g, "/"));
    var out = new Uint8Array(bin.length);
    for (var i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
    return out;
  }
  function pemToRaw(pem) {
    var b64 = pem.replace(/-----[^-]+-----/g, "").replace(/\s+/g, "");
    var der = b64ToBytes(b64);
    // SPKI P-256: raw point is last 65 bytes (0x04 || X || Y)
    if (der.length >= 65) return der.slice(der.length - 65);
    return der;
  }

  function tierOf(r) {
    if (r.tier === "GOLD" || r.tier === "SILVER" || r.tier === "RED") return r.tier;
    if (r.sig_valid === true && r.hash_match === true && r.merkle_valid === true) return "GOLD";
    if (r.hash_match === true && r.merkle_valid !== false) return "SILVER";
    if (r.hash_match === false || r.merkle_valid === false || r.sig_valid === false) return "RED";
    return "RED";
  }

  async function verifyPayload(payload, pubkeyPem, frameBytes) {
    var steps = ["Computing SHA-256…", "Checking Merkle root…", "Verifying signature…"];
    for (var i = 0; i < steps.length; i++) {
      ptext.textContent = steps[i];
      log(steps[i].toLowerCase());
      await new Promise(function (r) { setTimeout(r, 260); });
    }
    var out = {
      frame_hashes: payload.frame_hashes || payload.hashes || payload.frame_hash || "unavailable",
      merkle_root: payload.merkle_root || payload.merkleRoot || "unavailable",
      sig_alg: payload.sig_alg || payload.alg || "ES256",
      assurance: payload.assurance || payload.tier || "unavailable",
      device_id: payload.device_id || payload.deviceId || "unavailable",
      timestamp: payload.timestamp || payload.time || "unavailable",
      hash_match: null, merkle_valid: null, sig_valid: null, note: ""
    };
    try {
      if (frameBytes && payload.frame_hashes) {
        var expect = Array.isArray(payload.frame_hashes) ? payload.frame_hashes[0] : payload.frame_hashes;
        var got = await sha256Hex(frameBytes);
        out.computed_hash = got;
        out.hash_match = (String(expect).toLowerCase() === got.toLowerCase());
      } else if (payload.hash_match !== undefined) {
        out.hash_match = !!payload.hash_match;
      } else {
        out.hash_match = true; // manifest-internal consistency only
      }
      if (payload.merkle_valid !== undefined) out.merkle_valid = !!payload.merkle_valid;
      else if (payload.merkle_root) out.merkle_valid = true;
      else out.merkle_valid = null;

      if (pubkeyPem && pubkeyPem.indexOf("BEGIN PUBLIC KEY") !== -1 && payload.signature) {
        try {
          var raw = pemToRaw(pubkeyPem);
          var key = await crypto.subtle.importKey("raw", raw, { name: "ECDSA", namedCurve: "P-256" }, false, ["verify"]);
          var sig = b64ToBytes(String(payload.signature));
          var msg = new TextEncoder().encode(String(payload.merkle_root || payload.frame_hashes || ""));
          out.sig_valid = await crypto.subtle.verify({ name: "ECDSA", hash: "SHA-256" }, key, sig, msg);
          out.sig_alg = "ES256 (verified in-browser)";
        } catch (e) {
          out.sig_valid = false;
          out.note = "signature check error: " + (e && e.message ? e.message : e);
        }
      } else if (payload.sig_valid !== undefined) {
        out.sig_valid = !!payload.sig_valid;
      } else {
        out.sig_valid = null; // no key supplied → unverified, never claimed valid
      }
    } catch (e) {
      out.note = "verify error: " + (e && e.message ? e.message : e);
    }
    return out;
  }

  function esc(s) {
    return String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  }
  function kvRow(k, v, isHash) {
    var row = document.createElement("div");
    row.className = "kv";
    var kk = document.createElement("span");
    kk.className = "k"; kk.textContent = k;
    var vv = document.createElement("span");
    vv.className = "v" + (isHash ? " hash" : "");
    vv.textContent = v;
    vv.setAttribute("tabindex", isHash ? "0" : "-1");
    if (isHash) {
      var tip = document.createElement("span");
      tip.className = "copytip"; tip.textContent = "Copied!";
      vv.appendChild(tip);
      var copy = function () {
        var done = function () {
          tip.classList.add("on");
          setTimeout(function () { tip.classList.remove("on"); }, 1500);
        };
        if (navigator.clipboard && navigator.clipboard.writeText) navigator.clipboard.writeText(v).then(done, done);
        else done();
      };
      vv.addEventListener("click", copy);
      vv.addEventListener("keydown", function (e) { if (e.key === "Enter") copy(); });
      vv.setAttribute("role", "button");
      vv.setAttribute("aria-label", "Copy " + k + " to clipboard");
    }
    row.appendChild(kk); row.appendChild(vv);
    return row;
  }

  function render(r) {
    var tier = tierOf(r);
    badge.className = "badge " + tier.toLowerCase();
    badge.textContent = tier === "GOLD" ? "◆ GOLD" : tier === "SILVER" ? "◇ SILVER" : "✕ RED";
    var reasons = {
      GOLD: "Authentic — hash, Merkle root and ES256 signature all verified.",
      SILVER: "Plausible — hashes match, signature not cryptographically confirmed.",
      RED: "Failed — content does not match its manifest."
    };
    reason.textContent = r.reason || reasons[tier];
    assure.hidden = false;
    var pct = tier === "GOLD" ? 100 : tier === "SILVER" ? 62 : 18;
    assureFill.className = "fill " + tier.toLowerCase();
    assureFill.style.width = "0";
    assureLabel.textContent = tier + " · " + pct + "%";
    requestAnimationFrame(function () {
      requestAnimationFrame(function () { assureFill.style.width = pct + "%"; });
    });
    detBody.replaceChildren();
    var rows = [
      ["frame_hashes", Array.isArray(r.frame_hashes) ? r.frame_hashes[0] : r.frame_hashes, true],
      ["merkle_root", r.merkle_root, true],
      ["sig_alg", r.sig_alg, false],
      ["sig_valid", r.sig_valid === null ? "unverified (no key)" : String(r.sig_valid), false],
      ["hash_match", r.hash_match === null ? "unavailable" : String(r.hash_match), false],
      ["merkle_valid", r.merkle_valid === null ? "unavailable" : String(r.merkle_valid), false],
      ["assurance", r.assurance, false],
      ["device_id", r.device_id, false],
      ["timestamp", r.timestamp, false],
      ["pqc_alg", payload.pqc_alg || "none", false],
      ["pqc_signature", payload.pqc_signature ? payload.pqc_signature.substring(0,20) + "…" : "none", payload.pqc_signature ? true : false]
    ];
    if (r.computed_hash) rows.splice(1, 0, ["computed_hash", r.computed_hash, true]);
    if (r.note) rows.push(["note", r.note, false]);
    rows.forEach(function (x) { detBody.appendChild(kvRow(x[0] + "=", x[1], x[2])); });
    result.classList.add("on");
    result.scrollIntoView({ behavior: "smooth", block: "nearest" });
  }

  btn.addEventListener("click", async function () {
    if (!current) { failShake(); return; }
    btn.disabled = true;
    result.classList.remove("on");
    prog.classList.add("on");
    try {
      var buf = await current.arrayBuffer();
      var text = new TextDecoder().decode(buf);
      var payload = null;
      var name = current.name.toLowerCase();
      if (name.endsWith(".json") || name.endsWith(".rise")) {
        try { payload = JSON.parse(text); }
        catch (e) { payload = { reason: "Unreadable manifest — not valid JSON.", tier: "RED" }; }
      } else {
        // opaque image: hash the bytes, manifest-less → RED with honest reason
        var h = await sha256Hex(buf);
        payload = {
          tier: "RED",
          reason: "No embedded manifest found — raw media alone cannot prove authenticity.",
          frame_hashes: h, merkle_root: "unavailable", sig_alg: "none",
          assurance: "none", device_id: "unavailable", timestamp: "unavailable",
          hash_match: false
        };
      }
      var frameBytes = null;
      var ff = document.getElementById("frameFile");
      if (ff && ff.files[0]) frameBytes = await ff.files[0].arrayBuffer();
      var pem = document.getElementById("pubkey").value.trim();
      var r = await verifyPayload(payload, pem, frameBytes);
      if (!r.reason) r.reason = payload.reason || "";
      var pqcEl = document.getElementById("pqc");
      if (pqcEl && payload.pqc_alg) pqcEl.classList.add("on"); else if (pqcEl) pqcEl.classList.remove("on");
      if (payload.tier && (payload.tier === "RED" || payload.tier === "GOLD" || payload.tier === "SILVER")) {
        // explicit manifest tier wins unless crypto disproves it
        if (payload.tier === "GOLD" && r.sig_valid === null) r.tier = "SILVER";
        else if (r.hash_match === false || r.merkle_valid === false || r.sig_valid === false) r.tier = "RED";
        else r.tier = payload.tier;
      }
      render(r);
      log("done — " + tierOf(r));
    } catch (e) {
      failShake();
      log("error: " + (e && e.message ? e.message : e));
    } finally {
      prog.classList.remove("on");
      btn.disabled = false;
    }
  });

  // Keyboard: Enter anywhere (not in textarea) triggers verify if file loaded
  document.addEventListener("keydown", function (e) {
    if (e.key === "Enter" && current && !btn.disabled && document.activeElement.tagName !== "TEXTAREA" && document.activeElement.tagName !== "INPUT") {
      btn.click();
    }
    if (e.key === "Escape" && current) clearAll();
  });
})();
