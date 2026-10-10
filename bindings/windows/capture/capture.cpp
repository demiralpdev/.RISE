// capture.cpp — Windows single-frame capture sketch (MediaFoundation).
//
// HONESTY HEADER: NOT compiled, NOT tested — STUB. No Windows toolchain on
// this machine (static review only). Thin shell: captures a frame, writes raw
// bytes for core/ to hash. No signing logic here.
//
// Build (on Windows, VS Developer Prompt):
//   cl /EHsc /std:c++17 capture.cpp mfplat.lib mfreadwrite.lib mfuuid.lib
// Run:
//   capture.exe [output-path]
//   Default output: ..\..\target\frame-001.yuy2
//
// Device priority: a source whose FriendlyName contains "USB" first
// (external webcam, silver tier by default), else the first video source.
//
// Webcam capability note: this sketch requests MFVideoFormat_YUY2 at
// 1280x720. If the device rejects the type, IMFSourceReader falls back to the
// device's native type and the writer records the actual subtype + stride in
// the sidecar <output>.meta file so core/ hashes exactly the bytes written.
// YUY2 is chosen because it is the widest-supported raw-ish type; MJPEG/H264
// outputs are deliberately NOT decoded here (hash the raw frame at shutter,
// no re-encode before hashing).
//
// Data flow: shutter -> IMFSourceReader::ReadSample -> raw bytes -> file.
// core/ then runs: rise-core hash <file>.

#include <cstdint>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

// --- Windows/MediaFoundation includes (Windows-only; see honesty header) ---
// #include <windows.h>
// #include <mfapi.h>
// #include <mfidl.h>
// #include <mfreadwrite.h>
// #include <mferror.h>
// #include <combaseapi.h>

namespace {

// Preferred capture parameters.
constexpr uint32_t kWidth = 1280;
constexpr uint32_t kHeight = 720;
// MFVideoFormat_YUY2 GUID data1 field (full GUID in mfapi.h).
constexpr const char* kPreferredSubtype = "YUY2";

// Case-insensitive substring match (ASCII-only, device names only).
bool ContainsCi(const std::string& haystack, const char* needle) {
  if (haystack.empty() || needle == nullptr || needle[0] == '\0') return false;
  const size_t nlen = std::strlen(needle);
  for (size_t i = 0; i + nlen <= haystack.size(); ++i) {
    size_t j = 0;
    for (; j < nlen; ++j) {
      const char a = haystack[i + j];
      const char b = needle[j];
      const char la = (a >= 'A' && a <= 'Z') ? static_cast<char>(a + 32) : a;
      const char lb = (b >= 'A' && b <= 'Z') ? static_cast<char>(b + 32) : b;
      if (la != lb) break;
    }
    if (j == nlen) return true;
  }
  return false;
}

// True when the source FriendlyName suggests an external USB webcam.
// USB sources cap the tier at silver (untrusted transport, no sensor path).
bool IsUsbSource(const std::string& friendly_name) {
  return ContainsCi(friendly_name, "USB") || ContainsCi(friendly_name, "UVC") ||
         ContainsCi(friendly_name, "Rapoo");
}

// Picks a source index: USB FriendlyName first, else index 0.
// Real implementation enumerates MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID
// activation objects, reads MF_DEVSOURCE_ATTRIBUTE_FRIENDLY_NAME, and returns
// the IMFActivate* for IMFSourceReader::Create. Kept as pure index logic here
// so the priority rule is reviewable without a Windows SDK.
int PickSourceIndex(const std::vector<std::string>& friendly_names) {
  if (friendly_names.empty()) return -1;
  for (size_t i = 0; i < friendly_names.size(); ++i) {
    if (IsUsbSource(friendly_names[i])) return static_cast<int>(i);
  }
  return 0;
}

// Writes raw bytes + a <path>.meta sidecar (subtype, stride, size).
// core/ hashes the raw file only; the sidecar is informational.
bool WriteFrame(const char* path, const uint8_t* bytes, size_t len,
                const char* subtype, uint32_t stride) {
  FILE* f = std::fopen(path, "wb");
  if (f == nullptr) return false;
  const size_t n = std::fwrite(bytes, 1, len, f);
  std::fclose(f);
  if (n != len) return false;
  std::string meta_path = std::string(path) + ".meta";
  FILE* m = std::fopen(meta_path.c_str(), "w");
  if (m != nullptr) {
    std::fprintf(m, "subtype=%s\nwidth=%u\nheight=%u\nstride=%u\nbytes=%zu\n",
                 subtype, kWidth, kHeight, stride, len);
    std::fclose(m);
  }
  return true;
}

}  // namespace

// Real ReadSample flow (reference; requires Windows SDK):
//
//   MFStartup(MF_VERSION);
//   IMFSourceReader* reader = nullptr;
//   MFCreateSourceReaderFromMediaSource(activate, nullptr, &reader);
//   IMFMediaType* type = nullptr;
//   MFCreateMediaType(&type);
//   type->SetGUID(MF_MT_MAJOR_TYPE, MFMediaType_Video);
//   type->SetGUID(MF_MT_SUBTYPE, MFVideoFormat_YUY2);
//   MFSetAttributeSize(type, MF_MT_FRAME_SIZE, 1280, 720);
//   if (FAILED(reader->SetCurrentMediaType(MF_SOURCE_READER_FIRST_VIDEO_STREAM,
//                                          nullptr, type))) {
//     // Fallback: keep the device native type; record actual subtype.
//     reader->GetCurrentMediaType(MF_SOURCE_READER_FIRST_VIDEO_STREAM, &type);
//   }
//   // Let exposure settle (~1s), then:
//   IMFSample* sample = nullptr;
//   reader->ReadSample(MF_SOURCE_READER_FIRST_VIDEO_STREAM, 0,
//                      nullptr, nullptr, nullptr, &sample);
//   // Lock IMFMediaBuffer, copy bytes, WriteFrame(...), MFShutdown();

int main(int argc, char** argv) {
  const char* out = (argc > 1) ? argv[1] : "..\\..\\target\\frame-001.yuy2";
  std::fprintf(stderr,
               "STUB: capture.cpp was not compiled or tested on Windows.\n"
               "Priority rule: USB FriendlyName first, else first source.\n"
               "Preferred type: %s %ux%u. Output would be: %s\n",
               kPreferredSubtype, kWidth, kHeight, out);
  return 2;
}
