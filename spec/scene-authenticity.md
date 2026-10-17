# Scene Authenticity — Re-Capture Defense & Two-Axis Verification

> **Status: DESIGN (partially implemented).** Audit of `spec/` vs the real code + technical
> architecture + P0/P1/P2 roadmap + mobile reality check.
> Source: `spec/RISE-01.md` §5.1, `spec/threat-model.md` #2, `spec/solutions-physical.md`
> #1/#5/#7/#8/#10/#11, `spec/solutions-digital.md` #8, `PLAN.md` §7, `ROADMAP.md`.
>
> **Core diagnosis:** RISE currently produces only a **data-origin & integrity** proof
> ("these bytes were signed with this key at this timestamp"); it does **not** produce a
> **scene-authenticity** proof ("these bytes were formed by photons from a real 3D world").
>
> **Turkish original:** kept side-by-side as `spec/scene-authenticity.tr.md`.
>
> **Implementation status (P0, core path — DONE):** the capture class is now recorded and
> signature-covered, and the verifier labels it. See §6.

---

## 0. Pre-Audit: Are the `spec/` Defenses in the Code?

Short answer: apart from the `capture_source: "screen"` declaration, every physical/re-capture
defense currently exists only in the design docs (`spec/`); it is not implemented. Worse, the
existing capture shells contain a device-selection logic that reverses security:

| Spec defense | Actual state in the codebase | Evidence / Location |
|---|---|---|
| `threat-model` #2: frame-interval + stamp-order check → `suspicious` | **Missing (spec only).** `VerifyDecision` in `lib.rs` has only `Valid`, `Red`, `Unknown` (no `Suspicious`). `ManifestV1` holds a single `timestamp_ms`; there is no per-frame time series, and `main.rs` (`video_cmd`) and `RiseCapture.kt` extract frames post-hoc at a fixed rate (1 fps / 200 ms). | `lib.rs`, `verify.py` |
| `solutions-physical` #1 & #10: nonce light challenge + moiré/refresh artifacts | **Missing (spec only).** No shell triggers a flash/screen burst, and there is no FFT/moiré frequency analysis in `core/`. | `solutions-physical.md` |
| `solutions-physical` #5: fake USB camera (USB = auto silver) and virtual-camera rejection | **Inverted / missing.** `capture.cpp`, `capture.swift`, `Capture.swift` and `capture.sh` prefer external cameras whose name contains "USB", "UVC", "Rapoo" over the built-in camera. Windows writes `usb=1` to a `.meta` sidecar, but that bit never enters the signed `ManifestV1`. No OS has a virtual-camera (OBS, v4l2loopback) filter. | `capture.cpp`, `capture.swift`, `capture.sh` |
| OBS binding hole: `rise-auto-seal.py` | **Active hole.** On recording stop, `rise-auto-seal.py` runs `rise-core video <path> --device obs`; the manifest leaves `capture_source` empty (`None`), so the verifier evaluates the OBS recording as a camera/file seal. | `rise-auto-seal.py` |
| `solutions-physical` #7, #8, #11: encrypted MIPI, depth (L4), sensor-serial matching | **Missing (spec only).** No code reads depth, sensor serial or MIPI state from hardware/OS APIs. | `solutions-physical.md` |
| `solutions-digital` #8: screenshot laundering (artifacts + watermark) | **Partial (declaration only).** No in-image compression/watermark detection. Only `rise-core screen` writes `capture_source: "screen"` and `verify.py` shows the warning. | `main.rs`, `verify.py` |
| Windows gold-L2 binding: TPM + VBS (`attest.cpp`) | **Disconnected (self-asserted).** As noted in `REDTEAM_FINDINGS.md` (F7), `attest.exe` creates a P-256 key in the TPM and prints `TIER=gold-L2`, but `rise-core sign` still uses the on-disk software PEM key (`--key`); the TPM signature and VBS quote are not bound to the manifest. | `attest.cpp`, `REDTEAM_FINDINGS.md` |

---

## 1. Threat Taxonomy of the Re-Capture Family

| # | Threat vector | Attack mechanism and why it currently passes | Detection nature: provable vs heuristic |
|---|---|---|---|
| 1 | **Virtual camera** (OBS, ManyCam, v4l2loopback, DirectShow/CMIO) | The attacker feeds an AI still/video into a user-space virtual-camera driver. `capture.cpp`, `capture.sh` or `capture.swift` opens the first video source it finds, takes the YUY2/JPEG frame and hashes/signs it with `rise-core`. Because there is no device-topology / bus check, it produces a manifest byte-identical to a legitimate camera capture. | **Provable on a signed OS/app environment (deterministic):** on a locked-bootloader app verified by App Attest / Play Integrity, the OS camera API can deterministically tell virtual/external cameras apart and reject them. On open desktop OSes (Linux/Windows root/admin) it is not provable (the kernel driver can be impersonated). |
| 2 | **Screen re-photography / HDMI capture** (analog hole & UVC capture card) | **2a (optical):** a real phone/webcam is aimed at a high-PPI OLED screen. Photons pass through the real lens and real CMOS sensor, so every hardware attestation (StrongBox, SE, encrypted MIPI included) passes 100%. **2b (HDMI→UVC):** an HDMI capture card presents itself as a standard USB UVC webcam and emits the AI output as raw YUY2 frames. | **2a (optical screen): heuristic only (probabilistic).** No cryptographic layer can prove the photon source in front of the sensor; only depth (parallax), IMU and optical artifacts catch it probabilistically. **2b (HDMI UVC): provable (rule-based).** Every external USB/UVC device is detected from the hardware path (`USB\VID_...`) and deterministically barred from Gold. |
| 3 | **Print re-capture** | An AI image is printed on high-quality matte photo paper and photographed with a real camera. It bypasses screen-detection algorithms entirely because there is no refresh rate (PWM/flicker), pixel grid (moiré) or screen-glare artifact. | **Heuristic only (physical consistency).** Detected probabilistically only via 3D depth (LiDAR / dual-camera stereo disparity) or the absence of 3D parallax when the camera moves (all points lying on a single 2D homography plane). |
| 4 | **Replay of previously signed media** (pre-signed / stream replay) | **4a (file replay):** media that was genuinely captured and signed earlier is presented as a new event. **4b (stream replay):** a previously recorded real camera stream (RAW/YUV) is fed back into `rise-core` at capture time from a virtual camera or memory. | **Provable (cryptographic).** 4a: the RFC 3161 QTSP timestamp proves the capture *upper* bound, and a fresh verifier nonce embedded in the signed manifest (or a NIST randomness beacon / block digest) proves the *lower* bound. 4b: not provable without a live challenge-response (nonce); provable with one. |
| 5 | **Emulator/VM with fake attestation** (QEMU, Android Emulator, swtpm) | The attacker runs a virtual TPM (swtpm) / software Keystore inside an emulator or VM and feeds an AI frame to the virtual camera. Because the current `attest.cpp` only checks that it can open a key in the local TPM (it does not validate the EK/AIK certificate chain), swtpm easily earns gold-L2. | **Provable (cryptographic — within PKI bounds).** Server-side verification of the TPM Endorsement Key (EK) certificate up to the vendor root (AMD/Intel/Infineon), of the Android Key Attestation chain up to Google's Hardware Root CA (plus the revoked leaked-keybox list), and of iOS App Attest up to Apple's root. |
| 6 | **Rooted device that bypasses the OS camera source** (Frida / Zygisk / HAL hook) | On a real phone the hardware TEE/StrongBox key exists, but the device is rooted (or the bootloader is unlocked). The attacker hooks `libcameraservice`, the Camera2 `ImageReader` or the JNI `rise_hash_frame` function with Frida/LSPosed and swaps the YUV buffer for an AI frame just before signing, then has it signed with the real TEE key. | **Partly provable (bootloader/attestation) + partly heuristic (runtime hooking).** The bootloader lock and `verifiedBootState == VERIFIED` are proven cryptographically in the hardware-backed Key Attestation extension. But with a 0-day kernel/in-app memory injection on a locked bootloader, detecting it without in-sensor signing (L4) is heuristic. |

---

## 2. Defense-in-Depth — Ranked by (Effectiveness × Feasibility)

Each layer is scored **(effectiveness / 10) × (feasibility / 10) = leverage score** (out of 100).

### Rank 1: OS camera-device source validation and virtual/loopback/external rejection

- **Score:** effectiveness 8/10 × feasibility 9.5/10 = **76 / 100**
- **Detection nature:** provable on locked mobile OSes (iOS/Android); on desktop OSes
  (Windows/Linux) it is a high-barrier heuristic/rule-based check (a kernel-level root/admin
  can defeat it).
- **Concrete per-platform implementation:**
  - **iOS (`AVCaptureDevice.deviceType`):** drop the `.external` device type in `Capture.swift`.
    Accept only `.builtInWideAngleCamera`, `.builtInUltraWideCamera`, `.builtInTelephotoCamera`;
    reject `.external` and `.continuityCamera` sources from any Gold/Silver camera flow.
  - **Android (`Camera2 CameraCharacteristics`):** in `RiseCapture.kt` reject
    `LENS_FACING == LENS_FACING_EXTERNAL` (require `LENS_FACING_BACK` or `LENS_FACING_FRONT`).
    Also require `INFO_SUPPORTED_HARDWARE_LEVEL` to be other than `LEGACY`/`EXTERNAL`
    (`LIMITED`, `FULL` or `LEVEL_3`), and check that `SENSOR_INFO_PHYSICAL_SIZE` and
    `LENS_INFO_AVAILABLE_FOCAL_LENGTHS` are within physical bounds.
  - **Windows (MediaFoundation + SetupAPI/CfgMgr32):** the `FriendlyName` text match
    (`IsUsbSource`) in `capture.cpp` is unsafe on its own because OBS Virtual Camera can rename
    itself. Read `MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK`:
    - if the symbolic link starts with `\\?\root#...` or `\\?\swd#...` (Software Device), it is
      a virtual camera → **RED**;
    - query `DEVPKEY_Device_EnumeratorName` and the ACPI `_PLD` (Physical Location of Device)
      via `CM_Get_DevNode_Property`: if the device is not embedded in the laptop chassis
      (i.e. `usb=1` external UVC), write `"capture_source": "external_usb"` into the manifest and
      cap the badge at **silver**.
  - **Linux (v4l2 + sysfs):** check `/sys/class/video4linux/videoN/device`. Virtual cameras such
    as `v4l2loopback` and `akvcam` live under `/sys/devices/virtual/video4linux/` and have no
    physical PCI/USB bus device symlink → **RED**.

### Rank 2: Capture-pipeline hardware attestation and key binding

- **Score:** effectiveness 9/10 × feasibility 8/10 = **72 / 100**
- **Detection nature:** cryptographically provable (up to vendor PKI roots).
- **What it proves:**
  1. The private key was generated in a real hardware chip (Android StrongBox/TEE KeyMint, iOS
     Secure Enclave, Windows physical TPM 2.0) and cannot be exported.
  2. Via the Android Key Attestation X.509 extension (`1.3.6.1.4.1.11129.2.1.17`) and iOS App
     Attest, that the device bootloader is locked (`VerifiedBootState == VERIFIED`), the OS patch
     level is current, and the calling app is the officially signed RISE package (`applicationId`
     + signing-certificate SHA-256 digest).
- **What it does not prove:**
  1. It does not prove the scene in front of the camera is the real 3D world (a screen/print can be
     photographed).
  2. Even on a locked device at capture time, it does not prove the raw pixel path between sensor
     and TEE if the app internally calls the wrong API or has memory corruption (because the hash
     is taken outside the TEE, in app memory).
- **Critical missing piece to fix in the current code:** on Windows, `attest.cpp` only prints
  `TIER=gold-L2`; it must actually sign the manifest with the in-TPM key via `NCryptSignHash` and
  add the TPM EK/AIK certificate chain + PCR/VBS quote to the manifest. On Android,
  `RiseKeystore.kt` must put the full X.509 attestation chain into the manifest via
  `store.getCertificateChain(alias)` (today it only takes the SPKI PEM from the leaf certificate).

### Rank 3: Challenge-response liveness (temporal nonce + interactive motion)

- **Score:** effectiveness 8.5/10 × feasibility 7.5/10 = **63.75 / 100**
- **Detection nature:**
  - **Time window (freshness / anti-replay): cryptographically provable.** At capture time the app
    embeds a 32-byte `server_nonce` obtained from the verifier server (or, when offline, from the
    last known drand/NIST randomness-beacon round) into both the KeyStore/AppAttest signature and
    the manifest; the output is stamped with an RFC 3161 TSA. This mathematically proves the media
    was produced inside the narrow window between `T_nonce` and `T_tsa` (e.g. ≤10 seconds).
  - **Scene / pose challenge (active liveness): heuristic / probabilistic.** In high-risk flows
    (e.g. insurance damage, notary/KYC, field inspection) the user is asked to tilt/slide the
    camera (pitch/yaw trajectory) toward a randomly appearing on-screen target for 1.5 seconds.
    Synchronizing a pre-recorded video or a static screen to that instantaneous trajectory requires
    real-time robotics/rendering rigging.

### Rank 4: Cross-sensor signals bound into the manifest (IMU ↔ optical flow, dual-lens depth, ENF/light)

- **Score:** effectiveness 8/10 × feasibility 7/10 = **56 / 100**
- **Detection nature:** heuristic only (physical-consistency checking). It is not cryptographic
  proof, but it is the strongest layer for catching analog-hole (2D screen and print) attacks:

| Cross signal | How it works | Reliability & limits |
|---|---|---|
| **IMU (gyro/accelerometer) ↔ frame optical-flow correlation** | A 100–200 Hz IMU window is recorded for 500 ms around the shutter. The global optical flow computed from consecutive video/burst frames (homography rotation) is cross-correlated with the gyro angular rate (`∫ ω dt`) — check r > 0.85 and phase delay < 15 ms. | **High.** A video fed into a virtual camera does not match the phone's hand tremor; a camera aimed at a screen reveals the near-field perspective shift as a 2D-plane homography. |
| **Dual-lens disparity (Wide + Ultra-Wide) / LiDAR depth** | At the shutter, two simultaneous frames are taken from the main and ultra-wide cameras (or motion parallax from a short burst). If the computed depth map fits a single plane (`z = ax + by + c`, RANSAC inlier ratio > 92%), the scene is a flat screen or paper print. | **High.** Catches 2D screen and paper-print attacks by physical geometry. Discriminative power drops only for real 3D props/statues or far-distance landscape (infinite focus). |
| **Ambient-light flicker ↔ mains frequency (50/60 Hz ENF)** | The phone's ambient-light sensor (ALS) and the rolling-shutter row-brightness variation (100 Hz / 120 Hz lighting flicker) are compared with the country's mains frequency (e.g. Turkey/EU 50 Hz → 100 Hz flicker; US 60 Hz → 120 Hz), and the screen refresh rate (60/120/144 Hz) harmonic is searched for. | **Medium-low.** No signal in natural daylight or DC LED lighting; use as a supporting signal only. |

### Rank 5: In-image statistical & sensor signals (reliable vs weak split)

- **Score:** effectiveness 4.5/10 × feasibility 6/10 = **27 / 100**
- **Detection nature:** all heuristic (statistical). None can ever be a "green/gold" criterion on
  its own; they can only raise a "suspicious (downgrade)" flag:

| Signal | Reliability | Why reliable or weak? |
|---|---|---|
| **PRNU (sensor fingerprint)** | Medium (RAW/YUV only) | A fixed-pattern noise from the sensor's manufacturing imperfections. Strong in raw Bayer/YUV frames with a reference dark/flat frame from the same device. But H.264/HEVC compression, multi-frame HDR/Night-Mode fusion, and resolution scaling rapidly destroy PRNU correlation. |
| **Rolling-shutter row temporality** | Medium (with IMU) | CMOS rows are read out top-to-bottom (~10–15 ms readout). Under fast motion, the skew of vertical lines must match the gyro data exactly. In the image alone it is easily simulated in software. |
| **Screen moiré & refresh band** | Weak-medium | Produces periodic peaks in the FFT spectrum of amateur screen captures. But slightly defocusing the camera, adjusting angle/distance, or using a matte screen protector eliminates the moiré entirely (consistent with `REDTEAM_FINDINGS.md` Round 4 A2). |
| **Lens distortion & specular/light consistency** | Weak | When you photograph a screen with a real camera, the camera's own radial lens distortion and vignetting are already applied to that frame! AI generators have also surpassed human and classical-detector thresholds for light/reflection consistency. |
| **EXIF plausibility** | Very weak (garbage) | EXIF tags (Make, Model, ExposureTime, ISO) are plain text written by client software; `exiftool` can replace them with a real iPhone 16 Pro profile byte-for-byte in 1 millisecond. |

---

## 3. The Limit That Can Never Be Solved (Analog Hole) and Honest Badge/UX Architecture

### The unsolvable limit

No cryptographic protocol (C2PA, RISE, or hardware-signed cameras) can prove with certainty
whether photons hitting the optical sensor came from a real physical event or from a calibrated
projection/micro-LED scene (**Analog Hole**). Even in-sensor signing (Gold-L4) only says "these
pixels really came out of this CMOS sensor's ADC"; it does not say what the sensor was looking at.

### Honest badge and UX design: the two-axis verification model

A single Gold / Silver / Red badge creates the illusion in the user's mind that "Gold = the content
is real, it cannot be AI". To prevent this, `ManifestV1` and the verify-web UI must split
verification into two independent axes:

**1. Axis A — Pipeline Provenance (cryptographic proof):**
- **Gold-L2 (hardware-attested camera):** locked device + TEE/SE/StrongBox signature + verified
  built-in physical camera API + multi RFC 3161 TSA.
- **Silver (software / external camera):** software key or external USB camera.
- **Screen / Virtual (screen or virtual capture):** `capture_source: "screen" | "obs" | "virtual"`.
  Never gets a camera badge; gets a separate blue/grey "Screen Recording" badge.

**2. Axis B — Scene Liveness (probabilistic / physical consistency):**
- **Liveness: 3D + motion verified:** dual-camera/LiDAR depth map is not planar AND the IMU/optical
  flow correlation passes.
- **Liveness: not verified (single frame / planar):** captured with a physical camera, but there is
  no depth/motion data or the scene is a flat surface (screen/print/wall).
- **Liveness: suspicious (downgrade):** a 2D screen/moiré trace or an IMU-image mismatch was detected.

### UX text templates (for verify-web and the CLI)

- **Hardware-attested + 3D scene consistent:**
  > **Hardware-Attested Camera Capture (Gold-L2) · 3D Scene Consistent**
  > This media was recorded at [date/time] from a verified built-in camera sensor, signed with a
  > hardware security chip, and not a single pixel has been changed since. Multi-lens depth and
  > motion-sensor data are consistent with a 3-dimensional physical scene.

- **Hardware-attested + liveness not verified (critical honesty statement):**
  > **Hardware-Attested Camera Capture (Gold-L2) · Scene Depth Not Verified**
  > This media was taken with a real physical camera and has not been altered. Limit: because the
  > capture was a single/planar frame, it cannot be technically proven whether the camera was aimed
  > at a real event or at another screen/print (analog hole).

- **OBS / screen capture (`capture_source: "screen"` or `"obs"`):**
  > **Timestamped Screen/Software Recording (Not Camera Evidence)**
  > This file proves that the pixels shown on the device screen or in the OBS scene at [date/time]
  > were sealed unaltered. It does not prove whether the content shown on the screen is AI-generated
  > or staged.

---

## 4. P0 / P1 / P2 Phased Roadmap and the Highest-Leverage First Step

### The cheapest, highest-leverage FIRST STEP (1–2 days, zero cost)

Close the virtual/OBS/external-camera gate and make the `capture_source` field mandatory:

1. Add a `--capture-source obs` parameter to the `rise-core video` call inside `rise-auto-seal.py`
   (open a `--capture-source` flag on the `sign` and `video` subcommands in `main.rs`).
2. In `capture.cpp`, `capture.swift`, `Capture.swift` and `capture.sh`, remove the inverted logic
   that prefers the USB camera over the built-in camera; make the built-in physical camera the first
   priority, reject virtual cameras (`v4l2loopback`, Windows `SWD\`, OBS) entirely, and write a
   mandatory `"capture_source": "external_usb"` into the manifest for USB cameras.
3. In `verify.py`, make sure any package whose `capture_source` is `"screen"`, `"obs"` or
   `"external_usb"` never enters camera Gold candidacy and prints an open-source warning.

### Phased roadmap (P0 → P1 → P2)

| Phase | Scope and engineering tasks | Concrete acceptance criteria |
|---|---|---|
| **P0 (1–2 weeks):** virtual-camera rejection + source classification + Windows TPM signature binding | 1. Virtual-camera blocking (`v4l2loopback`, Windows `SWD\`/OBS, Android `EXTERNAL`) and built-in camera priority in every OS shell. 2. A `--capture-source` parameter for `rise-core sign`/`video`, and `rise-auto-seal.py` tagging recordings as `"obs"`. 3. The TPM P-256 key in Windows `attest.cpp` actually signing the manifest via `NCryptSignHash` (closing the F7 finding). | - With `v4l2loopback` or OBS Virtual Camera active, the capture tools must exit with `ERR_VIRTUAL_CAMERA_REJECTED` (exit code != 0). - An `.rvid` sealed with OBS must return `capture_source: "obs"` and a screen/software warning from verify-web (pytest test). - On Windows, no manifest produced with a non-TPM PEM key may earn gold-L2. |
| **P1 (3–6 weeks):** mobile hardware attestation chain + server nonce + IMU correlation | 1. Wire the Android `RiseKeystore.kt` X.509 certificate chain (`getCertificateChain`) + Play Integrity token verification into `verify-web/trust.py`. 2. iOS `Attest.swift` Secure Enclave + `DCAppAttestService` assertion verification. 3. Collect a 500 ms IMU (gyro) window at capture time and embed the video/burst optical-flow correlation score (`imu_flow_corr`) into the manifest. | - A Key Attestation chain produced on a bootloader-unlocked or emulated Android device must be downgraded to silver or red by verify-web. - While filming a video on a static screen with a handheld phone, the hand-tremor (IMU) ↔ in-video optical-flow mismatch must return `scene_liveness: "suspicious"`. - Signing attempts with an old (replayed) nonce must be rejected. |
| **P2 (2–3 months):** dual-lens stereo depth (disparity) + two-axis badge UX | 1. Simultaneous dual-frame or depth-map capture at the shutter via iOS (`AVCaptureMultiCamSession` / `AVCaptureDepthDataOutput`) and Android (`LogicalMultiCamera` Wide+UltraWide). 2. A RANSAC planarity test (`planarity_score`) checking whether the scene is a 2D plane (screen/paper print). 3. Moving the verify-web UI to the two-axis badge (Pipeline Provenance and Scene Liveness). | - When a flat monitor or paper print is photographed, `planarity_score > 0.92` must make the `scene_liveness` field return `"flat_surface_warning"` and deny 3D Scene Approval. - In a real 3D-depth scene (<3 metres) both the Gold-L2 and `3D Scene Consistent` badges must be produced. |

---

## 5. Mobile Reality Check: What Works in Practice, What Stays on Paper

Realistic engineering and UX assessment of the three mechanisms proposed in
`spec/solutions-physical.md` on ordinary consumer phones (standard iPhone and Android devices):

| Proposal | Mobile reality | Why? (UX, hardware and cost analysis) |
|---|---|---|
| **1. Encrypted MIPI + hash at sensor output (Gold-L4)** | **Stays 100% on paper** (impossible for a third-party app) | On standard iOS and Android phones, the MIPI CSI-2 data path between the camera sensor and the SoC ISP is hardware-closed and managed by the OS kernel/ISP firmware. No App Store / Play Store app can reach the MIPI bus, cryptographically verify the sensor's hardware serial number, or take a hash inside the TEE before the ISP. This is possible only in Qualcomm/Apple's own camera firmware or in C2PA-equipped dedicated cameras from Leica/Sony/Nikon. **Decision:** remove the Gold-L4 tier from the mobile-app scope and mark it "future OEM/hardware-vendor integration". |
| **2. Nonce light challenge (flash / screen burst)** | **Stays on paper for general capture; works only in a <1.5 m indoor niche** | **Physical barrier:** in outdoor daylight (10,000–100,000 lux) or on subjects more than 2 metres away, the phone's LED flash cannot create a measurable reflection difference on the scene because of the inverse-square law (1/r²); it falls below the noise floor. **UX barrier:** firing a random flash for 300–600 ms on every photo/video capture ruins natural shooting, disturbs people and drains the battery. **Decision:** do not make it mandatory in the general photo/video mode (drop the "mandatory on every capture" rule in `solutions-physical.md`). Make it an optional high-assurance step only for a "close-range document / damage / object verification mode" (<1 metre, e.g. insurance vehicle damage or meter reading). |
| **3. Depth + IMU parallax check** | **Works in practice** (the highest-leverage mobile solution) | A dedicated LiDAR sensor exists only on Pro iPhones, and ToF on very few Android phones; but on almost all modern smartphones a main + ultra-wide dual camera and a gyroscope (IMU) are standard. When the user presses the shutter, doing (1) simultaneous two-frame capture from the main and ultra-wide cameras and computing stereo disparity (parallax), and (2) comparing the 300 ms of natural hand micro-motion at capture time with the optical flow — with zero UX friction — is fully writable today with standard iOS (AVFoundation + CoreMotion) and Android (Camera2 + SensorManager) APIs. |

---

## 6. P0 Implementation Status (core path — DONE)

The capture class is now recorded, signature-covered and verifier-labelled. Verified locally:

| Change | File | Evidence |
|---|---|---|
| `--capture-source <camera\|screen\|obs\|virtual\|external_usb>` on `sign` and `video`; unknown value = hard error (exit 2) | `core/src/main.rs` (`normalize_capture_source`, `SignArgs`, `VideoArgs`) | `rise-core sign --capture-source bogus` → rc 2; `--capture-source obs` → manifest carries `capture_source: "obs"` |
| The value is set **before** stamp/sign, so it is covered by the signature | `core/src/main.rs` (`sign_cmd`, `video_cmd`) | tampering `obs`→`camera` after signing → `INVALID: invalid signature` (rc 3) |
| OBS recordings are labelled | `bindings/obs/rise-auto-seal.py` | `--capture-source obs` added to the `rise-core video` invocation |
| Non-camera classes carry a disclaimer in the verifier | `verify-web/verify.py` (`_NON_CAMERA_SOURCES`, `_ok`) | verify-web returns badge `silver` + `capture_source: "obs"` + `capture_note` ("…does NOT prove the pixels came from a real camera sensor.") |
| Regression tests | `verify-web/tests/test_badge.py` | `test_obs_capture_labeled_not_camera`, `test_virtual_and_external_usb_labeled` |
| **Rank 1 source gate** — each shell refuses a virtual/loopback source before it can become evidence | `bindings/{ios,macos}/capture/*.swift`, `bindings/android/capture/RiseCapture.kt`, `bindings/windows/capture/capture.cpp`, `bindings/linux/capture/cam_class.sh` | macOS: live enumeration on the author machine rejected an installed OBS Virtual Camera while accepting the built-in lens (probe rc 0), `swiftc -typecheck` rc 0; iOS `swiftc -typecheck` rc 0; Android compile-verified against android-35 (kotlinc 2.3.20 + JRE 26.0.1, rc 0, 31 classes for the whole module — the run caught and fixed an `isNullOrEmpty()` misuse on `FloatArray?`) and its capture path was then **driven end-to-end on a real phone** (photo + 14-frame video seals, both `capture_source:"camera"`, re-verified on the Mac); Linux `test_cam_class.sh` 6/6 (virtual → exit 10); **the Windows edit is static-only** — no MSVC on this machine |
| Gold needs a hardware-backed key **declared in the pinned registry**, never in the manifest | `verify-web/verify.py` (`hardware_backing`, `_HARDWARE_BACKINGS`) | a manifest self-claim of `"hardware_backed": true` cannot raise the backing (fail-closed to software); 82 pytest tests pass |
| Windows fault facts stay honest | `bindings/windows/attest/attest.cpp` | VBS is reported as *enablement requested*, and no surface may read it as running (`running unverified`); the TPM signature/quote producer is explicitly absent |

**Still open under P0:** the Windows TPM `NCryptSignHash` binding plus the EK/AIK cert chain and PCR/VBS quote emission (F7 — `tpm_ecc_signature` / `tpm_cert_chain` / `tpm_quote` currently have no producer, so no surface may claim a TPM-bound signature). The Rank 1 per-OS source gate is implemented: macOS/iOS are typechecked (macOS additionally proven by a live enumeration); Android is compile-verified and its capture path was driven end-to-end on a real phone (photo + 14-frame video seals, both `capture_source:"camera"`; the Mac re-derived 14/14 frame hashes, 56/56 tiles and the Merkle root, and the ES256 signature verifies `Verified OK` under `video-001.pub`); but the **Windows leg is static-only** (no MSVC on the author machine) and **no leg has yet been driven against a hostile virtual source** — the phone exposes no LEGACY/EXTERNAL camera, macOS was probed at enumeration level only, and Linux's `cam_class` test is synthetic.

---

## References

- `spec/RISE-01.md` §5.1 (capture_source classes + analog-hole limit)
- `spec/threat-model.md` #2 (screen re-capture), #1 (fake file), #4 (after-the-fact edit)
- `spec/solutions-physical.md` #1, #5, #7, #8, #10, #11
- `spec/solutions-digital.md` #8 (screenshot laundering)
- `spec/solutions-root.md` §1 (supply chain), §2 (OS), §5 (golden rules)
- `PLAN.md` §7 (Risks: "Screen re-capture", "Fake USB/MIPI source")
- `REDTEAM_FINDINGS.md` (F7 TPM binding; Round 4 A2 moiré)
- `bindings/obs/rise-auto-seal.py`, `core/src/main.rs` (`screen_cmd`, `video_cmd`),
  `core/src/lib.rs`, `verify-web/verify.py`, `verify-web/trust.py`
