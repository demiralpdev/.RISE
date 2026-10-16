// RISE Showcase, Fine-Art Brushstroke Canvases & Navigation Controller (site.js)
// Keeps verify.js completely untouched while managing:
// 1. Hand-Painted Watercolor Frame for the hero eye (layered wash mask + pigment canvases)
// 2. Seamless dual-video ping-pong crossfades (hero eye, top leaf shadows, tulip dropzone)
// 3. View switching ([data-nav-view]: showcase, workspace, download, install) + live clock
// 4. 5 Custom-Painted Impasto/Gouache Artworks in the Showcase + 3 Setup Card Paintings + Horizon Banner
// 5. Multi-Directional Atelier Showcase Choreography (5 directions -> Center Spotlight Easel)
// 6. Self-Completing On-Scroll Left-to-Right Terminal Typewriter (100% completion once in view)
// 7. Refractive Water-Drop Wave on 02 Verify (#drop + #tulip-ripple-canvas + #fileIn)

(function () {
  "use strict";

  // ── 1. Hand-Painted Watercolor Frame for the Hero Eye (locked seed = 918273) ──
  // Layered wet-on-wet technique: an organic base blob is recursively deformed,
  // then dozens of further-deformed copies are glazed on top of each other at
  // very low opacity. The overlap builds an opaque core with soft, feathery,
  // bleeding edges — the way stacked washes behave on cold-press paper.
  // A per-pixel pass then adds granulation, paper tooth and the darker pigment
  // ring that forms where a wash dries. Three outputs:
  //   #eye-wc-under → washes, blooms & splatter on the paper (multiply)
  //   #eye-bleed-bg → alpha mask for the video (CSS mask-image)
  //   #eye-wc-over  → pooled pigment + granulation over the video rim (multiply)
  function paintWatercolorEyeFrame() {
    const wrap = document.getElementById("eye-bleed-wrap");
    const film = document.getElementById("eye-bleed-bg");
    const under = document.getElementById("eye-wc-under");
    const over = document.getElementById("eye-wc-over");
    if (!wrap || !film || !under || !over) return null;

    const cssW = film.clientWidth;
    const cssH = film.clientHeight;
    if (cssW < 40 || cssH < 40) return null; // view hidden — try again later

    const k = Math.min(window.devicePixelRatio || 1, 1.5);
    const W = Math.round(cssW * k);
    const H = Math.round(cssH * k);
    const cx = W / 2;
    const cy = H / 2;
    const N = W * H;

    // Deterministic RNG (mulberry32) → the same painting on every visit
    let seed = 918273;
    const rand = () => {
      seed = (seed + 0x6d2b79f5) | 0;
      let t = seed;
      t = Math.imul(t ^ (t >>> 15), t | 1);
      t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
    const gauss = () => {
      let u = 0;
      while (u === 0) u = rand();
      return Math.sqrt(-2 * Math.log(u)) * Math.cos(2 * Math.PI * rand());
    };
    const between = (lo, hi) => lo + rand() * (hi - lo);
    const smooth = (t) => (t <= 0 ? 0 : t >= 1 ? 1 : t * t * (3 - 2 * t));

    // ── Paper tooth / granulation field (value noise, 3 octaves) ──
    const perm = new Uint8Array(512);
    for (let i = 0; i < 256; i++) perm[i] = i;
    for (let i = 255; i > 0; i--) {
      const j = Math.floor(rand() * (i + 1));
      const t = perm[i];
      perm[i] = perm[j];
      perm[j] = t;
    }
    for (let i = 0; i < 256; i++) perm[i + 256] = perm[i];
    const lattice = (x, y) => perm[perm[x & 255] + (y & 255)] / 255;
    function vnoise(x, y) {
      const xi = Math.floor(x);
      const yi = Math.floor(y);
      const xf = x - xi;
      const yf = y - yi;
      const u = xf * xf * (3 - 2 * xf);
      const v = yf * yf * (3 - 2 * yf);
      const a = lattice(xi, yi);
      const b = lattice(xi + 1, yi);
      const c = lattice(xi, yi + 1);
      const d = lattice(xi + 1, yi + 1);
      return a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v;
    }
    const speck = (x, y) => {
      let h = (x * 374761393 + y * 668265263) | 0;
      h = Math.imul(h ^ (h >>> 13), 1274126177);
      return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
    };
    const tooth = new Float32Array(N);
    const f1 = 1 / (15 * k);
    const f2 = 1 / (5.5 * k);
    const f3 = 1 / (2.1 * k);
    for (let y = 0, i = 0; y < H; y++) {
      for (let x = 0; x < W; x++, i++) {
        tooth[i] =
          0.5 * vnoise(x * f1, y * f1) +
          0.3 * vnoise(x * f2 + 31.7, y * f2 + 11.3) +
          0.2 * vnoise(x * f3 + 7.1, y * f3 + 53.9);
      }
    }

    // ── Brush geometry ──
    // Superellipse "blob" with per-vertex wetness (v): high v bleeds, low v stays crisp
    function blob(ox, oy, rx, ry, n, count, vLo, vHi) {
      const pts = [];
      const rot = rand() * Math.PI * 2;
      for (let i = 0; i < count; i++) {
        const t = rot + (i / count) * Math.PI * 2 + gauss() * 0.05;
        const c = Math.cos(t);
        const s = Math.sin(t);
        const r = 1 / Math.pow(Math.pow(Math.abs(c), n) + Math.pow(Math.abs(s), n), 1 / n);
        const j = 1 + gauss() * 0.03;
        pts.push({ x: ox + c * r * rx * j, y: oy + s * r * ry * j, v: between(vLo, vHi) });
      }
      return pts;
    }

    // Recursive midpoint displacement (fractal edge, scale-proportional)
    function deform(pts, iters, spread) {
      for (let it = 0; it < iters; it++) {
        const out = [];
        for (let i = 0; i < pts.length; i++) {
          const a = pts[i];
          const b = pts[(i + 1) % pts.length];
          const len = Math.hypot(b.x - a.x, b.y - a.y);
          const v = (a.v + b.v) * 0.5;
          out.push(a, {
            x: (a.x + b.x) / 2 + gauss() * len * v * spread,
            y: (a.y + b.y) / 2 + gauss() * len * v * spread,
            v: v * between(0.8, 1.15),
          });
        }
        pts = out;
      }
      return pts;
    }

    function trace(ctx, pts) {
      ctx.beginPath();
      ctx.moveTo(pts[0].x, pts[0].y);
      for (let i = 1; i < pts.length; i++) ctx.lineTo(pts[i].x, pts[i].y);
      ctx.closePath();
    }

    // Glaze many slightly different copies → soft, layered watercolor wash
    function wash(ctx, base, o) {
      ctx.fillStyle = o.color;
      ctx.strokeStyle = o.color;
      ctx.lineJoin = "round";
      ctx.lineWidth = (o.edgeW || 1) * k;
      for (let l = 0; l < o.layers; l++) {
        trace(ctx, deform(base, o.iters, o.spread));
        ctx.globalAlpha = o.alpha;
        ctx.fill();
        if (o.edge) {
          ctx.globalAlpha = o.edge; // pigment pooling at each drying edge
          ctx.stroke();
        }
      }
      ctx.globalAlpha = 1;
    }

    // ── A. Video mask: soft wet-edged window ──
    const maskCanvas = document.createElement("canvas");
    maskCanvas.width = W;
    maskCanvas.height = H;
    const mctx = maskCanvas.getContext("2d");
    const maskBase = deform(blob(cx, cy, 0.261 * W, 0.221 * H, 2.3, 11, 0.45, 1.9), 2, 0.12);
    wash(mctx, maskBase, { color: "#000", layers: 44, iters: 4, spread: 0.2, alpha: 0.055 });

    // ── B. Under-painting on the paper ──
    under.width = W;
    under.height = H;
    const uctx = under.getContext("2d");
    uctx.clearRect(0, 0, W, H);

    // Broad warm-sienna wash the eye sits in
    wash(uctx, deform(blob(cx + 0.01 * W, cy, 0.41 * W, 0.37 * H, 2.2, 9, 0.5, 1.8), 2, 0.14), {
      color: "rgb(178, 120, 80)", layers: 30, iters: 4, spread: 0.2, alpha: 0.02, edge: 0.018, edgeW: 0.8,
    });
    // Golden-ochre glaze drifting up-right (echoes the RISE wordmark)
    wash(uctx, deform(blob(cx + 0.23 * W, cy - 0.21 * H, 0.13 * W, 0.105 * H, 2, 8, 0.6, 1.8), 2, 0.14), {
      color: "rgb(204, 154, 74)", layers: 20, iters: 4, spread: 0.22, alpha: 0.018, edge: 0.014, edgeW: 0.8,
    });
    // Rose-umber bloom pooling lower-left
    wash(uctx, deform(blob(cx - 0.24 * W, cy + 0.22 * H, 0.16 * W, 0.13 * H, 2, 8, 0.6, 1.8), 2, 0.14), {
      color: "rgb(166, 96, 72)", layers: 22, iters: 4, spread: 0.22, alpha: 0.02, edge: 0.016, edgeW: 0.8,
    });
    // Whisper of sage (ties into the leaf shadows)
    wash(uctx, deform(blob(cx - 0.25 * W, cy - 0.2 * H, 0.14 * W, 0.11 * H, 2, 7, 0.6, 1.6), 2, 0.14), {
      color: "rgb(128, 138, 104)", layers: 18, iters: 4, spread: 0.22, alpha: 0.02, edge: 0.014, edgeW: 0.8,
    });
    // Tight darker pool hugging the eye, so the video reads as wet paint
    const pool = maskBase.map((p) => ({
      x: cx + (p.x - cx) * 1.07,
      y: cy + (p.y - cy) * 1.09,
      v: p.v,
    }));
    wash(uctx, pool, {
      color: "rgb(150, 96, 62)", layers: 20, iters: 4, spread: 0.18, alpha: 0.028, edge: 0.026, edgeW: 0.9,
    });

    // Splatter & droplets flicked off the brush (two diagonal clusters)
    for (let i = 0; i < 26; i++) {
      const ang = (rand() < 0.56 ? -0.68 : 2.42) + gauss() * 0.42;
      const dist = between(1.04, 1.34);
      const x = Math.max(0.05 * W, Math.min(0.95 * W, cx + Math.cos(ang) * 0.36 * W * dist));
      const y = Math.max(0.06 * H, Math.min(0.94 * H, cy + Math.sin(ang) * 0.33 * H * dist));
      const r = (0.6 + Math.abs(gauss()) * 1.5) * k * (rand() < 0.16 ? 2.6 : 1);
      const drop = deform(blob(x, y, r, r * between(0.8, 1.1), 2, 7, 0.6, 1.2), 2, 0.12);
      trace(uctx, drop);
      uctx.globalAlpha = between(0.18, 0.45);
      uctx.fillStyle = rand() < 0.75 ? "rgb(152, 94, 60)" : "rgb(190, 140, 70)";
      uctx.fill();
      if (r > 3 * k) {
        uctx.globalAlpha = 0.28;
        uctx.lineWidth = 0.8 * k;
        uctx.strokeStyle = "rgb(120, 72, 44)";
        uctx.stroke();
      }
    }
    uctx.globalAlpha = 1;

    // ── C. Per-pixel finishing ──
    // Under: paper granulation + soft fade before the canvas border
    const ud = uctx.getImageData(0, 0, W, H);
    const u = ud.data;
    const borderM = 0.07 * Math.min(W, H);
    for (let y = 0, i = 0; y < H; y++) {
      for (let x = 0; x < W; x++, i++) {
        const p = i * 4 + 3;
        if (u[p] === 0) continue;
        const d = Math.min(x, W - 1 - x, y, H - 1 - y);
        const f = d < borderM ? smooth(d / borderM) : 1;
        u[p] = Math.min(255, u[p] * f * (0.5 + 1.0 * tooth[i]));
      }
    }
    uctx.putImageData(ud, 0, 0);

    // Mask + over-layer from the same accumulated alpha
    const md = mctx.getImageData(0, 0, W, H);
    const m = md.data;
    over.width = W;
    over.height = H;
    const octx = over.getContext("2d");
    const od = octx.createImageData(W, H);
    const o = od.data;
    // Video element box (see .hero-eye-video in styles.css): the painted blob
    // window sits fully inside it, so the blob edge alone defines the dissolve.
    const vx0 = 0.179 * W;
    const vx1 = 0.821 * W;
    const vy0 = 0.203 * H;
    const vy1 = 0.797 * H;
    const ringF = 1 / (85 * k);

    // Paper tone for the edge bleed (:root --bg in styles.css — same value, no new color)
    const PAPER_R = 245;
    const PAPER_G = 242;
    const PAPER_B = 235;
    // Feather band ≈ 0.18 × min(W,H): ~100px at 1180px layout width
    // (film is 128%×140% of the 590×395 wrap → min ≈ 553 css px; 0.18 × 553 ≈ 100).
    const featherPx = 0.18 * Math.min(W, H);
    const blurR = Math.max(1, Math.round(featherPx / 2));

    // Pass 1 — remap glazed layers → raw mask alpha.
    // Widened falloff: t reaches 1 only near full accumulation, so the
    // 1→0 transition spans almost the whole wash penumbra (was: / 0.7).
    const raw = new Float32Array(N);
    for (let y = 0, i = 0; y < H; y++) {
      for (let x = 0; x < W; x++, i++) {
        const a = m[i * 4 + 3] / 255;
        if (a < 0.004) {
          raw[i] = 0;
          continue;
        }
        let t = smooth((a - 0.015) / 0.87);
        raw[i] = t;
      }
    }

    // Spread the raw alpha over the feather band (separable box blur,
    // radius = half the band → the 1→0 falloff ends up ~featherPx wide).
    const tmp = new Float32Array(N);
    const win = blurR * 2 + 1;
    for (let y = 0; y < H; y++) {
      const row = y * W;
      let acc = 0;
      for (let x = -blurR; x <= blurR; x++) acc += raw[row + Math.min(W - 1, Math.max(0, x))];
      for (let x = 0; x < W; x++) {
        tmp[row + x] = acc / win;
        acc += raw[row + Math.min(W - 1, x + blurR + 1)] - raw[row + Math.max(0, x - blurR)];
      }
    }
    const soft = new Float32Array(N);
    for (let x = 0; x < W; x++) {
      let acc = 0;
      for (let y = -blurR; y <= blurR; y++) acc += tmp[Math.min(H - 1, Math.max(0, y)) * W + x];
      for (let y = 0; y < H; y++) {
        soft[y * W + x] = acc / win;
        acc += tmp[Math.min(H - 1, y + blurR + 1) * W + x] - tmp[Math.max(0, y - blurR) * W + x];
      }
    }

    // Pass 2 — grain (transition band only), rim ink, and edge bleed.
    for (let y = 0, i = 0; y < H; y++) {
      for (let x = 0; x < W; x++, i++) {
        const p = i * 4;
        let t = soft[i];
        if (t <= 0.0005) {
          m[p + 3] = 0;
          continue;
        }
        if (t > 1) t = 1;

        const g = tooth[i];
        let ink = 0;
        if (t < 0.999) {
          // Grain lives inside the transition band only: e*t is 0 in the
          // opaque core (t→1) and outside (t→0), so no clean cut survives
          // while core and paper stay untouched.
          const e = 1 - t;
          // Granulation: pigment breaks up into the paper grain at the edge
          t += e * t * ((g - 0.5) * 1.2 + (speck(x, y) - 0.5) * 0.35);
          t = t < 0 ? 0 : t > 1 ? 1 : t;
          // Inverted halo: center stays clear, pigment rises gently outward
          // (monotonic — no gaussian bump, so no ring or hard edge forms).
          const halo = 1 - smooth(t / 0.6);
          const along = 0.35 + 0.65 * vnoise(x * ringF + 91.3, y * ringF + 17.9);
          ink = halo * along * (0.55 + 0.9 * g) * 0.08 * 0.35;
        } else {
          // Center stays fully clear so the eye reads crisp
          ink = 0;
        }
        // Edge bleed: below ~0.20 mask alpha the eye is nearly transparent,
        // so dissolve the pooled pigment into the paper tone (multiply-safe:
        // paper-toned over-glaze ≈ invisible) and fade its alpha — no
        // luminance/color step is left at the cut point.
        let cr = 122;
        let cg = 74;
        let cb = 44;
        if (t < 0.2) {
          const b = smooth(t / 0.2);
          cr = PAPER_R + (cr - PAPER_R) * b;
          cg = PAPER_G + (cg - PAPER_G) * b;
          cb = PAPER_B + (cb - PAPER_B) * b;
          ink *= 0.3 + 0.7 * b; // let a whisper of halo survive to the outer edge
        }
        m[p + 3] = t * 255;
        o[p] = cr;
        o[p + 1] = cg;
        o[p + 2] = cb;
        o[p + 3] = Math.min(255, ink * 255);
      }
    }
    mctx.putImageData(md, 0, 0);
    octx.putImageData(od, 0, 0);

    return { canvas: maskCanvas, W: cssW, H: cssH };
  }

  // Paint once visible, apply the mask, then let the paint "spread" in.
  (function initWatercolorEyeFrame() {
    const wrap = document.getElementById("eye-bleed-wrap");
    const film = document.getElementById("eye-bleed-bg");
    if (!wrap || !film) return;
    wrap.classList.add("wc-pending");

    let painted = null;
    let maskUrl = null;
    let timer = 0;

    function applyMask(url) {
      const css = `url("${url}")`;
      film.style.webkitMaskImage = css;
      film.style.maskImage = css;
      if (maskUrl) URL.revokeObjectURL(maskUrl);
      maskUrl = url;
      requestAnimationFrame(() => {
        wrap.classList.remove("wc-pending");
        wrap.classList.add("is-painted");
      });
    }

    function run() {
      const w = film.clientWidth;
      const h = film.clientHeight;
      if (
        painted &&
        Math.abs(w - painted.W) / painted.W < 0.12 &&
        Math.abs(h - painted.H) / painted.H < 0.12
      ) {
        return;
      }
      let result = null;
      try {
        result = paintWatercolorEyeFrame();
      } catch (err) {
        wrap.classList.remove("wc-pending"); // fall back to the soft CSS mask
        return;
      }
      if (!result) return;
      painted = result;
      result.canvas.toBlob((blob) => {
        if (!blob) return;
        const url = URL.createObjectURL(blob);
        const img = new Image();
        img.onload = () => applyMask(url);
        img.src = url;
      }, "image/png");
    }

    function schedule() {
      clearTimeout(timer);
      timer = setTimeout(run, painted ? 180 : 0);
    }

    if ("ResizeObserver" in window) {
      new ResizeObserver(schedule).observe(wrap);
    } else {
      window.addEventListener("resize", schedule);
    }
    schedule();
  })();

  // ── 2. Seamless Dual-Video Ping-Pong Crossfade Loops ──
  function setupSeamlessVideoLoop(idA, idB, fadeDurationSec = 1.2) {
    const vidA = document.getElementById(idA);
    const vidB = document.getElementById(idB);
    if (!vidA || !vidB) return;

    let active = vidA;
    let standby = vidB;
    let isCrossfading = false;

    function ensurePlay(v) {
      if (!v) return;
      const p = v.play();
      if (p && typeof p.catch === "function") p.catch(() => {});
    }

    ensurePlay(active);
    window.addEventListener("load", () => ensurePlay(active), { once: true });
    window.addEventListener("pageshow", () => ensurePlay(active));

    function checkTime() {
      if (risePerfLiteOn()) return; // perf-lite: crossfade chains stay off
      if (
        !document.hidden &&
        !active.paused &&
        !isCrossfading &&
        active.duration &&
        isFinite(active.duration) &&
        active.duration > fadeDurationSec * 1.5
      ) {
        const remaining = active.duration - active.currentTime;
        if (remaining <= fadeDurationSec) {
          isCrossfading = true;
          standby.currentTime = 0;
          ensurePlay(standby);
          standby.classList.add("is-fading-in");

          setTimeout(() => {
            active.classList.remove("is-base", "is-fading-in");
            active.pause();
            standby.classList.remove("is-fading-in");
            standby.classList.add("is-base");

            const temp = active;
            active = standby;
            standby = temp;
            isCrossfading = false;
          }, fadeDurationSec * 1000);
        }
      }
      requestAnimationFrame(checkTime);
    }

    requestAnimationFrame(checkTime);
  }

  setupSeamlessVideoLoop("hero-eye-video-a", "hero-eye-video-b", 1.15);
  setupSeamlessVideoLoop("leaf-video-a", "leaf-video-b", 1.3);
  setupSeamlessVideoLoop("tulip-video-a", "tulip-video-b", 1.2);

  // ── 2b. Anticipatory video start (warm ~800px before visible) ──
  // Slow-connection guard (2g/slow-2g/saveData → poster + click-to-play, NO
  // autoplay, NO preload) and prefers-reduced-motion guard (poster, no
  // autoplay) run FIRST: on either, strip autoplay, pause, and never
  // auto-warm. Fail-closed: any play() rejection → paused video on poster.
  function riseIsSlowConnection() {
    try {
      const c = navigator.connection || navigator.mozConnection || navigator.webkitConnection;
      if (!c) return false;
      if (c.saveData) return true;
      const t = String(c.effectiveType || "").toLowerCase();
      return t.indexOf("2g") !== -1; // covers "2g" + "slow-2g", not 3g/4g
    } catch (_) {
      return false;
    }
  }
  function risePrefersReducedMotion() {
    try {
      return (
        typeof window.matchMedia === "function" &&
        window.matchMedia("(prefers-reduced-motion: reduce)").matches
      );
    } catch (_) {
      return false;
    }
  }
  const riseSlowConn = riseIsSlowConnection();
  const riseNoAutoplay = riseSlowConn || risePrefersReducedMotion();
  function risePosterizeVideo(v, dropPreload) {
    try {
      v.removeAttribute("autoplay");
      if (dropPreload) v.preload = "none"; // no preload bytes on constrained links
      v.pause();
      if (!v.dataset.riseClickToPlay) {
        v.dataset.riseClickToPlay = "1";
        v.addEventListener(
          "click",
          () => {
            try {
              const p = v.play();
              if (p && typeof p.catch === "function") p.catch(() => {});
            } catch (_) {}
          },
          { passive: true }
        );
      }
    } catch (_) {}
  }
  if (riseNoAutoplay) {
    document.querySelectorAll("video").forEach((v) => risePosterizeVideo(v, riseSlowConn));
  }

  // Warm one video ahead of scroll: muted + playsinline stay (autoplay
  // policy); never restarts a video that is already playing.
  function riseWarmVideo(v) {
    if (!v || risePerfLiteOn() || v.dataset.riseWarmed) return;
    try {
      if (!v.paused && v.readyState >= 2) {
        v.dataset.riseWarmed = "1";
        return; // already playing — do not restart on actual entry
      }
    } catch (_) {}
    v.dataset.riseWarmed = "1";
    try {
      v.muted = true;
      v.setAttribute("playsinline", "");
    } catch (_) {}
    try {
      const p = v.play();
      if (p && typeof p.catch === "function") {
        p.catch(() => {
          try {
            v.pause();
          } catch (_) {} // fail-closed: poster frame remains
        });
      }
    } catch (_) {
      try {
        v.pause();
      } catch (_) {}
    }
  }

  // Guarantee background autoplay videos are running right away
  const kickAutoplayVideos = () => {
    if (riseNoAutoplay || risePerfLiteOn()) return; // poster-only modes
    document.querySelectorAll("video[autoplay]").forEach((v) => {
      if (v.paused) {
        const p = v.play();
        if (p && typeof p.catch === "function") p.catch(() => {});
      }
    });
  };
  kickAutoplayVideos();
  ["pointerdown", "touchstart", "scroll", "keydown"].forEach((evt) => {
    window.addEventListener(evt, kickAutoplayVideos, { once: true, passive: true });
  });

  // IntersectionObserver with a generous 800px rootMargin: each video starts
  // loading/playing well before it becomes visible, so playback is seamless
  // on arrival. On intersect also idle-kicks the single next upcoming video
  // in DOM order (preload ahead, not everything at once).
  try {
    if (!riseNoAutoplay && "IntersectionObserver" in window) {
      const riseAheadIO = new IntersectionObserver(
        (entries) => {
          entries.forEach((en) => {
            if (!en.isIntersecting) return;
            const v = en.target;
            riseWarmVideo(v);
            try {
              riseAheadIO.unobserve(v);
            } catch (_) {}
            try {
              const all = Array.from(document.querySelectorAll("video"));
              const nxt = all[all.indexOf(v) + 1];
              if (nxt && !nxt.dataset.riseWarmed) {
                const kickNext = () => riseWarmVideo(nxt);
                if ("requestIdleCallback" in window) {
                  window.requestIdleCallback(kickNext, { timeout: 2000 });
                } else {
                  setTimeout(kickNext, 1200);
                }
              }
            } catch (_) {}
          });
        },
        { rootMargin: "800px 0px 800px 0px", threshold: 0 }
      );
      document.querySelectorAll("video").forEach((v) => {
        try {
          riseAheadIO.observe(v);
        } catch (_) {}
      });
    }
  } catch (_) {} // fail-closed: poster frame, never a broken player

  // ── 2c. perf-lite mode + video lifecycle (offscreen / tab-hidden pause) ──
  // perf-lite is the one-way switch for struggling devices: reduced motion,
  // data saver, two consecutive sub-42fps seconds, or battery saver (Chromium:
  // getBattery). Effects: html.perf-lite drops every backdrop-filter
  // (styles.css) and videos + crossfade chains go poster-only.
  function risePerfLiteOn() {
    return document.documentElement.classList.contains("perf-lite");
  }
  function riseEngagePerfLite(reason) {
    try {
      if (risePerfLiteOn()) return;
      document.documentElement.classList.add("perf-lite");
      document.documentElement.setAttribute("data-perf-lite-reason", reason);
      document.querySelectorAll("video").forEach((v) => risePosterizeVideo(v, true));
    } catch (_) {}
  }
  // Ink-bleed canvas pixel budget: full 1.75x only on fine-pointer desktops.
  // Phones/tablets (coarse pointer) get 1.25x and perf-lite devices 1x, because
  // these margin-inclusive canvases are repainted every animation frame and are
  // the heaviest paint on weak mobile GPUs (reported: Android showcase jank).
  function riseInkCanvasCap() {
    if (risePerfLiteOn()) return 1;
    try {
      if (window.matchMedia && window.matchMedia("(pointer: coarse)").matches) {
        return 1.25;
      }
    } catch (_) {}
    return 1.75;
  }

  if (risePrefersReducedMotion()) riseEngagePerfLite("reduced-motion");
  else if (riseSlowConn) riseEngagePerfLite("slow-connection");

  function riseWatchFps() {
    if (risePerfLiteOn() || navigator.webdriver) return; // rAF timing is not meaningful under automation
    let frames = 0;
    let badWindows = 0;
    let windows = 0;
    let start = performance.now();
    function tick(now) {
      frames++;
      const elapsed = now - start;
      if (elapsed >= 1000) {
        if ((frames * 1000) / elapsed < 42) badWindows++;
        else badWindows = 0;
        windows++;
        frames = 0;
        start = now;
        if (badWindows >= 2) {
          riseEngagePerfLite("low-fps");
          return;
        }
        if (windows >= 6) return;
      }
      requestAnimationFrame(tick);
    }
    requestAnimationFrame(tick);
  }
  window.addEventListener("load", () => setTimeout(riseWatchFps, 1500), { once: true });

  try {
    if (navigator.getBattery) {
      navigator
        .getBattery()
        .then((b) => {
          const check = () => {
            if (!b.charging && b.level <= 0.2) riseEngagePerfLite("battery");
          };
          check();
          b.addEventListener("levelchange", check);
          b.addEventListener("chargingchange", check);
        })
        .catch(() => {});
    }
  } catch (_) {}

  // All modes: stop decoding videos that left the viewport or the tab.
  function riseResumeVideo(v) {
    if (riseNoAutoplay || risePerfLiteOn()) return;
    try {
      const p = v.play();
      if (p && typeof p.catch === "function") p.catch(() => {});
    } catch (_) {}
  }
  try {
    if ("IntersectionObserver" in window) {
      const riseVisibleIO = new IntersectionObserver(
        (entries) => {
          entries.forEach((en) => {
            const v = en.target;
            try {
              if (!en.isIntersecting) {
                if (!v.paused) {
                  v.dataset.risePausedOffscreen = "1";
                  v.pause();
                }
              } else if (v.dataset.risePausedOffscreen) {
                delete v.dataset.risePausedOffscreen;
                riseResumeVideo(v);
              }
            } catch (_) {}
          });
        },
        { rootMargin: "200px 0px 200px 0px", threshold: 0 }
      );
      document.querySelectorAll("video").forEach((v) => {
        try {
          riseVisibleIO.observe(v);
        } catch (_) {}
      });
    }
  } catch (_) {}

  document.addEventListener("visibilitychange", () => {
    try {
      const vids = document.querySelectorAll("video");
      if (document.hidden) {
        vids.forEach((v) => {
          if (!v.paused) {
            v.dataset.riseHiddenWasPlaying = "1";
            v.pause();
          }
        });
      } else {
        vids.forEach((v) => {
          if (v.dataset.riseHiddenWasPlaying) {
            delete v.dataset.riseHiddenWasPlaying;
            riseResumeVideo(v);
          }
        });
      }
    } catch (_) {}
  });

  // ── 3. Navigation, Live Clock & 1st/2nd Visit Toggle ──
  const viewMap = {
    showcase: "view-showcase",
    workspace: "view-workspace",
    download: "view-download",
    install: "view-install",
  };

  const navLinks = document.querySelectorAll("[data-nav-view]");
  const viewSections = document.querySelectorAll(".view-page");
  const leafBg = document.querySelector(".leaf-shadow-bg");

  function switchView(viewKey, pushState = true) {
    const targetSecId = viewMap[viewKey] || "view-showcase";

    viewSections.forEach((sec) => {
      if (sec.id === targetSecId) {
        sec.classList.add("active");
        sec.removeAttribute("hidden");
      } else {
        sec.classList.remove("active");
        sec.setAttribute("hidden", "");
      }
    });

    navLinks.forEach((link) => {
      if (link.classList.contains("nav-tab")) {
        if (link.getAttribute("data-nav-view") === viewKey) {
          link.classList.add("active");
        } else {
          link.classList.remove("active");
        }
      }
    });

    if (leafBg) {
      leafBg.style.display = "block";
    }

    if (pushState) {
      const hashMap = {
        showcase: "#overview",
        workspace: "#verify",
        download: "#download",
        install: "#install",
      };
      if (hashMap[viewKey]) {
        history.replaceState(null, "", hashMap[viewKey]);
      }
    }

    window.scrollTo({ top: 0, behavior: "smooth" });
  }

  navLinks.forEach((link) => {
    link.addEventListener("click", (e) => {
      const viewKey = link.getAttribute("data-nav-view");
      if (viewKey && viewMap[viewKey]) {
        e.preventDefault();
        switchView(viewKey, true);
      }
    });
  });

  function applyInitialRoute() {
    const h = (window.location.hash || "").toLowerCase();
    const params = new URLSearchParams(window.location.search);
    const qView = params.get("view");

    if (qView && viewMap[qView]) {
      switchView(qView, false);
    } else if (h === "#verify" || h === "#workspace") {
      switchView("workspace", false);
    } else if (h === "#download") {
      switchView("download", false);
    } else if (h === "#install" || h === "#cli") {
      switchView("install", false);
    } else {
      switchView("showcase", false);
    }
  }
  applyInitialRoute();
  window.addEventListener("hashchange", applyInitialRoute);

  // Live Clock in Header
  const clockEl = document.getElementById("clock");
  if (clockEl) {
    function updateClock() {
      const now = new Date();
      clockEl.textContent = now.toTimeString().slice(0, 8);
    }
    updateClock();
    setInterval(updateClock, 1000);
  }

  // Visit Pill & Toast
  const visitPill = document.getElementById("visit-pill");
  const routeToast = document.getElementById("route-toast");
  const toastGoShowcase = document.getElementById("toast-go-showcase");
  const toastClose = document.getElementById("toast-close");

  if (visitPill) {
    visitPill.addEventListener("click", () => {
      const isCurrentlyShowcase = document
        .getElementById("view-showcase")
        ?.classList.contains("active");
      if (isCurrentlyShowcase) {
        visitPill.textContent = "2nd+ Visit: Verify";
        switchView("workspace", true);
      } else {
        visitPill.textContent = "1st Visit: Intro";
        switchView("showcase", true);
      }
    });
  }
  if (toastGoShowcase) {
    toastGoShowcase.addEventListener("click", () => {
      if (routeToast) routeToast.classList.remove("show");
      switchView("showcase", true);
    });
  }
  if (toastClose) {
    toastClose.addEventListener("click", () => {
      if (routeToast) routeToast.classList.remove("show");
    });
  }

  // ── 4. Bespoke Fine-Art Impasto & Gouache Canvas Paintings ──
  function makeRng(seedInit) {
    let s = seedInit >>> 0;
    return function () {
      s = (s * 1664525 + 1013904223) % 4294967296;
      return s / 4294967296;
    };
  }

  function drawImpastoStroke(ctx, x, y, len, thick, angle, color, alpha = 0.85) {
    ctx.save();
    ctx.translate(x, y);
    ctx.rotate(angle);
    ctx.globalAlpha = alpha;
    ctx.fillStyle = color;

    ctx.beginPath();
    ctx.roundRect(-len * 0.5, -thick * 0.5, len, thick, thick * 0.45);
    ctx.fill();

    ctx.globalAlpha = alpha * 0.32;
    ctx.fillStyle = "rgba(255, 250, 235, 0.65)";
    ctx.fillRect(-len * 0.42, -thick * 0.36, len * 0.84, Math.max(1.2, thick * 0.16));
    ctx.restore();
  }

  // 4A. Paint the Atelier Backdrop Canvas (#cap-brush-canvas)
  function paintAtelierBackdrop(progress = 1) {
    const canvas = document.getElementById("cap-brush-canvas");
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const w = canvas.width;
    const h = canvas.height;
    const rng = makeRng(408211);

    const bgGrad = ctx.createLinearGradient(0, 0, w, h);
    bgGrad.addColorStop(0, "#241d16");
    bgGrad.addColorStop(0.48, "#38291d");
    bgGrad.addColorStop(1, "#18211c");
    ctx.fillStyle = bgGrad;
    ctx.fillRect(0, 0, w, h);

    const glow = ctx.createRadialGradient(w * 0.5, h * 0.48, 20, w * 0.5, h * 0.48, w * 0.55);
    glow.addColorStop(0, "rgba(235, 178, 82, 0.28)");
    glow.addColorStop(0.55, "rgba(184, 112, 48, 0.12)");
    glow.addColorStop(1, "rgba(0, 0, 0, 0)");
    ctx.fillStyle = glow;
    ctx.fillRect(0, 0, w, h);

    const palette = [
      "#4a3623",
      "#6b4c2b",
      "#8c6234",
      "#b8843e",
      "#d9a752",
      "#273b30",
      "#365242",
      "#59412a",
    ];

    const totalStrokes = 110;
    const visibleStrokes = Math.max(45, Math.round(totalStrokes * (0.45 + 0.55 * progress)));

    for (let i = 0; i < visibleStrokes; i++) {
      const sx = rng() * w;
      const sy = rng() * h;
      const len = 60 + rng() * 140;
      const thick = 14 + rng() * 28;
      const ang = -0.35 + rng() * 0.7;
      const col = palette[Math.floor(rng() * palette.length)];
      drawImpastoStroke(ctx, sx, sy, len, thick, ang, col, 0.34 + rng() * 0.28);
    }
  }

  // 4B. Paint the 5 Custom Artwork Canvases inside the 5 Showcase Cards
  function paintPieceCanvases() {
    const canvases = document.querySelectorAll(".cap-piece-canvas");
    canvases.forEach((canvas) => {
      const ctx = canvas.getContext("2d");
      if (!ctx) return;
      const w = canvas.width;
      const h = canvas.height;
      const kind = canvas.getAttribute("data-piece-art");

      if (kind === "shutter-iris") {
        const rng = makeRng(10101);
        const bg = ctx.createLinearGradient(0, 0, w, h);
        bg.addColorStop(0, "#2c3d27");
        bg.addColorStop(0.45, "#6b5328");
        bg.addColorStop(1, "#e8c878");
        ctx.fillStyle = bg;
        ctx.fillRect(0, 0, w, h);

        const bgCols = ["#334a2d", "#4d6338", "#87652c", "#b88b3c", "#e5c168", "#f3dfb1", "#994e24"];
        for (let i = 0; i < 140; i++) {
          drawImpastoStroke(
            ctx,
            rng() * w,
            rng() * h,
            42 + rng() * 95,
            10 + rng() * 22,
            -0.5 + rng() * 1.0,
            bgCols[Math.floor(rng() * bgCols.length)],
            0.65
          );
        }

        const cx = w * 0.44;
        const cy = h * 0.52;
        const outerR = h * 0.44;

        ctx.save();
        ctx.beginPath();
        ctx.arc(cx, cy, outerR, 0, Math.PI * 2);
        ctx.fillStyle = "#3b2a18";
        ctx.fill();
        ctx.lineWidth = 10;
        ctx.strokeStyle = "#c99742";
        ctx.stroke();

        ctx.beginPath();
        ctx.arc(cx, cy, outerR * 0.84, 0, Math.PI * 2);
        ctx.lineWidth = 5;
        ctx.strokeStyle = "#f0ce7e";
        ctx.stroke();

        for (let b = 0; b < 8; b++) {
          const a = (b / 8) * Math.PI * 2;
          ctx.save();
          ctx.translate(cx, cy);
          ctx.rotate(a);
          ctx.fillStyle = b % 2 === 0 ? "#594126" : "#735432";
          ctx.strokeStyle = "#e0b65c";
          ctx.lineWidth = 2;
          ctx.beginPath();
          ctx.moveTo(outerR * 0.22, -outerR * 0.12);
          ctx.quadraticCurveTo(outerR * 0.55, -outerR * 0.35, outerR * 0.78, outerR * 0.15);
          ctx.lineTo(outerR * 0.28, outerR * 0.26);
          ctx.closePath();
          ctx.fill();
          ctx.stroke();
          ctx.restore();
        }

        const sunGrad = ctx.createRadialGradient(cx, cy, 4, cx, cy, outerR * 0.45);
        sunGrad.addColorStop(0, "#fffdf2");
        sunGrad.addColorStop(0.5, "#ffd66b");
        sunGrad.addColorStop(1, "rgba(255, 196, 77, 0)");
        ctx.fillStyle = sunGrad;
        ctx.beginPath();
        ctx.arc(cx, cy, outerR * 0.45, 0, Math.PI * 2);
        ctx.fill();

        for (let r = 0; r < 7; r++) {
          drawImpastoStroke(
            ctx,
            cx + 80 + r * 28,
            cy - 45 + r * 8,
            190,
            14,
            -0.42 + r * 0.06,
            "#fff2b8",
            0.48
          );
        }
        ctx.restore();

        const flowerSpots = [
          [w * 0.18, h * 0.74, "#f5ecd7", "#b85d19"],
          [w * 0.26, h * 0.82, "#f0b942", "#5c2e0e"],
          [w * 0.64, h * 0.72, "#fff7e4", "#c4621d"],
          [w * 0.74, h * 0.56, "#f5c24c", "#6b3310"],
          [w * 0.83, h * 0.76, "#fff7e4", "#b85d19"],
          [w * 0.68, h * 0.30, "#f2b840", "#6b3310"],
        ];
        flowerSpots.forEach(([fx, fy, petalCol, centerCol]) => {
          for (let p = 0; p < 8; p++) {
            const ang = (p / 8) * Math.PI * 2;
            drawImpastoStroke(
              ctx,
              fx + Math.cos(ang) * 14,
              fy + Math.sin(ang) * 14,
              18,
              7,
              ang,
              petalCol,
              0.92
            );
          }
          ctx.beginPath();
          ctx.arc(fx, fy, 7, 0, Math.PI * 2);
          ctx.fillStyle = centerCol;
          ctx.fill();
        });
      } else if (kind === "river-stream") {
        const rng = makeRng(20202);
        const bg = ctx.createLinearGradient(0, 0, 0, h);
        bg.addColorStop(0, "#10254c");
        bg.addColorStop(0.38, "#264b78");
        bg.addColorStop(0.7, "#113647");
        bg.addColorStop(1, "#0b241f");
        ctx.fillStyle = bg;
        ctx.fillRect(0, 0, w, h);

        const horizonGlow = ctx.createRadialGradient(w * 0.52, h * 0.28, 10, w * 0.52, h * 0.28, w * 0.42);
        horizonGlow.addColorStop(0, "rgba(255, 198, 105, 0.78)");
        horizonGlow.addColorStop(0.5, "rgba(219, 132, 64, 0.35)");
        horizonGlow.addColorStop(1, "rgba(16, 37, 76, 0)");
        ctx.fillStyle = horizonGlow;
        ctx.fillRect(0, 0, w, h);

        const waterCols = ["#153a66", "#1d4e82", "#14453d", "#226354", "#0f2942", "#2e6399"];
        for (let i = 0; i < 130; i++) {
          const wy = h * 0.32 + rng() * (h * 0.68);
          drawImpastoStroke(
            ctx,
            rng() * w,
            wy,
            55 + rng() * 120,
            8 + rng() * 16,
            (rng() - 0.5) * 0.14,
            waterCols[Math.floor(rng() * waterCols.length)],
            0.72
          );
        }

        for (let s = 0; s < 26; s++) {
          const t = s / 25;
          const sx = w * 0.18 + t * (w * 0.48) + (rng() - 0.5) * 35;
          const sy = h * 0.88 - t * (h * 0.54);
          drawImpastoStroke(
            ctx,
            sx,
            sy,
            70 + (1 - t) * 90,
            6 + (1 - t) * 8,
            -0.36 + t * 0.15,
            s % 2 === 0 ? "#ffcf66" : "#ff9b3d",
            0.82
          );
        }

        const lanterns = [
          [w * 0.16, h * 0.48],
          [w * 0.22, h * 0.46],
          [w * 0.72, h * 0.44],
          [w * 0.78, h * 0.49],
          [w * 0.85, h * 0.45],
        ];
        lanterns.forEach(([lx, ly]) => {
          const lGrad = ctx.createRadialGradient(lx, ly, 2, lx, ly, 24);
          lGrad.addColorStop(0, "#fff8db");
          lGrad.addColorStop(0.4, "#ffae38");
          lGrad.addColorStop(1, "rgba(255, 140, 30, 0)");
          ctx.fillStyle = lGrad;
          ctx.beginPath();
          ctx.arc(lx, ly, 24, 0, Math.PI * 2);
          ctx.fill();

          for (let r = 0; r < 5; r++) {
            drawImpastoStroke(
              ctx,
              lx + (rng() - 0.5) * 8,
              ly + 24 + r * 11,
              22 - r * 2,
              4.5,
              0,
              "#ffbe4f",
              0.7 - r * 0.1
            );
          }
        });
      } else if (kind === "merkle-parchment") {
        const rng = makeRng(30303);
        ctx.fillStyle = "#f4ebd6";
        ctx.fillRect(0, 0, w, h);

        for (let i = 0; i < 65; i++) {
          drawImpastoStroke(
            ctx,
            rng() * w,
            rng() * h,
            90 + rng() * 130,
            24 + rng() * 32,
            (rng() - 0.5) * 0.4,
            i % 2 === 0 ? "#eadbc0" : "#f7f0df",
            0.5
          );
        }

        const midX = w * 0.5;
        const midY = h * 0.5;

        for (let l = 0; l < 11; l++) {
          const lx = w * 0.12 + l * 22;
          const ly = h * 0.14 + l * 9;
          drawImpastoStroke(ctx, lx, ly, 36, 9, -0.6 + (l % 2) * 1.1, "#4e6b3a", 0.88);
          if (l % 3 === 0) {
            ctx.beginPath();
            ctx.arc(lx + 6, ly + 8, 5.5, 0, Math.PI * 2);
            ctx.fillStyle = "#6e8743";
            ctx.fill();
          }
        }

        ctx.fillStyle = "rgba(196, 114, 74, 0.22)";
        ctx.fillRect(midX + 12, 14, midX - 26, midY - 22);
        ctx.save();
        ctx.strokeStyle = "#9e4d28";
        ctx.lineWidth = 14;
        ctx.beginPath();
        ctx.arc(w * 0.74, midY - 12, 58, Math.PI, 0, false);
        ctx.stroke();
        ctx.restore();

        ctx.fillStyle = "rgba(184, 122, 82, 0.24)";
        ctx.fillRect(24, midY + 14, midX - 38, midY - 26);
        ctx.fillStyle = "#7a4928";
        ctx.fillRect(w * 0.21, midY + 34, 54, 72);
        ctx.strokeStyle = "#f4ebd6";
        ctx.lineWidth = 2.5;
        ctx.strokeRect(w * 0.21 + 6, midY + 40, 42, 60);

        ctx.fillStyle = "rgba(104, 138, 102, 0.22)";
        ctx.fillRect(midX + 12, midY + 14, midX - 26, midY - 26);
        for (let f = 0; f < 16; f++) {
          drawImpastoStroke(
            ctx,
            midX + 40 + rng() * (midX - 80),
            midY + 30 + rng() * (midY - 55),
            34,
            11,
            (rng() - 0.5) * 1.2,
            f % 2 === 0 ? "#486e4c" : "#c79852",
            0.78
          );
        }

        ctx.strokeStyle = "#b88938";
        ctx.lineWidth = 2.2;
        ctx.strokeRect(14, 12, w - 28, h - 24);
        ctx.beginPath();
        ctx.moveTo(midX, 12);
        ctx.lineTo(midX, h - 12);
        ctx.moveTo(14, midY);
        ctx.lineTo(w - 14, midY);
        ctx.stroke();

        ctx.save();
        ctx.translate(midX, midY);
        ctx.rotate(Math.PI / 4);
        ctx.fillStyle = "#d99b26";
        ctx.fillRect(-10, -10, 20, 20);
        ctx.restore();

        const labels = [
          ["T0 · 8f43a1 ✓", 26, midY - 12],
          ["T1 · 6b96e4 ✓", midX + 22, midY - 12],
          ["T2 · df890c ✓", 26, h - 22],
          ["T3 · 2dc391 ✓", midX + 22, h - 22],
        ];
        ctx.font = "600 12px 'JetBrains Mono', monospace";
        labels.forEach(([txt, lx, ly]) => {
          ctx.fillStyle = "rgba(24, 22, 20, 0.78)";
          ctx.fillRect(lx - 6, ly - 14, 108, 20);
          ctx.fillStyle = "#f5f2eb";
          ctx.fillText(txt, lx, ly);
        });
      } else if (kind === "quantum-astrolabe") {
        const rng = makeRng(40404);
        const bg = ctx.createLinearGradient(0, 0, 0, h);
        bg.addColorStop(0, "#121624");
        bg.addColorStop(0.58, "#22263d");
        bg.addColorStop(0.82, "#785332");
        bg.addColorStop(1, "#cfa25a");
        ctx.fillStyle = bg;
        ctx.fillRect(0, 0, w, h);

        for (let i = 0; i < 110; i++) {
          const sy = rng() * h;
          const col =
            sy < h * 0.62
              ? i % 2 === 0
                ? "#1b2033"
                : "#2b314d"
              : i % 2 === 0
              ? "#c9954c"
              : "#e8c47e";
          drawImpastoStroke(
            ctx,
            rng() * w,
            sy,
            50 + rng() * 110,
            10 + rng() * 18,
            (rng() - 0.5) * 0.25,
            col,
            0.72
          );
        }

        ctx.fillStyle = "#ffe28a";
        for (let s = 0; s < 75; s++) {
          ctx.beginPath();
          ctx.arc(rng() * w, rng() * (h * 0.65), 1 + rng() * 2.2, 0, Math.PI * 2);
          ctx.fill();
        }

        const cx = w * 0.54;
        const cy = h * 0.48;
        ctx.save();
        ctx.strokeStyle = "rgba(255, 224, 148, 0.55)";
        ctx.lineWidth = 1.6;
        for (let k = 0; k < 12; k++) {
          const a = (k / 12) * Math.PI * 2;
          ctx.beginPath();
          ctx.moveTo(cx, cy);
          ctx.lineTo(cx + Math.cos(a) * 165, cy + Math.sin(a) * 125);
          ctx.stroke();
        }

        [108, 82, 54].forEach((rad, idx) => {
          ctx.beginPath();
          ctx.arc(cx, cy, rad, 0, Math.PI * 2);
          ctx.lineWidth = idx === 0 ? 9 : 4;
          ctx.strokeStyle = idx % 2 === 0 ? "#e6b44c" : "#fff0b8";
          ctx.stroke();
        });

        ctx.strokeStyle = "#ffd875";
        ctx.lineWidth = 5;
        ctx.beginPath();
        ctx.moveTo(cx - 70, cy + 45);
        ctx.lineTo(cx + 75, cy - 52);
        ctx.stroke();

        ctx.beginPath();
        ctx.arc(cx, cy, 10, 0, Math.PI * 2);
        ctx.fillStyle = "#ffcf5c";
        ctx.fill();
        ctx.restore();
      } else if (kind === "atelier-window") {
        const rng = makeRng(50505);
        ctx.fillStyle = "#7a684b";
        ctx.fillRect(0, 0, w, h);
        for (let i = 0; i < 80; i++) {
          drawImpastoStroke(
            ctx,
            rng() * w,
            rng() * h,
            60 + rng() * 100,
            16 + rng() * 24,
            (rng() - 0.5) * 0.3,
            i % 2 === 0 ? "#8a7554" : "#66563d",
            0.65
          );
        }

        const wx = w * 0.18;
        const wy = h * 0.1;
        const ww = w * 0.44;
        const wh = h * 0.68;

        const skyGrad = ctx.createLinearGradient(wx, wy, wx, wy + wh);
        skyGrad.addColorStop(0, "#fff2bf");
        skyGrad.addColorStop(0.55, "#e6c973");
        skyGrad.addColorStop(1, "#557a46");
        ctx.fillStyle = skyGrad;
        ctx.fillRect(wx, wy, ww, wh);

        drawImpastoStroke(ctx, wx + ww * 0.5, wy + wh * 0.72, ww * 0.95, 42, -0.08, "#829e4b", 0.9);
        drawImpastoStroke(ctx, wx + ww * 0.5, wy + wh * 0.86, ww * 0.95, 44, 0.06, "#446934", 0.92);
        [0.22, 0.31, 0.72].forEach((pos) => {
          drawImpastoStroke(
            ctx,
            wx + ww * pos,
            wy + wh * 0.58,
            14,
            58,
            0,
            "#1f3d22",
            0.92
          );
        });

        ctx.strokeStyle = "#59331b";
        ctx.lineWidth = 8;
        ctx.strokeRect(wx, wy, ww, wh);

        ctx.fillStyle = "#9c522b";
        ctx.fillRect(wx - 48, wy - 4, 44, wh + 8);
        ctx.fillRect(wx + ww + 4, wy - 4, 44, wh + 8);

        ctx.save();
        const beamGrad = ctx.createLinearGradient(wx + ww * 0.3, wy + wh * 0.3, w * 0.9, h);
        beamGrad.addColorStop(0, "rgba(255, 244, 190, 0.42)");
        beamGrad.addColorStop(1, "rgba(255, 244, 190, 0)");
        ctx.fillStyle = beamGrad;
        ctx.beginPath();
        ctx.moveTo(wx + 20, wy + wh);
        ctx.lineTo(wx + ww, wy + 20);
        ctx.lineTo(w, h);
        ctx.lineTo(wx + 80, h);
        ctx.closePath();
        ctx.fill();
        ctx.restore();
      }
    });
  }

  // 4C. Paint the 3 Workflow Setup Card Canvases (.wf-art-canvas)
  function paintWorkflowCanvases() {
    document.querySelectorAll(".wf-art-canvas").forEach((canvas) => {
      const ctx = canvas.getContext("2d");
      if (!ctx) return;
      const w = canvas.width;
      const h = canvas.height;
      const theme = canvas.getAttribute("data-art-theme");
      const rng = makeRng(theme === "twilight" ? 771 : theme === "horizon" ? 882 : 993);

      let cols = ["#17223b", "#263859", "#495d82", "#c78842", "#e8b968"];
      if (theme === "horizon") {
        cols = ["#4a2515", "#7d3e1e", "#b5632a", "#df9b44", "#f5d384"];
      } else if (theme === "meadow") {
        cols = ["#142e24", "#214d3a", "#367356", "#8ca64e", "#e0be63"];
      }

      const grad = ctx.createLinearGradient(0, 0, w, h);
      grad.addColorStop(0, cols[0]);
      grad.addColorStop(0.55, cols[2]);
      grad.addColorStop(1, cols[3]);
      ctx.fillStyle = grad;
      ctx.fillRect(0, 0, w, h);

      for (let i = 0; i < 85; i++) {
        drawImpastoStroke(
          ctx,
          rng() * w,
          rng() * h,
          45 + rng() * 95,
          12 + rng() * 22,
          -0.35 + rng() * 0.7,
          cols[Math.floor(rng() * cols.length)],
          0.65
        );
      }

      const halo = ctx.createRadialGradient(w * 0.5, h * 0.5, 8, w * 0.5, h * 0.5, h * 0.65);
      halo.addColorStop(0, "rgba(255, 226, 150, 0.34)");
      halo.addColorStop(1, "rgba(0, 0, 0, 0)");
      ctx.fillStyle = halo;
      ctx.fillRect(0, 0, w, h);
    });
  }

  // 4D. Paint the Full-Width Sunlit Horizon Landscape Banner (#horizon-brush-canvas)
  function paintHorizonBanner() {
    const canvas = document.getElementById("horizon-brush-canvas");
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const w = canvas.width;
    const h = canvas.height;
    const rng = makeRng(20261014);

    const sky = ctx.createLinearGradient(0, 0, 0, h);
    sky.addColorStop(0, "#b8c7cc");
    sky.addColorStop(0.35, "#ebd9b0");
    sky.addColorStop(0.54, "#fae8b4");
    sky.addColorStop(0.62, "#7d9956");
    sky.addColorStop(1, "#37522c");
    ctx.fillStyle = sky;
    ctx.fillRect(0, 0, w, h);

    const sunX = w * 0.62;
    const sunY = h * 0.28;
    for (let r = 0; r < 14; r++) {
      const ang = 0.35 + (r / 14) * 2.1;
      drawImpastoStroke(
        ctx,
        sunX + Math.cos(ang) * 180,
        sunY + Math.sin(ang) * 140,
        360,
        28,
        ang,
        "#fff3c4",
        0.34
      );
    }

    const cloudCols = ["#f5ebd1", "#e6d2a8", "#d4ba8c", "#fff9e8", "#c9b38f"];
    for (let c = 0; c < 130; c++) {
      drawImpastoStroke(
        ctx,
        rng() * w,
        rng() * (h * 0.5),
        70 + rng() * 140,
        18 + rng() * 32,
        -0.2 + rng() * 0.4,
        cloudCols[Math.floor(rng() * cloudCols.length)],
        0.72
      );
    }

    const hillCols = ["#466634", "#5e8040", "#7c9c4e", "#9cb058", "#c49a45", "#354f28", "#8f6832"];
    for (let m = 0; m < 180; m++) {
      const my = h * 0.5 + rng() * (h * 0.5);
      drawImpastoStroke(
        ctx,
        rng() * w,
        my,
        65 + rng() * 150,
        14 + rng() * 28,
        -0.18 + rng() * 0.36,
        hillCols[Math.floor(rng() * hillCols.length)],
        0.8
      );
    }
  }

  paintAtelierBackdrop(1);
  paintPieceCanvases();
  paintWorkflowCanvases();
  paintHorizonBanner();

  // ── 5. 3-Layer Hybrid Typographic Reveal Engine ──
  // Layer 1 (Monospace indices/tags/commands): SHA-256 Cryptographic Hex Scramble Decode
  // Layer 2 (Editorial Headings h2/h3): Clip-Path Split-Word Mask Rise
  // Layer 3 (Body descriptions & lists): Wet-Ink Blur-to-Crisp Word Stagger
  const typeRegistry = [];
  const HEX_GLYPHS = "0123456789ABCDEF//·:×+";

  function buildSplitWords(el, mode, baseStepMs) {
    const fullText = el.textContent.replace(/\s+/g, " ").trim();
    const words = fullText.split(" ");
    el.textContent = "";
    words.forEach((w, idx) => {
      if (mode === "editorial-rise") {
        const mask = document.createElement("span");
        mask.className = "word-mask";
        const inner = document.createElement("span");
        inner.className = "word-inner";
        inner.style.setProperty("--w-delay", `${idx * baseStepMs}ms`);
        inner.textContent = w;
        mask.appendChild(inner);
        el.appendChild(mask);
      } else {
        const span = document.createElement("span");
        span.className = "ink-word";
        span.style.setProperty("--w-delay", `${idx * baseStepMs}ms`);
        span.textContent = w;
        el.appendChild(span);
      }
      if (idx < words.length - 1) {
        el.appendChild(document.createTextNode(" "));
      }
    });
    return fullText;
  }

  function runCryptoHexDecode(el, fullText, durationMs = 460, delayMs = 0) {
    if (el._hexAnimId) cancelAnimationFrame(el._hexAnimId);
    const startTime = performance.now() + delayMs;
    const len = fullText.length;

    function step(now) {
      if (now < startTime) {
        el._hexAnimId = requestAnimationFrame(step);
        return;
      }
      const t = Math.min(1, (now - startTime) / durationMs);
      const ease = 1 - Math.pow(1 - t, 2.1);
      const lockedCount = Math.floor(ease * len);

      if (t >= 1) {
        el.textContent = fullText;
        el._hexAnimId = null;
        return;
      }

      let out = fullText.slice(0, lockedCount);
      const scrambleWindow = Math.min(len - lockedCount, 8);
      for (let i = 0; i < scrambleWindow; i++) {
        const origChar = fullText[lockedCount + i];
        if (origChar === " ") {
          out += " ";
        } else {
          const rIdx = Math.floor(Math.random() * HEX_GLYPHS.length);
          out += HEX_GLYPHS[rIdx];
        }
      }
      el.textContent = out;
      el._hexAnimId = requestAnimationFrame(step);
    }

    el._hexAnimId = requestAnimationFrame(step);
  }

  function initHybridTypographicSystem() {
    // 1. Prepare all [data-scroll-type] elements outside .atelier-card
    document.querySelectorAll("[data-scroll-type]").forEach((el) => {
      if (el.closest(".atelier-card")) return;

      const tag = el.tagName.toLowerCase();
      const isMono =
        el.classList.contains("flow-idx") ||
        el.classList.contains("ov-eyebrow");
      const isHeading =
        tag === "h1" ||
        tag === "h2" ||
        tag === "h3" ||
        el.classList.contains("flow-title") ||
        el.classList.contains("ov-h2") ||
        el.classList.contains("ov-h2-center");

      if (isMono) {
        const fullText = el.textContent.replace(/\s+/g, " ").trim();
        el.classList.add("typo-crypto-decode");
        typeRegistry.push({
          el,
          mode: "crypto-decode",
          fullText,
          started: false,
        });
      } else if (isHeading) {
        const fullText = buildSplitWords(el, "editorial-rise", 34);
        el.classList.add("typo-editorial-rise");
        typeRegistry.push({
          el,
          mode: "editorial-rise",
          fullText,
          started: false,
        });
      } else {
        const fullText = buildSplitWords(el, "wet-ink", 16);
        el.classList.add("typo-wet-ink");
        typeRegistry.push({
          el,
          mode: "wet-ink",
          fullText,
          started: false,
        });
      }
    });

    // 2. Prepare all 5 showcase cards (.atelier-card) with 3-Layer Hybrid Choreography
    document.querySelectorAll(".atelier-card").forEach((card) => {
      const tagEl = card.querySelector(".atelier-tag");
      const titleEl = card.querySelector(".atelier-title");
      const descEl = card.querySelector(".atelier-desc");
      const cmdCodeEl = card.querySelector(".atelier-cmd-bar code");

      if (tagEl && !tagEl.dataset.origText) {
        tagEl.dataset.origText = tagEl.textContent.replace(/\s+/g, " ").trim();
      }
      if (titleEl && !titleEl.dataset.splitReady) {
        buildSplitWords(titleEl, "editorial-rise", 42);
        titleEl.dataset.splitReady = "1";
      }
      if (descEl && !descEl.dataset.splitReady) {
        buildSplitWords(descEl, "wet-ink", 18);
        descEl.dataset.splitReady = "1";
      }
      if (cmdCodeEl && !cmdCodeEl.dataset.origText) {
        cmdCodeEl.dataset.origText = cmdCodeEl.textContent.replace(/\s+/g, " ").trim();
      }
    });
  }

  function triggerTypewriterItem(item, durationMs = 520, delayMs = 0) {
    if (!item || item.started) return;
    item.started = true;
    item.el.classList.add("is-typo-revealed");
    if (item.mode === "crypto-decode") {
      runCryptoHexDecode(item.el, item.fullText, 460, delayMs);
    }
  }

  initHybridTypographicSystem();

  // ── 6. Per-Pixel WebGL GLSL "Ink Bleed" (Mürekkep Yayılması) Engine (#ov-capabilities) ──
  // Why 2D Canvas polygons/splines fail vs. Real GPU Ink Bleed:
  // - Scaling a 2D polygon path out from a center point looks like an inflating vector blob.
  // - Real "Ink Bleed" (Mürekkep Yayılması) is a 2D spatial capillary percolation process:
  //   ink flows through a stationary porous fiber resistance field F(x, y) across the paper.
  // - Here, a single shared WebGL GLSL Fragment Shader evaluates 3-level Iterated Domain
  //   Warping (Inigo Quilez fBm(p + fBm(p + fBm(p)))) + high-frequency anisotropic paper
  //   fiber capillaries + fluid UV pigment advection + Beer-Lambert optical ink pooling
  //   (col * col) at every single pixel in parallel on the GPU.
  // - 1 single ink drop lands at a random point (dropX, dropY) on the pure #ffffff paper
  //   and percolates outward with branching capillary tendrils, islands, and feathery
  //   fibers until all 4 corners of the photograph are 100% revealed.
  const plateAnimState = new WeakMap();
  let sharedInkGl = null;

  function ensurePlateStainCanvas(card) {
    const wrap = card.querySelector(".atelier-art-wrap");
    if (!wrap) return null;
    let canvas = wrap.querySelector(".plate-stain-canvas");
    if (!canvas) {
      canvas = document.createElement("canvas");
      canvas.className = "plate-stain-canvas";
      canvas.setAttribute("aria-hidden", "true");
      wrap.appendChild(canvas);
    }
    return canvas;
  }

  // Initialize the shared WebGL Procedural Ink Bleed Mask Shader (Zero Texture Dependencies)
  function getSharedInkBleedGL() {
    if (sharedInkGl !== null) return sharedInkGl;

    const glCanvas = document.createElement("canvas");
    const gl =
      glCanvas.getContext("webgl", {
        alpha: true,
        antialias: false,
        depth: false,
        stencil: false,
        premultipliedAlpha: false,
        preserveDrawingBuffer: true,
      }) ||
      glCanvas.getContext("experimental-webgl", {
        alpha: true,
        antialias: false,
        depth: false,
        stencil: false,
        premultipliedAlpha: false,
        preserveDrawingBuffer: true,
      });

    if (!gl) {
      sharedInkGl = false;
      return false;
    }

    const vsSource = `
      attribute vec2 aPos;
      varying vec2 vUv;
      void main() {
        vUv = aPos * 0.5 + 0.5;
        gl_Position = vec4(aPos, 0.0, 1.0);
      }
    `;

    // Authentic Hele-Shaw capillary tendril ink bleed procedural fragment shader
    // (Matches reference images: viscous fingering, dendritic paper wicking, wet ink rim pooling)
    const fsSource = `
      precision highp float;
      varying vec2 vUv;

      uniform vec2 uResolution;
      uniform vec2 uDropCenter;
      uniform float uProgress;
      uniform float uAspect;
      uniform vec2 uSeed;

      float hash21(vec2 p) {
        p = fract(p * vec2(234.34, 435.345));
        p += dot(p, p + 34.23);
        return fract(p.x * p.y);
      }

      float vnoise(vec2 x) {
        vec2 i = floor(x);
        vec2 f = fract(x);
        vec2 u = f * f * (3.0 - 2.0 * f);
        float a = hash21(i);
        float b = hash21(i + vec2(1.0, 0.0));
        float c = hash21(i + vec2(0.0, 1.0));
        float d = hash21(i + vec2(1.0, 1.0));
        return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
      }

      float fbm(vec2 p) {
        float v = 0.0;
        float a = 0.5;
        mat2 rot = mat2(0.80, 0.60, -0.60, 0.80);
        for (int i = 0; i < 5; i++) {
          v += a * vnoise(p);
          p = rot * p * 2.04 + vec2(17.1, 31.7);
          a *= 0.5;
        }
        return v;
      }

      float ridgedFbm(vec2 p) {
        float v = 0.0;
        float a = 0.5;
        float weight = 1.0;
        mat2 rot = mat2(0.80, -0.60, 0.60, 0.80);
        for (int i = 0; i < 4; i++) {
          float n = 1.0 - abs(vnoise(p) * 2.0 - 1.0);
          n = n * n * weight;
          weight = clamp(n * 1.6, 0.2, 1.0);
          v += a * n;
          p = rot * p * 2.18 + vec2(11.4, 29.3);
          a *= 0.5;
        }
        return v;
      }

      void main() {
        vec2 uv = gl_FragCoord.xy / uResolution.xy;
        vec2 delta = uv - uDropCenter;
        delta.x *= uAspect;
        float dist = length(delta);
        float angle = atan(delta.y, delta.x);

        float t = clamp(uProgress, 0.0, 1.0);
        if (t >= 0.999) {
          gl_FragColor = vec4(0.0, 0.0, 0.0, 0.0);
          return;
        }
        if (t <= 0.001) {
          gl_FragColor = vec4(1.0, 1.0, 1.0, 1.0);
          return;
        }

        // Fluid expansion through paper fibers ("kağıt emer ya büyür")
        float spread = pow(t, 0.72);
        float maxReach = 1.65;
        float curR = spread * maxReach;

        // Capillary fingers shooting outward along fiber paths
        float angF1 = sin(angle * 7.0 + uSeed.x * 2.0) * 0.38;
        float angF2 = cos(angle * 19.0 + uSeed.y * 3.0) * 0.22;
        float angF3 = sin(angle * 41.0 + uSeed.x * 5.0) * 0.12;
        float radialFingers = angF1 + angF2 + angF3;

        // Dendritic veins inside the paper pores (Hele-Shaw viscous fingering)
        vec2 warp = vec2(fbm(uv * 4.5 + uSeed), fbm(uv * 4.5 + vec2(8.1, 3.7) + uSeed));
        float veins = ridgedFbm(uv * 9.5 + warp * 2.4 + uSeed);

        // Micro-granulation of paper tooth
        float microTooth = vnoise(uv * 85.0 + uSeed * 3.5);

        // Capillary wicking resistance
        float tendrilStrength = smoothstep(0.01, 0.25, spread) * (0.34 + 0.26 * spread);
        float effRadius = curR + radialFingers * tendrilStrength + (veins - 0.5) * (0.26 * tendrilStrength) + (microTooth - 0.5) * 0.02;

        float diff = dist - effRadius;

        // ZONES:
        // 1. Fully soaked paper interior: transparent to reveal photograph underneath
        if (diff < -0.055) {
          gl_FragColor = vec4(0.0, 0.0, 0.0, 0.0);
          return;
        }

        // 2. Wet dendritic capillary ink rim (Sumi-e ink pooling + capillary fingers)
        if (diff < 0.016) {
          float inkAlpha = smoothstep(-0.055, -0.015, diff) * (1.0 - smoothstep(0.006, 0.016, diff));
          vec3 inkCol = vec3(0.07, 0.06, 0.05) + (microTooth - 0.5) * 0.03;
          float paperMix = smoothstep(0.006, 0.016, diff);
          vec3 col = mix(inkCol, vec3(1.0), paperMix);
          float alpha = mix(inkAlpha * 0.95, 1.0, paperMix);
          gl_FragColor = vec4(col, alpha);
          return;
        }

        // 3. Capillary water halo advancing into dry paper
        if (diff < 0.065) {
          float haloT = smoothstep(0.016, 0.065, diff);
          vec3 haloCol = mix(vec3(0.88, 0.85, 0.82), vec3(1.0), haloT);
          gl_FragColor = vec4(haloCol, 1.0);
          return;
        }

        // 4. Dry white paper
        gl_FragColor = vec4(1.0, 1.0, 1.0, 1.0);
      }
    `;

    function compileShader(type, source) {
      const s = gl.createShader(type);
      gl.shaderSource(s, source);
      gl.compileShader(s);
      if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) {
        gl.deleteShader(s);
        return null;
      }
      return s;
    }

    const vs = compileShader(gl.VERTEX_SHADER, vsSource);
    const fs = compileShader(gl.FRAGMENT_SHADER, fsSource);
    if (!vs || !fs) {
      sharedInkGl = false;
      return false;
    }

    const prog = gl.createProgram();
    gl.attachShader(prog, vs);
    gl.attachShader(prog, fs);
    gl.linkProgram(prog);
    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
      sharedInkGl = false;
      return false;
    }

    gl.useProgram(prog);

    const quadBuf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, quadBuf);
    gl.bufferData(
      gl.ARRAY_BUFFER,
      new Float32Array([-1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1]),
      gl.STATIC_DRAW
    );

    const aPos = gl.getAttribLocation(prog, "aPos");
    gl.enableVertexAttribArray(aPos);
    gl.vertexAttribPointer(aPos, 2, gl.FLOAT, false, 0, 0);

    sharedInkGl = {
      glCanvas,
      gl,
      prog,
      uResolution: gl.getUniformLocation(prog, "uResolution"),
      uDropCenter: gl.getUniformLocation(prog, "uDropCenter"),
      uProgress: gl.getUniformLocation(prog, "uProgress"),
      uAspect: gl.getUniformLocation(prog, "uAspect"),
      uSeed: gl.getUniformLocation(prog, "uSeed"),
    };
    return sharedInkGl;
  }

  // 2D Procedural Ink Bleed Fallback (Smooth Dendritic Capillary Dissolve)
  function render2DInkBleed(ctx, W, H, dpr, cw, ch, dropX, dropY, rawT, seedX, seedY) {
    ctx.save();
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, cw, ch);

    if (rawT >= 0.999) {
      ctx.restore();
      return;
    }

    // Paint solid white paper
    ctx.fillStyle = "#ffffff";
    ctx.fillRect(0, 0, cw, ch);

    if (rawT <= 0.001) {
      ctx.restore();
      return;
    }

    const maxCornerPx = Math.hypot(
      Math.max(dropX, cw - dropX),
      Math.max(dropY, ch - dropY)
    );
    const spread = Math.pow(rawT, 0.72);
    const curR = spread * maxCornerPx * 1.35;

    // 1. Cut out the dendritic ink stain using destination-out to reveal the photo underneath
    ctx.globalCompositeOperation = "destination-out";

    const numPts = 120;
    const points = [];
    for (let i = 0; i < numPts; i++) {
      const angle = (i / numPts) * Math.PI * 2;
      const f1 = Math.sin(angle * 7 + seedX) * 0.28;
      const f2 = Math.cos(angle * 19 + seedY) * 0.18;
      const f3 = Math.sin(angle * 41 + seedX * 1.7) * 0.10;
      const whisker = Math.sin(angle * 83 + seedY * 2.3) * 0.04;
      const tendril = (f1 + f2 + f3 + whisker) * Math.min(1, spread * 1.5);
      const r = Math.max(0, curR * (1 + tendril));
      points.push({
        x: dropX + Math.cos(angle) * r,
        y: dropY + Math.sin(angle) * r,
      });
    }

    ctx.beginPath();
    ctx.moveTo(points[0].x, points[0].y);
    for (let i = 1; i < points.length; i++) {
      const prev = points[i - 1];
      const curr = points[i];
      const mx = (prev.x + curr.x) * 0.5;
      const my = (prev.y + curr.y) * 0.5;
      ctx.quadraticCurveTo(prev.x, prev.y, mx, my);
    }
    ctx.closePath();
    ctx.fillStyle = "#000000";
    ctx.fill();

    // 2. Stroke dark Sumi-e pooled ink rim along the drying edge
    ctx.globalCompositeOperation = "source-over";
    ctx.lineWidth = Math.max(1.5, 4.5 * (1 - rawT * 0.6));
    ctx.strokeStyle = "rgba(24, 22, 20, 0.88)";
    ctx.stroke();

    // 3. Faint warm water halo
    ctx.lineWidth = Math.max(4, 12 * (1 - rawT * 0.5));
    ctx.strokeStyle = "rgba(180, 168, 152, 0.26)";
    ctx.stroke();

    ctx.restore();
  }

  function clearCanvasToPureWhite(card) {
    const canvas = ensurePlateStainCanvas(card);
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const cw = Math.max(320, Math.round(rect.width || 820));
    const ch = Math.max(180, Math.round(rect.height || 461));
    const dpr = Math.min(window.devicePixelRatio || 1, riseInkCanvasCap());
    const W = Math.round(cw * dpr);
    const H = Math.round(ch * dpr);
    if (canvas.width !== W || canvas.height !== H) {
      canvas.width = W;
      canvas.height = H;
    }
    const ctx = canvas.getContext("2d");
    if (ctx) {
      ctx.save();
      ctx.setTransform(1, 0, 0, 1, 0, 0);
      ctx.fillStyle = "#ffffff";
      ctx.fillRect(0, 0, W, H);
      ctx.restore();
    }
  }

  function stopCardAnimation(card) {
    const prev = plateAnimState.get(card);
    if (prev && prev.rafId) {
      cancelAnimationFrame(prev.rafId);
      prev.rafId = null;
    }
  }

  function ensureTextInkCanvas(card) {
    let canvas = card.querySelector(".text-ink-stain-canvas");
    if (!canvas) {
      canvas = document.createElement("canvas");
      canvas.className = "text-ink-stain-canvas";
      canvas.setAttribute("aria-hidden", "true");
      card.prepend(canvas);
    }
    return canvas;
  }

  // Authentic Jagged & Irregular ("Tırtıklı ve Düzensiz") Sumi-e Dendritic Ink Bleed covering both text and photo background
  function renderTextBlackInkBleed(ctx, W, H, dpr, cw, ch, dropX, dropY, rawT, seedX, seedY, cardIdx) {
    if (!ctx) return;
    ctx.save();
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, cw, ch);

    if (rawT <= 0.001) {
      ctx.restore();
      return;
    }

    const spread = Math.pow(Math.min(1, rawT), 0.62);

    // Safety margin from canvas border: ink must NEVER touch within 20px of any canvas edge
    const edgeMargin = 20;

    const cardCenterX = cw * 0.50;
    const cardCenterY = ch * 0.50;

    // As ink bleeds from the drop point in the text area, the fluid center shifts toward the card center to envelop photo background too
    const curOriginX = dropX + (cardCenterX - dropX) * Math.min(1, spread * 0.72);
    const curOriginY = dropY + (cardCenterY - dropY) * Math.min(1, spread * 0.72);

    // Generous base reaches to ensure both text and photo background are completely enveloped
    const contentW = cw - 180;
    const contentH = ch - 180;
    const baseW = contentW * 0.54;
    const baseH = contentH * 0.56;

    // 260 perimeter points for high-density jagged capillary detail ("tırtıklar");
    // phones and lite devices trace at 150 (the per-frame math is O(points))
    const numPts = riseInkCanvasCap() < 1.75 ? 150 : 260;
    const points = [];

    // Helper: shortest angular distance
    function angleDiff(a, b) {
      let d = (a - b) % (Math.PI * 2);
      if (d < -Math.PI) d += Math.PI * 2;
      if (d > Math.PI) d -= Math.PI * 2;
      return d;
    }

    // 7 Asymmetric, wild dendritic branch tongues across the whole composition
    const branches = [
      { angle: 0.20 + (seedX * 0.11) % 0.6, reach: 0.36, width: 0.40 }, // Right of photo
      { angle: 1.10 + (seedY * 0.13) % 0.6, reach: 0.40, width: 0.46 }, // Bottom-right of photo
      { angle: 1.70 + (seedX * 0.09) % 0.6, reach: 0.32, width: 0.42 }, // Bottom-middle
      { angle: 2.70 + (seedY * 0.15) % 0.6, reach: 0.38, width: 0.44 }, // Bottom-left of text
      { angle: 3.75 + (seedX * 0.17) % 0.6, reach: 0.42, width: 0.48 }, // Top-left of text
      { angle: 4.85 + (seedY * 0.12) % 0.6, reach: 0.35, width: 0.40 }, // Top-middle
      { angle: 5.75 + (seedX * 0.14) % 0.6, reach: 0.44, width: 0.50 }  // Top-right of photo
    ];

    for (let i = 0; i < numPts; i++) {
      const angle = (i / numPts) * Math.PI * 2;
      const cosA = Math.cos(angle);
      const sinA = Math.sin(angle);

      // Superelliptical base reach (p=2.6) ensuring solid envelope of text + photo
      const p = 2.6;
      const cosPow = Math.pow(Math.abs(cosA), p);
      const sinPow = Math.pow(Math.abs(sinA), p);
      const baseReach = Math.pow(cosPow * Math.pow(baseW, p) + sinPow * Math.pow(baseH, p), 1 / p);

      // 1. DÜZENSİZLİK (Irregular asymmetric branch tongues):
      let branchReach = 0;
      for (let b = 0; b < branches.length; b++) {
        const d = angleDiff(angle, branches[b].angle);
        branchReach += branches[b].reach * Math.exp(-(d * d) / (2 * branches[b].width * branches[b].width));
      }

      // 2. TIRTIKLAR (Sharp, jagged needle-like capillary teeth):
      const tooth1 = (1.0 - 2.0 * Math.abs(Math.sin(angle * 16.0 + seedX * 2.1))) * 0.09;
      const tooth2 = (1.0 - 2.0 * Math.abs(Math.cos(angle * 37.0 + seedY * 1.7))) * 0.06;
      const tooth3 = (1.0 - 2.0 * Math.abs(Math.sin(angle * 79.0 + seedX * 3.4))) * 0.035;

      // Micro fibrous stochastic jitter per point
      const microJitter = ((Math.sin(i * 97.7 + seedY * 19.3) * 43758.5453) % 1) * 0.03;

      const totalMod = 1.0 + (branchReach + tooth1 + tooth2 + tooth3 + microJitter) * Math.min(1, spread * 1.3);
      const targetR = baseReach * totalMod * spread;

      // Mathematical boundary guarantee: strictly limit radius so points NEVER reach canvas edge
      const toEdgeX = cosA > 0.0001 ? (cw - edgeMargin - curOriginX) / cosA : cosA < -0.0001 ? (edgeMargin - curOriginX) / cosA : 9999;
      const toEdgeY = sinA > 0.0001 ? (ch - edgeMargin - curOriginY) / sinA : sinA < -0.0001 ? (edgeMargin - curOriginY) / sinA : 9999;
      const maxSafeR = Math.max(10, Math.min(toEdgeX, toEdgeY) * 0.94);

      const r = Math.min(targetR, maxSafeR);
      points.push({
        x: curOriginX + cosA * r,
        y: curOriginY + sinA * r,
        r: r,
        angle: angle,
        cosA: cosA,
        sinA: sinA
      });
    }

    // Helper: draw jagged path directly with lineTo (preserves crisp serrations!)
    function traceJaggedPath(pts, scale) {
      ctx.beginPath();
      const n = pts.length;
      const startX = curOriginX + (pts[0].x - curOriginX) * scale;
      const startY = curOriginY + (pts[0].y - curOriginY) * scale;
      ctx.moveTo(startX, startY);

      for (let i = 1; i < n; i++) {
        const px = curOriginX + (pts[i].x - curOriginX) * scale;
        const py = curOriginY + (pts[i].y - curOriginY) * scale;
        ctx.lineTo(px, py);
      }
      ctx.closePath();
    }

    // A. Outer thin capillary water halo
    traceJaggedPath(points, 1.022);
    ctx.fillStyle = "rgba(18, 17, 16, 0.12)";
    ctx.fill();

    // B. Deep velvety carbon black core (Sumi-e ink) with crisp jagged teeth
    traceJaggedPath(points, 1.0);
    ctx.fillStyle = "#100f0e";
    ctx.fill();

    // C. Micro-capillary fibrous drying rim
    ctx.lineWidth = 1.0;
    ctx.strokeStyle = "rgba(10, 9, 8, 0.90)";
    ctx.stroke();

    // D. Branching dendritic tendrils shooting outward from finger peaks (viscous fingering)
    if (spread > 0.15) {
      ctx.strokeStyle = "#100f0e";
      ctx.lineCap = "round";
      ctx.lineJoin = "round";

      for (let b = 0; b < branches.length; b++) {
        const bAngle = branches[b].angle;
        for (let t = -1; t <= 1; t++) {
          const tAngle = bAngle + t * 0.18 + Math.sin(seedX + b * 7 + t) * 0.08;
          const ptIdx = Math.floor(((tAngle % (Math.PI * 2) + Math.PI * 2) % (Math.PI * 2)) / (Math.PI * 2) * numPts);
          const rootPt = points[ptIdx % numPts];

          const tendrilReach = (18 + (b * 11) % 24 + Math.abs(t) * 12) * spread;
          const cosT = Math.cos(tAngle + t * 0.22);
          const sinT = Math.sin(tAngle + t * 0.22);

          const endX = rootPt.x + cosT * tendrilReach;
          const endY = rootPt.y + sinT * tendrilReach;

          if (endX >= edgeMargin && endX <= cw - edgeMargin && endY >= edgeMargin && endY <= ch - edgeMargin) {
            const midX = rootPt.x + cosT * tendrilReach * 0.5 + sinT * (t * 7);
            const midY = rootPt.y + sinT * tendrilReach * 0.5 - cosT * (t * 7);

            ctx.lineWidth = Math.max(0.8, 2.6 * (1 - Math.abs(t) * 0.3) * (1 - (1 - spread) * 0.4));
            ctx.beginPath();
            ctx.moveTo(rootPt.x, rootPt.y);
            ctx.quadraticCurveTo(midX, midY, endX, endY);
            ctx.stroke();
          }
        }
      }
    }

    // E. Scattered "Dağınık, Rastgele" Satellite Droplets and Spatters
    const numDroplets = 32;
    for (let k = 0; k < numDroplets; k++) {
      const rand1 = Math.sin(seedX * 13.7 + k * 91.3) * 0.5 + 0.5;
      const rand2 = Math.cos(seedY * 17.1 + k * 53.7) * 0.5 + 0.5;
      const rand3 = Math.sin((seedX + seedY) * 7.9 + k * 31.1) * 0.5 + 0.5;

      const dropAngle = (k / numDroplets) * Math.PI * 2 + (rand1 - 0.5) * 0.55;
      const cosD = Math.cos(dropAngle);
      const sinD = Math.sin(dropAngle);

      const ptIdx = Math.floor(((dropAngle % (Math.PI * 2) + Math.PI * 2) % (Math.PI * 2)) / (Math.PI * 2) * numPts);
      const localR = points[ptIdx % numPts].r;

      const distOffset = 6 + rand2 * 36;
      const totalDist = localR + distOffset;

      const dX = curOriginX + cosD * totalDist;
      const dY = curOriginY + sinD * totalDist;
      if (dX < edgeMargin || dX > cw - edgeMargin || dY < edgeMargin || dY > ch - edgeMargin) {
        continue;
      }

      const triggerT = 0.20 + rand3 * 0.55;
      if (rawT > triggerT) {
        const dProgress = Math.min(1, (rawT - triggerT) / 0.18);
        const baseRadius = 1.3 + rand1 * 4.8;
        const curRadius = baseRadius * Math.pow(dProgress, 0.5);

        ctx.beginPath();
        ctx.arc(dX, dY, curRadius, 0, Math.PI * 2);
        ctx.fillStyle = rand2 > 0.75 ? "rgba(18, 17, 16, 0.80)" : "#100f0e";
        ctx.fill();

        if (rand1 > 0.55 && dProgress > 0.75) {
          const microX = dX + cosD * (curRadius + 3.5);
          const microY = dY + sinD * (curRadius + 3.5);
          if (microX >= edgeMargin && microX <= cw - edgeMargin && microY >= edgeMargin && microY <= ch - edgeMargin) {
            ctx.beginPath();
            ctx.arc(microX, microY, 1.1, 0, Math.PI * 2);
            ctx.fillStyle = "rgba(18, 17, 16, 0.70)";
            ctx.fill();
          }
        }
      }
    }

    ctx.restore();
  }

  function resetPlateToWhitePaper(card) {
    stopCardAnimation(card);
    card.classList.remove("is-photo-done");
    clearCanvasToPureWhite(card);
    const textCanvas = card.querySelector(".text-ink-stain-canvas");
    if (textCanvas) {
      const tctx = textCanvas.getContext("2d");
      if (tctx) tctx.clearRect(0, 0, textCanvas.width, textCanvas.height);
    }
    const cardBody = card.querySelector(".atelier-card-body");
    if (cardBody) {
      cardBody.style.setProperty("--ink-text-reveal", "0");
      cardBody.classList.remove("is-ink-stage-1", "is-ink-stage-2", "is-ink-stage-3");
      cardBody.style.webkitMaskImage = "none";
      cardBody.style.maskImage = "none";
    }
    const titleEl = card.querySelector(".atelier-title");
    const descEl = card.querySelector(".atelier-desc");
    if (titleEl) titleEl.classList.remove("is-typo-revealed");
    if (descEl) descEl.classList.remove("is-typo-revealed");
  }

  // Run the authentic Single-Drop "Ink Bleed" (Mürekkep Yayılması) reveal for a showcase card
  function runSingleDropInkBleedReveal(card, idx, customCoords = null) {
    stopCardAnimation(card);
    card.classList.remove("is-photo-done");

    const canvas = ensurePlateStainCanvas(card);
    const textCanvas = ensureTextInkCanvas(card);
    const cardBody = card.querySelector(".atelier-card-body");
    if (!canvas || !textCanvas) return;

    const dpr = Math.min(window.devicePixelRatio || 1, riseInkCanvasCap());

    // Photo canvas sizing
    const rect = canvas.getBoundingClientRect();
    const cw = Math.max(320, Math.round(rect.width || 820));
    const ch = Math.max(180, Math.round(rect.height || 461));
    const W = Math.round(cw * dpr);
    const H = Math.round(ch * dpr);
    if (canvas.width !== W || canvas.height !== H) {
      canvas.width = W;
      canvas.height = H;
    }
    const ctx = canvas.getContext("2d");

    // Text and photo background ink canvas sizing: accurately measure card bounds + 180px margin
    const cRect = card.getBoundingClientRect();
    const textCw = Math.max(900, Math.round(cRect.width + 180));
    const textCh = Math.max(480, Math.round(cRect.height + 180));
    const textW = Math.round(textCw * dpr);
    const textH = Math.round(textCh * dpr);
    if (textCanvas.width !== textW || textCanvas.height !== textH) {
      textCanvas.width = textW;
      textCanvas.height = textH;
    }
    const tctx = textCanvas.getContext("2d");

    // RANDOM DROP POINT directly inside the text area ("yazıların bulunduğu rastgele bir nokta üzerinden olsun")
    // Text area is roughly the left 42% of the card
    const bRect = cardBody ? cardBody.getBoundingClientRect() : { width: 440, height: 340 };
    const innerTextW = Math.max(260, Math.round(bRect.width));
    const innerTextH = Math.max(220, Math.round(bRect.height));
    const textDropX =
      customCoords && typeof customCoords.x === "number"
        ? customCoords.x + 90
        : 90 + innerTextW * (0.30 + ((idx * 37) % 40) * 0.01);
    const textDropY =
      customCoords && typeof customCoords.y === "number"
        ? customCoords.y + 90
        : 90 + innerTextH * (0.32 + ((idx * 43) % 36) * 0.01);

    // Photo drop coordinates
    const photoDropX = cw * (0.35 + ((idx * 29) % 30) * 0.01);
    const photoDropY = ch * (0.38 + ((idx * 41) % 24) * 0.01);
    const uDropX = photoDropX / cw;
    const uDropY = photoDropY / ch;
    const aspect = cw / Math.max(1, ch);

    const seedX = 11.3 + (idx + 1) * 19.1;
    const seedY = 27.9 + (idx + 1) * 31.7;

    const inkGl = getSharedInkBleedGL();
    const totalDuration = 1600; // ms
    const startNow = performance.now();

    let stage1Done = false;
    let stage2Done = false;
    let stage3Done = false;
    const tagEl = card.querySelector(".atelier-tag");
    const titleEl = card.querySelector(".atelier-title");
    const descEl = card.querySelector(".atelier-desc");
    const cmdCodeEl = card.querySelector(".atelier-cmd-bar code");

    function frame(now) {
      try {
        const elapsed = now - startNow;
        const rawT = Math.min(1, elapsed / totalDuration);

        // A. Render Black Ink Bleed behind the text
        if (tctx) {
          renderTextBlackInkBleed(tctx, textW, textH, dpr, textCw, textCh, textDropX, textDropY, rawT, seedX, seedY, idx);
        }

        // B. 3-Layer Hybrid Typographic Choreography + Radial Ink-Front Mask
        //    Clips the entire cardBody to the expanding black ink radius so no text or button
        //    can EVER appear before the black ink has physically painted the paper underneath it!
        if (cardBody) {
          const textAlpha = Math.min(1, Math.max(0, (rawT - 0.08) * 1.9));
          cardBody.style.setProperty("--ink-text-reveal", textAlpha.toFixed(3));

          const localDropX = textDropX - 90;
          const localDropY = textDropY - 90;
          const maxBodyReach = Math.hypot(innerTextW, innerTextH) * 1.15;
          const inkFrontR = Math.pow(rawT, 0.76) * maxBodyReach;
          if (rawT >= 0.94) {
            cardBody.style.webkitMaskImage = "none";
            cardBody.style.maskImage = "none";
          } else {
            const featherR = Math.max(18, inkFrontR);
            const coreR = Math.max(0, featherR - 36);
            const maskGrad = `radial-gradient(circle at ${localDropX.toFixed(0)}px ${localDropY.toFixed(0)}px, #000000 ${coreR.toFixed(0)}px, rgba(0,0,0,0.45) ${((coreR + featherR) * 0.5).toFixed(0)}px, transparent ${featherR.toFixed(0)}px)`;
            cardBody.style.webkitMaskImage = maskGrad;
            cardBody.style.maskImage = maskGrad;
          }

          // Stage 1 (rawT >= 0.14): Monospace tag cryptographic hex decode + seal badge pop
          if (rawT >= 0.14 && !stage1Done) {
            stage1Done = true;
            cardBody.classList.add("is-ink-stage-1");
            if (tagEl && tagEl.dataset.origText) {
              runCryptoHexDecode(tagEl, tagEl.dataset.origText, 440, 0);
            }
          }

          // Stage 2 (rawT >= 0.24): Editorial Split-Word Mask Rise on Serif Title + Wet-Ink Focus on Description
          if (rawT >= 0.24 && !stage2Done) {
            stage2Done = true;
            cardBody.classList.add("is-ink-stage-2");
            if (titleEl) titleEl.classList.add("is-typo-revealed");
            if (descEl) descEl.classList.add("is-typo-revealed");
          }

          // Stage 3 (rawT >= 0.58): CLI Command Bar + Button reveal ONLY after black ink reaches the bottom bar
          if (rawT >= 0.58 && !stage3Done) {
            stage3Done = true;
            cardBody.classList.add("is-ink-stage-3");
            if (cmdCodeEl && cmdCodeEl.dataset.origText) {
              runCryptoHexDecode(cmdCodeEl, cmdCodeEl.dataset.origText, 480, 20);
            }
          }
        }

        // C. Render Photo ink bleed reveal
        if (ctx) {
          let glRendered = false;
          if (inkGl) {
            try {
              const { glCanvas, gl, prog } = inkGl;
              if (glCanvas.width !== W || glCanvas.height !== H) {
                glCanvas.width = W;
                glCanvas.height = H;
              }
              gl.viewport(0, 0, W, H);
              gl.useProgram(prog);

              gl.uniform2f(inkGl.uResolution, W, H);
              gl.uniform2f(inkGl.uDropCenter, uDropX, uDropY);
              gl.uniform1f(inkGl.uProgress, rawT);
              gl.uniform1f(inkGl.uAspect, aspect);
              gl.uniform2f(inkGl.uSeed, seedX, seedY);

              gl.drawArrays(gl.TRIANGLES, 0, 6);

              ctx.save();
              ctx.setTransform(1, 0, 0, 1, 0, 0);
              ctx.clearRect(0, 0, W, H);
              ctx.drawImage(glCanvas, 0, 0, W, H);
              ctx.restore();
              glRendered = true;
            } catch (_) {
              glRendered = false;
            }
          }

          if (!glRendered) {
            render2DInkBleed(ctx, W, H, dpr, cw, ch, photoDropX, photoDropY, rawT, seedX, seedY);
          }
        }

        if (rawT < 1) {
          const st = plateAnimState.get(card) || {};
          st.rafId = requestAnimationFrame(frame);
          plateAnimState.set(card, st);
        } else {
          if (ctx) {
            ctx.save();
            ctx.setTransform(1, 0, 0, 1, 0, 0);
            ctx.clearRect(0, 0, W, H);
            ctx.restore();
          }
          if (cardBody) {
            cardBody.style.setProperty("--ink-text-reveal", "1");
            cardBody.style.webkitMaskImage = "none";
            cardBody.style.maskImage = "none";
          }
          card.classList.add("is-photo-done");
        }
      } catch (err) {
        if (ctx) ctx.clearRect(0, 0, W, H);
        if (cardBody) {
          cardBody.style.setProperty("--ink-text-reveal", "1");
          cardBody.style.webkitMaskImage = "none";
          cardBody.style.maskImage = "none";
        }
        card.classList.add("is-photo-done");
      }
    }

    clearCanvasToPureWhite(card);
    if (tctx) tctx.clearRect(0, 0, textW, textH);
    if (risePerfLiteOn()) {
      // Lite devices skip the 1600ms repaint loop and paint the final revealed
      // state in a single pass (rawT=1 also takes frame()'s finalize branch)
      frame(startNow + totalDuration + 50);
      return;
    }
    const st = { rafId: requestAnimationFrame(frame) };
    plateAnimState.set(card, st);
  }

  // ── 6b. Pure-White Scroll-Driven 5-Photo Sequence Controller ──
  function initPureWhiteScrollShowcase() {
    const capStage = document.getElementById("ov-capabilities");
    const cards = Array.from(document.querySelectorAll(".atelier-card[data-stack-card]"));
    const progressLabel = document.getElementById("stack-progress-label");
    const wfCards = Array.from(document.querySelectorAll(".wf-card"));
    // Entrance write cache: once a setup card's eased progress settles, its
    // per-frame opacity/transform writes in section C are skipped entirely
    const wfLastEase = new Array(wfCards.length).fill(-1);

    if (!cards.length) return;

    let currentIdx = -1;

    // Doc-space travel cache: stage progress is measured against frozen document
    // geometry captured on section entry, so mobile URL-bar innerHeight changes
    // (which resize the vh-based runway height) can never rewind stage progress
    const travelCache = { ready: false, vh: 0, topDoc: 0, travel: 1 };

    // Initialize all cards on white paper; clicking text or photo drops an ink droplet right there
    cards.forEach((c, idx) => {
      resetPlateToWhitePaper(c);
      const wrap = c.querySelector(".atelier-art-wrap");
      if (wrap) {
        wrap.setAttribute("aria-label", "Click anywhere on the paper to drop an ink bleed droplet");
        wrap.addEventListener("click", (e) => {
          const r = wrap.getBoundingClientRect();
          const clickX = Math.max(24, Math.min(r.width - 24, e.clientX - r.left));
          const clickY = Math.max(24, Math.min(r.height - 24, e.clientY - r.top));
          runSingleDropInkBleedReveal(c, idx, { x: clickX, y: clickY });
        });
      }
      const cardBody = c.querySelector(".atelier-card-body");
      if (cardBody) {
        cardBody.setAttribute("aria-label", "Click anywhere on the text to drop an ink droplet");
        cardBody.addEventListener("click", (e) => {
          const r = cardBody.getBoundingClientRect();
          const clickX = Math.max(16, Math.min(r.width - 16, e.clientX - r.left));
          const clickY = Math.max(16, Math.min(r.height - 16, e.clientY - r.top));
          runSingleDropInkBleedReveal(c, idx, { x: clickX, y: clickY });
        });
      }
    });
    cards[0].classList.add("is-viewer-active", "is-active-piece");

    let lastSwitchTime = 0;
    let queuedTargetIdx = null;
    let dwellTimer = null;
    const MIN_CARD_DWELL_MS = 600; // Balanced dwell time: never skips cards, never feels stuck

    function commitCardSwitch(stepIdx) {
      const n = cards.length;
      const targetIdx = Math.max(0, Math.min(n - 1, stepIdx));
      if (targetIdx === currentIdx) return;
      currentIdx = targetIdx;
      lastSwitchTime = performance.now();

      cards.forEach((card, i) => {
        const isOn = i === currentIdx;
        card.classList.toggle("is-viewer-active", isOn);
        card.classList.toggle("is-active-piece", isOn);
        if (isOn) {
          runSingleDropInkBleedReveal(card, i);
        } else {
          stopCardAnimation(card);
          resetPlateToWhitePaper(card);
        }
      });

      if (capStage) capStage.style.setProperty("--cap-progress", String((currentIdx + 1) / n));
      if (progressLabel) {
        progressLabel.textContent =
          String(currentIdx + 1).padStart(2, "0") + " / " + String(n).padStart(2, "0");
      }
    }

    // Sequential Single-Step Gate: steps 1 -> 2 -> 3 at a balanced 600ms pace
    function activatePlateOnScroll(desiredIdx) {
      const n = cards.length;
      const clampedDesired = Math.max(0, Math.min(n - 1, desiredIdx));
      queuedTargetIdx = clampedDesired;

      if (currentIdx === -1) {
        commitCardSwitch(clampedDesired);
        return;
      }

      if (clampedDesired === currentIdx) {
        if (dwellTimer) {
          clearTimeout(dwellTimer);
          dwellTimer = null;
        }
        return;
      }

      const now = performance.now();
      const elapsed = now - lastSwitchTime;
      const nextStepIdx =
        currentIdx + (clampedDesired > currentIdx ? 1 : -1);

      if (elapsed >= MIN_CARD_DWELL_MS) {
        commitCardSwitch(nextStepIdx);
        if (queuedTargetIdx !== currentIdx) {
          if (dwellTimer) clearTimeout(dwellTimer);
          dwellTimer = setTimeout(() => {
            dwellTimer = null;
            if (queuedTargetIdx !== null && queuedTargetIdx !== currentIdx) {
              activatePlateOnScroll(queuedTargetIdx);
            }
          }, MIN_CARD_DWELL_MS);
        }
      } else if (!dwellTimer) {
        const waitMs = Math.max(50, MIN_CARD_DWELL_MS - elapsed);
        dwellTimer = setTimeout(() => {
          dwellTimer = null;
          if (queuedTargetIdx !== null && queuedTargetIdx !== currentIdx) {
            activatePlateOnScroll(queuedTargetIdx);
          }
        }, waitMs);
      }
    }

    function clamp01(v) {
      return Math.max(0, Math.min(1, v));
    }

    function computeHysteresisCardIndex(stageProgress, count, activeIdx) {
      const rawFloat = stageProgress * count;
      const candidate = Math.min(count - 1, Math.max(0, Math.floor(rawFloat)));
      if (activeIdx < 0 || candidate === activeIdx) return candidate;

      // Asymmetric deadband: forward advances stay responsive (0.12), backward
      // flips need a deliberate scroll-up (0.35) so rubber-band jolts and
      // URL-bar height shifts cannot rewind the counter mid-scroll
      const forwardDeadband = 0.12;
      const backwardDeadband = 0.35;
      if (candidate > activeIdx) {
        const boundary = activeIdx + 1;
        if (rawFloat < boundary + forwardDeadband) return activeIdx;
      } else if (candidate < activeIdx) {
        const boundary = activeIdx;
        if (rawFloat > boundary - backwardDeadband) return activeIdx;
      }
      return candidate;
    }

    // Balanced wheel speed cap inside #ov-capabilities: prevents flying past multiple cards
    // on a fast flick without making normal scroll feel blocked
    if (capStage) {
      window.addEventListener(
        "wheel",
        (e) => {
          const vh = window.innerHeight || 800;
          const rect = capStage.getBoundingClientRect();
          const isPinned = rect.top <= 56 && rect.bottom >= vh - 10;
          if (!isPinned) return;

          // Boundary escape: with the first/last card active there is nothing
          // left to pace, and clamping here trapped the user inside the runway
          // (reported: scrolling up from card 1 jittered and snapped back).
          if (currentIdx <= 0 && e.deltaY < 0) return;
          if (currentIdx >= cards.length - 1 && e.deltaY > 0) return;

          const now = performance.now();
          const inCooldown = now - lastSwitchTime < MIN_CARD_DWELL_MS * 0.65;
          const maxDelta = inCooldown ? 44 : 82;

          if (Math.abs(e.deltaY) > maxDelta) {
            e.preventDefault();
            // "instant", not "auto": html { scroll-behavior: smooth } turns
            // "auto" into an animated glide, and a stale in-flight animation
            // could re-target mid-gesture, so fast wheels made the page jump
            // up and snap back ("yukarı atıp tekrar aynı yere atıyor").
            window.scrollBy({
              top: Math.sign(e.deltaY) * maxDelta,
              behavior: "instant",
            });
          }
        },
        { passive: false }
      );
    }

    const quickJumpBtn = document.getElementById("quick-jump-download-btn");
    const paintedCtaSection = document.getElementById("ov-painted-cta");
    const paintedBannerWrap = document.getElementById("painted-banner-wrap");
    let isQuickJumping = false;

    if (quickJumpBtn) {
      quickJumpBtn.addEventListener("click", () => {
        const targetEl = paintedBannerWrap || paintedCtaSection;
        if (!targetEl) return;

        // Bypass wheel/dwell lock during direct jump to Download banner
        isQuickJumping = true;
        if (dwellTimer) {
          clearTimeout(dwellTimer);
          dwellTimer = null;
        }
        targetEl.scrollIntoView({ behavior: "smooth", block: "center" });
        setTimeout(() => {
          isQuickJumping = false;
        }, 1100);
      });
    }

    function onScrollFrame() {
      const vh = window.innerHeight || 800;

      // Toggle quick-jump button visibility (hide once user reaches the Download RISE v0.2.5 banner)
      if (quickJumpBtn && paintedCtaSection) {
        const ctaRect = paintedCtaSection.getBoundingClientRect();
        if (ctaRect.top <= vh * 0.68) {
          quickJumpBtn.classList.add("is-hidden-at-target");
        } else {
          quickJumpBtn.classList.remove("is-hidden-at-target");
        }
      }

      // A. Trigger typewriter for non-card headings/paragraphs as they scroll into view
      typeRegistry.forEach((item) => {
        if (item.started) return;
        const rect = item.el.getBoundingClientRect();
        if (rect.top <= vh * 0.93 && rect.bottom >= 0) {
          triggerTypewriterItem(item, 680, 0);
        }
      });

      // B. Pure-white scroll-driven sequence inside #ov-capabilities
      if (capStage && !isQuickJumping) {
        const rect = capStage.getBoundingClientRect();
        const scrollY = window.scrollY || window.pageYOffset || 0;
        const scrolledDown = scrollY > 20;
        const inView = scrolledDown && rect.top <= vh * 0.72 && rect.bottom >= vh * 0.20;

        if (inView) {
          // Capture once per entry; recapture only on a viewport jump (>150px,
          // e.g. rotation) — the runway is min-height:465vh, so innerHeight
          // changes were previously warping stageProgress mid-scroll
          if (!travelCache.ready || Math.abs(vh - travelCache.vh) > 150) {
            travelCache.vh = vh;
            travelCache.topDoc = rect.top + scrollY;
            travelCache.travel = Math.max(1, rect.height - vh);
            travelCache.ready = true;
          }
          const stageProgress = clamp01(
            (scrollY - travelCache.topDoc) / travelCache.travel
          );
          const scrollIdx = computeHysteresisCardIndex(
            stageProgress,
            cards.length,
            currentIdx
          );
          activatePlateOnScroll(scrollIdx);
        } else {
          travelCache.ready = false;
          if (rect.top > vh * 0.88 && currentIdx !== -1) {
            if (dwellTimer) {
              clearTimeout(dwellTimer);
              dwellTimer = null;
            }
            queuedTargetIdx = null;
            currentIdx = -1;
            cards.forEach((c) => resetPlateToWhitePaper(c));
            cards[0].classList.add("is-viewer-active", "is-active-piece");
            // Keep the counter in sync with the restored first card
            if (capStage) capStage.style.setProperty("--cap-progress", String(1 / cards.length));
            if (progressLabel) {
              progressLabel.textContent =
                "01 / " + String(cards.length).padStart(2, "0");
            }
          }
        }
      }

      // C. Multi-directional entrance for the 3 Setup Cards in #ov-workflows
      const wfDirs = [
        { x: -85, y: 32, rot: -2.2 },
        { x: 0, y: 78, rot: 0 },
        { x: 85, y: 32, rot: 2.2 },
      ];
      wfCards.forEach((card, idx) => {
        const rect = card.getBoundingClientRect();
        const startY = vh * 0.98;
        const endY = vh * 0.62;
        const p = clamp01((startY - rect.top) / Math.max(1, startY - endY));
        const ease = 1 - Math.pow(1 - p, 3);
        if (Math.abs(ease - wfLastEase[idx]) < 0.002) return;
        wfLastEase[idx] = ease;
        const dir = wfDirs[idx % wfDirs.length];
        const tx = (1 - ease) * dir.x;
        const ty = (1 - ease) * dir.y;
        const rot = (1 - ease) * dir.rot;
        card.style.opacity = (0.15 + 0.85 * ease).toFixed(3);
        card.style.transform = `translate3d(${tx.toFixed(1)}px, ${ty.toFixed(1)}px, 0) rotate(${rot.toFixed(2)}deg)`;
      });
    }

    let ticking = false;
    window.addEventListener(
      "scroll",
      () => {
        if (!ticking) {
          requestAnimationFrame(() => {
            onScrollFrame();
            ticking = false;
          });
          ticking = true;
        }
      },
      { passive: true }
    );

    window.addEventListener("resize", onScrollFrame, { passive: true });
    onScrollFrame();
  }

  initPureWhiteScrollShowcase();

  // ── 7. Refractive Water-Drop Wave Dropzone on 02 Verify (#drop) ──
  function initVerifierDropzoneReveal() {
    const dropEl = document.getElementById("drop");
    const fileInput = document.getElementById("fileIn");
    const videoEl = document.getElementById("tulip-video-a");
    const canvas = document.getElementById("tulip-ripple-canvas");
    if (!dropEl) return;

    dropEl.addEventListener("click", (e) => {
      if (e.target === fileInput) return;
      if (fileInput) {
        fileInput.value = "";
        fileInput.click();
      }
    });

    dropEl.addEventListener("keydown", (e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        if (fileInput) {
          fileInput.value = "";
          fileInput.click();
        }
      }
    });

    let lastDropCoords = null;
    let waveAnimId = null;

    function getRelativeCoords(clientX, clientY) {
      const rect = dropEl.getBoundingClientRect();
      const x = Math.max(0, Math.min(rect.width, clientX - rect.left));
      const y = Math.max(0, Math.min(rect.height, clientY - rect.top));
      return {
        x,
        y,
        w: rect.width,
        h: rect.height,
        pctX: ((x / rect.width) * 100).toFixed(2) + "%",
        pctY: ((y / rect.height) * 100).toFixed(2) + "%",
      };
    }

    function setOriginCoords(coords) {
      dropEl.style.setProperty("--drop-x", coords.pctX);
      dropEl.style.setProperty("--drop-y", coords.pctY);
    }

    function drawVideoCover(ctx, vid, cw, ch, offsetX = 0, offsetY = 0, scaleBoost = 1) {
      const vw = vid.videoWidth || 1920;
      const vh = vid.videoHeight || 1080;
      const scale = Math.max(cw / vw, ch / vh) * scaleBoost;
      const dw = vw * scale;
      const dh = vh * scale;
      const dx = (cw - dw) * 0.5 + offsetX;
      const dy = (ch - dh) * 0.5 + offsetY;
      ctx.drawImage(vid, dx, dy, dw, dh);
    }

    function renderRefractiveWaterWave(originX, originY) {
      if (!canvas) return;
      if (waveAnimId) cancelAnimationFrame(waveAnimId);

      const rect = dropEl.getBoundingClientRect();
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      const cw = Math.max(1, Math.round(rect.width || 620));
      const ch = Math.max(1, Math.round(rect.height || 320));
      canvas.width = cw * dpr;
      canvas.height = ch * dpr;

      const ctx = canvas.getContext("2d");
      if (!ctx) return;

      const maxRadius =
        Math.hypot(Math.max(originX, cw - originX), Math.max(originY, ch - originY)) *
        1.15;

      const duration = 2600;
      const startTime = performance.now();

      function frame(now) {
        const elapsed = now - startTime;
        const rawT = Math.min(1, elapsed / duration);
        const easeT = 1 - Math.pow(1 - rawT, 2.7);
        const fadeOut = rawT < 0.72 ? 1 : Math.max(0, 1 - (rawT - 0.72) / 0.28);

        const primaryR = easeT * maxRadius;
        const bandWidth = Math.max(68, Math.min(135, maxRadius * 0.22));

        dropEl.style.setProperty("--drop-r", `${Math.max(0, primaryR - bandWidth * 0.3).toFixed(1)}px`);
        dropEl.style.setProperty("--drop-edge", `${(primaryR + bandWidth * 0.35).toFixed(1)}px`);

        ctx.save();
        ctx.scale(dpr, dpr);
        ctx.clearRect(0, 0, cw, ch);

        const hasVideoFrame = videoEl && videoEl.readyState >= 2;

        if (hasVideoFrame) {
          const ringSteps = 26;
          const rStart = Math.max(0, primaryR - bandWidth * 0.85);
          const rEnd = primaryR + bandWidth * 0.55;
          const stepSize = (rEnd - rStart) / ringSteps;

          for (let i = 0; i < ringSteps; i++) {
            const rInner = rStart + i * stepSize;
            const rOuter = rInner + stepSize + 0.9;
            const rMid = (rInner + rOuter) * 0.5;
            const norm = (rMid - primaryR) / (bandWidth * 0.7);
            const wavePhase = norm * Math.PI * 2.2;
            const envelope = Math.exp(-norm * norm * 1.65) * fadeOut;

            if (envelope > 0.02) {
              const disp = Math.sin(wavePhase) * 18 * envelope;
              const zoomWarp = 1 + Math.cos(wavePhase) * 0.042 * envelope;

              ctx.save();
              ctx.beginPath();
              ctx.arc(originX, originY, Math.max(0.5, rOuter), 0, Math.PI * 2);
              if (rInner > 0.5) {
                ctx.arc(originX, originY, rInner, 0, Math.PI * 2, true);
              }
              ctx.clip("evenodd");

              drawVideoCover(
                ctx,
                videoEl,
                cw,
                ch,
                disp * 0.85,
                -disp * 0.65,
                zoomWarp
              );
              ctx.restore();
            }
          }
        }

        function drawThickLiquidCrest(radius, thickness, alphaScale) {
          if (radius <= 2 || alphaScale <= 0.01) return;
          const r0 = Math.max(0.5, radius - thickness * 0.65);
          const r1 = radius + thickness * 0.55;
          const grad = ctx.createRadialGradient(
            originX,
            originY,
            r0,
            originX,
            originY,
            r1
          );

          const a = alphaScale * fadeOut;
          grad.addColorStop(0.0, "rgba(255, 255, 255, 0)");
          grad.addColorStop(0.22, `rgba(12, 22, 30, ${(0.28 * a).toFixed(3)})`);
          grad.addColorStop(0.46, `rgba(165, 228, 255, ${(0.34 * a).toFixed(3)})`);
          grad.addColorStop(0.64, `rgba(255, 255, 255, ${(0.78 * a).toFixed(3)})`);
          grad.addColorStop(0.79, `rgba(255, 240, 200, ${(0.38 * a).toFixed(3)})`);
          grad.addColorStop(1.0, "rgba(255, 255, 255, 0)");

          ctx.fillStyle = grad;
          ctx.beginPath();
          ctx.arc(originX, originY, r1, 0, Math.PI * 2);
          ctx.arc(originX, originY, r0, 0, Math.PI * 2, true);
          ctx.fill("evenodd");
        }

        drawThickLiquidCrest(primaryR, bandWidth, 1.0);
        drawThickLiquidCrest(
          Math.max(0, (easeT - 0.14) * maxRadius * 0.92),
          bandWidth * 0.72,
          0.65
        );
        drawThickLiquidCrest(
          Math.max(0, (easeT - 0.29) * maxRadius * 0.78),
          bandWidth * 0.52,
          0.42
        );

        ctx.restore();

        if (rawT < 1) {
          waveAnimId = requestAnimationFrame(frame);
        } else {
          dropEl.classList.add("is-colorized");
          ctx.clearRect(0, 0, canvas.width, canvas.height);
        }
      }

      waveAnimId = requestAnimationFrame(frame);
    }

    function triggerWaterDropWave(coords) {
      setOriginCoords(coords);
      dropEl.classList.remove("is-colorized");
      renderRefractiveWaterWave(coords.x, coords.y);
    }

    // File-selected feedback box: markup/CSS exist, no JS ever toggled them.
    const finfoEl = document.getElementById("finfo");
    const fnameEl = document.getElementById("fname");
    const fsizeEl = document.getElementById("fsize");
    const fxBtn = document.getElementById("fx");

    function fmtSize(n) {
      if (typeof n !== "number" || !isFinite(n)) return "";
      if (n < 1024) return `${n} B`;
      if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
      return `${(n / (1024 * 1024)).toFixed(1)} MB`;
    }

    // VERIFY-GO-ENABLE-V1: no file -> gray disabled VERIFY; file present -> black enabled.
    function setGoEnabled(on) {
      const go = document.getElementById("goBtn");
      if (!go) return;
      go.disabled = !on;
      go.setAttribute("aria-disabled", on ? "false" : "true");
    }

    function showFile(f) {
      if (!f) return;
      window.__riseFile = f;
      if (fnameEl) fnameEl.textContent = f.name || "file";
      if (fsizeEl) fsizeEl.textContent = typeof f.size === "number" ? ` (${fmtSize(f.size)})` : "";
      if (finfoEl) finfoEl.classList.add("show");
      setGoEnabled(true);
    }

    function clearFile() {
      window.__riseFile = null;
      setGoEnabled(false);
      if (fileInput) fileInput.value = "";
      if (fnameEl) fnameEl.textContent = "";
      if (fsizeEl) fsizeEl.textContent = "";
      if (finfoEl) finfoEl.classList.remove("show");
      if (dropEl) dropEl.classList.remove("is-colorized");
      // B&W restore: stop any in-flight wave (its completion re-adds
      // is-colorized), close the veil's mask hole, and clear the ripple canvas.
      if (waveAnimId) {
        cancelAnimationFrame(waveAnimId);
        waveAnimId = null;
      }
      if (dropEl) {
        dropEl.style.setProperty("--drop-r", "0px");
        dropEl.style.setProperty("--drop-edge", "0px");
      }
      if (canvas) {
        const rippleCtx = canvas.getContext("2d");
        if (rippleCtx) rippleCtx.clearRect(0, 0, canvas.width, canvas.height);
      }
    }

    if (fxBtn) {
      fxBtn.addEventListener("click", (e) => {
        e.stopPropagation();
        clearFile();
      });
    }

    dropEl.addEventListener("dragover", (e) => {
      e.preventDefault();
      if (typeof e.clientX === "number" && typeof e.clientY === "number") {
        lastDropCoords = getRelativeCoords(e.clientX, e.clientY);
        setOriginCoords(lastDropCoords);
      }
    });

    dropEl.addEventListener("drop", (e) => {
      e.preventDefault();
      const dtFiles = e.dataTransfer && e.dataTransfer.files;
      if (dtFiles && dtFiles.length > 0 && fileInput) {
        try {
          const dt = new DataTransfer();
          dt.items.add(dtFiles[0]);
          fileInput.files = dt.files;
        } catch (_) {
          // DataTransfer assignment unsupported — still show the dropped file.
        }
        showFile(dtFiles[0]);
      }
      const coords =
        typeof e.clientX === "number" &&
        typeof e.clientY === "number" &&
        (e.clientX !== 0 || e.clientY !== 0)
          ? getRelativeCoords(e.clientX, e.clientY)
          : lastDropCoords || {
              x: (dropEl.clientWidth || 600) * 0.5,
              y: (dropEl.clientHeight || 320) * 0.5,
              pctX: "50%",
              pctY: "50%",
            };
      lastDropCoords = null;
      triggerWaterDropWave(coords);
    });

    if (fileInput) {
      fileInput.addEventListener("change", () => {
        if (fileInput.files && fileInput.files.length > 0) {
          showFile(fileInput.files[0]);
          const rect = dropEl.getBoundingClientRect();
          triggerWaterDropWave({
            x: (rect.width || 600) * 0.5,
            y: (rect.height || 320) * 0.5,
            pctX: "50%",
            pctY: "50%",
          });
        }
      });
    }

    window.__triggerRiseWaterDrop = function () {
      const rect = dropEl.getBoundingClientRect();
      triggerWaterDropWave({
        x: (rect.width || 600) * 0.5,
        y: (rect.height || 320) * 0.5,
        pctX: "50%",
        pctY: "50%",
      });
    };
  }

  initVerifierDropzoneReveal();

  // ── 7b. VERIFY-GO-ENABLE-V1: run client-side verification on VERIFY click ──
  // verify.js is a pure ES module (no DOM), so this wires #goBtn to its
  // extractRise/verifyBadge exports via dynamic import and renders the badge
  // into #res/#verdict/#why/#meter. #detH toggles details, #again resets.
  function initVerifyRun() {
    const go = document.getElementById("goBtn");
    if (!go || go.dataset.riseWired) return;
    go.dataset.riseWired = "1";
    const fileInput = document.getElementById("fileIn");
    const frameInput = document.getElementById("frameIn");
    const pubkeyEl = document.getElementById("pubkeyIn");
    const prog = document.getElementById("prog");
    const pbar = document.getElementById("pbar");
    const res = document.getElementById("res");
    const verdict = document.getElementById("verdict");
    const why = document.getElementById("why");
    const meter = document.getElementById("meter");
    const detB = document.getElementById("detB");

    function setProg(pct, show) {
      if (prog) prog.classList.toggle("show", !!show);
      if (pbar && show) {
        const n = Math.max(0, Math.min(100, Math.round(pct)));
        const bars = Math.round((n / 100) * 24);
        pbar.textContent = `[${"#".repeat(bars)}${".".repeat(24 - bars)}] ${n}%`;
      }
    }

    function showResult(badge, reason, facts) {
      if (!res) return;
      const b = String(badge || "unknown").toLowerCase();
      const bad = b === "red";
      res.classList.add("show");
      res.classList.remove("ok", "bad");
      res.classList.add(bad ? "bad" : "ok");
      if (verdict)
        verdict.textContent = bad
          ? "RED — untrusted"
          : b === "unknown"
            ? "UNKNOWN — unverified"
            : `${b.toUpperCase()} — verified`;
      if (why) why.textContent = reason || "";
      if (meter) meter.textContent = facts || "";
      if (detB) detB.textContent = facts || "";
    }

    const detH = document.getElementById("detH");
    if (detH && !detH.dataset.riseWired) {
      detH.dataset.riseWired = "1";
      detH.addEventListener("click", () => {
        const box = document.getElementById("det");
        if (box) box.classList.toggle("open");
      });
    }

    const again = document.getElementById("again");
    if (again && !again.dataset.riseWired) {
      again.dataset.riseWired = "1";
      again.addEventListener("click", (e) => {
        e.preventDefault();
        if (res) res.classList.remove("show", "ok", "bad");
        setProg(0, false);
        const fx = document.getElementById("fx");
        if (fx) fx.click();
      });
    }

    go.addEventListener("click", async () => {
      const f =
        (fileInput && fileInput.files && fileInput.files[0]) ||
        window.__riseFile ||
        null;
      if (!f || go.disabled) return;
      setProg(4, true);
      if (res) res.classList.remove("show", "ok", "bad");
      try {
        const mod = await import("./verify.js");
        const buf = new Uint8Array(await f.arrayBuffer());
        setProg(35, true);
        let manifest = null;
        let frameBytes = buf;
        let kind = "raw-bytes";
        const ext = mod.extractRise(buf);
        if (ext && ext.manifest) {
          manifest = ext.manifest;
          kind = ext.kind || "sealed";
          frameBytes = ext.clean || buf;
          if (frameInput && frameInput.files && frameInput.files[0]) {
            frameBytes = new Uint8Array(await frameInput.files[0].arrayBuffer());
            kind += "+detached-frame";
          }
        } else {
          const text = new TextDecoder().decode(buf);
          JSON.parse(text);
          manifest = text;
          kind = "json-manifest";
          if (frameInput && frameInput.files && frameInput.files[0]) {
            frameBytes = new Uint8Array(await frameInput.files[0].arrayBuffer());
          } else {
            frameBytes = new Uint8Array(0);
          }
        }
        setProg(62, true);
        const pem = pubkeyEl && pubkeyEl.value.trim() ? pubkeyEl.value.trim() : undefined;
        const out = await mod.verifyBadge(manifest, frameBytes, pem ? { pubkeyPem: pem } : {});
        setProg(100, true);
        const capNote = out.captureNote ? " — " + out.captureNote : "";
        showResult(
          out.badge || "unknown",
          (out.reason || "") + capNote,
          `${f.name || "file"} · ${buf.length} bytes · ${kind}${out.captureSource ? " · capture:" + out.captureSource : ""}`
        );
      } catch (err) {
        setProg(0, false);
        showResult(
          "red",
          "Unrecognized container: drop a sealed .rise/.rvid file or a .json manifest.",
          `${f.name || "file"} · not a sealed container`
        );
      }
    });
  }

  // ── 7c. RISE-GH-STARS-V1: GitHub stars badge (memory-only cache, fail silent) ──
  let riseStarsCache = null;
  function initGithubStars() {
    const el = document.getElementById("gh-stars");
    if (!el) return;
    if (riseStarsCache !== null) {
      el.textContent = riseStarsCache;
      return;
    }
    fetch("https://api.github.com/repos/demiralpdev/.RISE", {
      headers: { Accept: "application/vnd.github+json" },
    })
      .then((r) => {
        if (!r.ok) throw new Error(`stars ${r.status}`);
        return r.json();
      })
      .then((d) => {
        const n = d && d.stargazers_count;
        if (typeof n !== "number") return;
        riseStarsCache =
          "★ " + (n >= 1000 ? `${(n / 1000).toFixed(1).replace(/\.0$/, "")}k` : `${n}`);
        el.textContent = riseStarsCache;
      })
      .catch(() => {});
  }

  initVerifyRun();
  initGithubStars();

  // ── 7d. RISE-OSDL-V1: lead with the visitor's OS in downloads ──
  function riseVisitorOS() {
    try {
      const ua = navigator.userAgent || "";
      if (/Android/i.test(ua)) return "android";
      if (/iPhone|iPad|iPod/i.test(ua)) return "ios";
      const pl =
        (navigator.userAgentData && navigator.userAgentData.platform) ||
        navigator.platform ||
        "";
      if (/Win/i.test(pl) || /Windows/i.test(ua)) return "windows";
      if (/Mac/i.test(pl) || /Macintosh/i.test(ua)) return "macos";
      if (/Linux/i.test(pl) || /Linux/i.test(ua)) return "linux";
    } catch (_) {}
    return null;
  }

  function initDeviceDownloads() {
    const os = riseVisitorOS();
    if (!os) return;
    const names = {
      android: "Android",
      ios: "iOS",
      windows: "Windows",
      macos: "macOS",
      linux: "Linux",
    };
    let matchedItem = null;
    // Download view: pull the matching OS row to the top and tag it.
    try {
      const rows = document.querySelector(".dl-rows");
      if (rows) {
        const items = Array.prototype.slice.call(rows.querySelectorAll(".dl-item"));
        for (let i = 0; i < items.length; i++) {
          const titleEl = items[i].querySelector(".dl-os-title");
          if (!titleEl) continue;
          if (titleEl.textContent.trim().toLowerCase() !== os) continue;
          items[i].classList.add("is-device-match");
          if (!titleEl.querySelector(".dl-you")) {
            const pill = document.createElement("span");
            pill.className = "dl-you";
            pill.textContent = "your device";
            titleEl.appendChild(pill);
          }
          rows.insertBefore(items[i], rows.firstChild);
          matchedItem = items[i];
          break;
        }
      }
    } catch (_) {}
    // Painted bottom CTA: the baked HTML pill hardcodes "· Apple Silicon" with
    // the macOS .pkg link, so Android visitors were offered the Mac installer.
    // Retarget it to the visitor's own row (first real chip); macOS stays as-is.
    try {
      const ctaLabels = {
        android: "\u00b7 Android (build guide)",
        windows: "\u00b7 Windows x64",
        linux: "\u00b7 Linux x64",
      };
      if (matchedItem && os !== "macos" && ctaLabels[os]) {
        const cta = document.querySelector("#ov-painted-cta .pill-cta-primary");
        const muted = cta ? cta.querySelector(".pill-cta-muted") : null;
        const chip = matchedItem.querySelector(".dl-chip[href]");
        if (cta && muted && chip) {
          cta.setAttribute("href", chip.getAttribute("href"));
          if (chip.getAttribute("rel")) {
            cta.setAttribute("rel", chip.getAttribute("rel"));
          } else {
            cta.removeAttribute("rel");
          }
          muted.textContent = ctaLabels[os];
        }
      }
    } catch (_) {}
    // Quick Download sidebar: lead with the visitor's OS chip (mobile gets a pointer).
    try {
      const panels = document.querySelectorAll(".glass-panel");
      Array.prototype.forEach.call(panels, function (panel) {
        const kicker = panel.querySelector(".panel-kicker");
        if (!kicker || !/Quick Download/i.test(kicker.textContent || "")) return;
        const firstChip = panel.querySelector("a.dl-chip");
        if (!firstChip) return;
        const box = firstChip.parentElement;
        if (!box) return;
        if (os === "android" || os === "ios") {
          const hint = document.createElement("a");
          hint.className = "dl-chip sec";
          hint.setAttribute("href", "./index.html?view=download");
          hint.style.justifyContent = "space-between";
          const s1 = document.createElement("span");
          s1.textContent = names[os] + " setup \u2192";
          const s2 = document.createElement("span");
          s2.textContent = "see Download";
          hint.appendChild(s1);
          hint.appendChild(s2);
          box.insertBefore(hint, box.firstChild);
          return;
        }
        const chips = Array.prototype.slice.call(box.querySelectorAll("a.dl-chip"));
        for (let i = 0; i < chips.length; i++) {
          const label = (chips[i].textContent || "").toLowerCase();
          const hit =
            (os === "windows" && /windows/.test(label)) ||
            (os === "macos" && /macos/.test(label) && /pkg|installer/.test(label)) ||
            (os === "linux" && /linux/.test(label));
          if (!hit) continue;
          box.insertBefore(chips[i], box.firstChild);
          break;
        }
      });
    } catch (_) {}
  }

  initDeviceDownloads();

  // ── 8. Install Terminal Tabs & Copy Buttons + Sample .rise Trigger ──
  const tabBtns = document.querySelectorAll(".tab-btn[data-tab]");
  const termPanels = document.querySelectorAll(".term-panel");

  tabBtns.forEach((btn) => {
    btn.addEventListener("click", () => {
      const targetTab = btn.getAttribute("data-tab");
      tabBtns.forEach((b) => {
        const isSel = b === btn;
        b.setAttribute("aria-selected", isSel ? "true" : "false");
        b.setAttribute("tabindex", isSel ? "0" : "-1");
      });
      termPanels.forEach((panel) => {
        panel.classList.toggle("active", panel.id === `panel-${targetTab}`);
      });
    });
  });

  document.querySelectorAll("[data-copy]").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const codeId = btn.getAttribute("data-copy");
      const codeEl = document.getElementById(codeId);
      if (!codeEl) return;
      try {
        await navigator.clipboard.writeText(codeEl.innerText.trim());
        const prev = btn.textContent;
        btn.textContent = "Copied ✓";
        setTimeout(() => {
          btn.textContent = prev;
        }, 1600);
      } catch (_) {}
    });
  });

  const sampleNames = ["IST_4082.rise", "ARC_4089.rise", "ALP_4104.rise"];
  document.querySelectorAll("[data-sample-idx]").forEach((btn) => {
    btn.addEventListener("click", async (e) => {
      e.preventDefault();
      const idx = parseInt(btn.getAttribute("data-sample-idx") || "0", 10);
      const fname = sampleNames[idx] || "IST_4082.rise";

      switchView("workspace", true);

      try {
        const canvas = document.createElement("canvas");
        canvas.width = 640;
        canvas.height = 400;
        const ctx = canvas.getContext("2d");

        const grad = ctx.createLinearGradient(0, 0, 640, 400);
        grad.addColorStop(0, "#1d2d22");
        grad.addColorStop(0.5, "#3b2c1e");
        grad.addColorStop(1, "#121a15");
        ctx.fillStyle = grad;
        ctx.fillRect(0, 0, 640, 400);

        ctx.fillStyle = "#f5f2eb";
        ctx.font = "600 26px Georgia, serif";
        ctx.fillText("RISE Sealed Sample — " + fname, 40, 72);
        ctx.font = "14px monospace";
        ctx.fillStyle = "#ffcf70";
        ctx.fillText("JPEG APP11 JUMBF · 4-Tile SHA-256 · ES256 + ML-DSA-65", 40, 112);

        const blob = await new Promise((res) => canvas.toBlob(res, "image/jpeg", 0.92));
        const file = new File([blob], fname, { type: "image/jpeg" });

        const fileInput = document.getElementById("fileIn");
        if (fileInput) {
          const dt = new DataTransfer();
          dt.items.add(file);
          fileInput.files = dt.files;
          fileInput.dispatchEvent(new Event("change", { bubbles: true }));
        } else if (typeof window.__triggerRiseWaterDrop === "function") {
          window.__triggerRiseWaterDrop();
        }
      } catch (err) {
        console.error("Sample generation error:", err);
      }
    });
  });

  // Advanced (detached frame / custom PEM key) collapsible: CSS reveals
  // .adv-b only under .adv.open, but no handler ever toggled that class.
  const advBox = document.getElementById("adv");
  const advH = document.getElementById("advH");
  if (advBox && advH) {
    advH.addEventListener("click", () => {
      const open = advBox.classList.toggle("open");
      advH.setAttribute("aria-expanded", open ? "true" : "false");
    });
  }

  // ── 0. Branded splash preloader (#rise-preloader): show only with a fully-decoded logo ──
  // The logo must never render partially: the splash is held hidden
  // (.preloader-wait) and the logo itself stays visibility:hidden (styles.css)
  // until the logo bytes are fully decoded; on error the text-only fallback
  // shows (.logo-failed hides the <img>, never a broken-image glyph).
  (function initRisePreloader() {
    var el = document.getElementById("rise-preloader");
    if (!el) return;
    var done = false;
    var LOGO_SRC = "./rise-eye-logo-transparent.png";
    function reveal() {
      if (done) return;
      el.classList.remove("preloader-wait");
      el.classList.add("logo-ready");
    }
    function revealTextOnly() {
      if (done) return;
      el.classList.remove("preloader-wait");
      el.classList.add("logo-failed");
    }
    function hide() {
      if (done) return;
      done = true;
      el.classList.remove("preloader-wait");
      el.classList.add("is-done");
      setTimeout(function () {
        if (el && el.parentNode) el.parentNode.removeChild(el);
      }, 450);
    }
    el.classList.add("preloader-wait");
    var logo = el.querySelector(".preloader-logo");
    function logoReady(img) {
      try {
        if (img && typeof img.decode === "function") {
          img.decode().then(reveal, revealTextOnly);
          return;
        }
      } catch (_) {}
      reveal();
    }
    try {
      if (logo && logo.complete && logo.naturalWidth > 0) {
        logoReady(logo);
      } else {
        var probe = new Image();
        probe.onload = function () {
          logoReady(logo);
        };
        probe.onerror = function () {
          revealTextOnly();
        };
        if (logo) {
          logo.addEventListener("error", revealTextOnly, { once: true });
        }
        probe.src = LOGO_SRC;
      }
    } catch (_) {
      reveal();
    }
    if (document.readyState === "complete") {
      setTimeout(hide, 60);
      return;
    }
    window.addEventListener("load", function () {
      setTimeout(hide, 60);
    }, { once: true });
    setTimeout(hide, 8000);
  })();

  // FAQ accordion: one answer open at a time (instant, no transition).
  document.querySelectorAll(".faq-item").forEach((item) => {
    const btn = item.querySelector(".faq-q");
    if (!btn) return;
    btn.addEventListener("click", () => {
      const wasOpen = btn.getAttribute("aria-expanded") === "true";
      document.querySelectorAll(".faq-item .faq-q").forEach((b) => {
        b.setAttribute("aria-expanded", "false");
        const panel = document.getElementById(b.getAttribute("aria-controls"));
        if (panel) panel.hidden = true;
      });
      if (!wasOpen) {
        btn.setAttribute("aria-expanded", "true");
        const panel = document.getElementById(btn.getAttribute("aria-controls"));
        if (panel) panel.hidden = false;
      }
    });
  });
})();

// Offline cache registration (perf infra only, fail-safe): if Service Workers
// are unavailable or registration fails, the site works exactly as today.
try {
  if (
    typeof window !== "undefined" &&
    "serviceWorker" in navigator &&
    typeof window.location !== "undefined" &&
    /^https?:$/.test(window.location.protocol)
  ) {
    window.addEventListener(
      "load",
      function () {
        try {
          navigator.serviceWorker
            .register("./sw.js")
            .catch(function () {});
        } catch (_) {}
      },
      { once: true }
    );
  }
} catch (_) {}
