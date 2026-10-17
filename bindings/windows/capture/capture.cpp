// capture.cpp — Windows single-frame capture (MediaFoundation), thin shell.
//
// Status: REAL - compiled and run on DESKTOP-1LD8TK1 (2026-10-10). Real
// capture: HP True Vision HD Camera, YUY2 1280x720, 1843200 bytes, hashed
// on the PC by rise-core: 3f6515750f79e855e19c084675bca0d2e7cd1f491e1af090f5296836e2197f62.
// Gotcha: MF ReadSample returns E_POINTER when optional out-params are NULL;
// always pass real ones (see settle/shutter calls).
// (updated after first real run — see MATRIX.md).
//
// Thin shell: captures one frame, writes raw bytes + a .meta sidecar for
// core/ to hash. No signing logic here. Never uploads anywhere.
//
// Build (VS x64 prompt or vcvars + cl):
//   cl /EHsc /std:c++17 capture.cpp
// Run:
//   capture.exe [output-path]   (default: ..\..\target\frame-001.yuy2)
//
// Source policy (Rank 1): a virtual/software source is REJECTED, never
// captured — OBS Virtual Camera can replay any file, so it must not enter a
// capture flow. An integrated camera is preferred over an external USB one;
// an external source stays silver-capped.
// Preferred type YUY2 1280x720; on rejection the device's native type is
// used and the actual subtype/stride recorded in the sidecar. MJPEG/H264
// are NOT decoded — hash the raw bytes at shutter, no re-encode before hashing.

#include <windows.h>
#include <mfapi.h>
#include <mfidl.h>
#include <mfreadwrite.h>

#include <algorithm>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

#pragma comment(lib, "ole32.lib")
#pragma comment(lib, "mfplat.lib")
#pragma comment(lib, "mfreadwrite.lib")
#pragma comment(lib, "mfuuid.lib")
#pragma comment(lib, "mf.lib")

namespace {

constexpr uint32_t kWidth = 1280;
constexpr uint32_t kHeight = 720;

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

// True when the FriendlyName matches a known virtual/software capture product.
// Deliberately name-based rather than driver-based: it over-rejects rather than
// trusting a loopback feed.
bool IsVirtualName(const std::string& friendly_name) {
  static const char* const kVirtualNames[] = {
      "obs virtual camera", "virtual camera",   "droidcam",      "iriun",
      "epoccam",            "manycam",         "screen capture", "snap camera",
      "ndi"};
  return std::any_of(std::begin(kVirtualNames), std::end(kVirtualNames),
                     [&](const char* needle) {
                       return ContainsCi(friendly_name, needle);
                     });
}

// True when the symbolic link enumerates on a loopback bus (ROOT/SWD) rather
// than a hardware bus. Fail closed only when the link is actually present.
bool IsLoopbackLink(const std::string& symbolic_link) {
  if (symbolic_link.empty()) return false;
  return _strnicmp(symbolic_link.c_str(), "\\\\?\\root#", 9) == 0 ||
         _strnicmp(symbolic_link.c_str(), "\\\\?\\swd#", 9) == 0;
}

// True when the source is a software/loopback device rather than hardware.
bool IsVirtualSource(const std::string& friendly_name,
                     const std::string& symbolic_link) {
  return IsVirtualName(friendly_name) || IsLoopbackLink(symbolic_link);
}

std::string Narrow(const wchar_t* w) {
  if (w == nullptr) return std::string();
  int n = WideCharToMultiByte(CP_UTF8, 0, w, -1, nullptr, 0, nullptr, nullptr);
  if (n <= 1) return std::string();
  std::string s(static_cast<size_t>(n - 1), '\0');
  WideCharToMultiByte(CP_UTF8, 0, w, -1, &s[0], n, nullptr, nullptr);
  return s;
}

const char* SubtypeName(const GUID& g) {
  if (IsEqualGUID(g, MFVideoFormat_YUY2)) return "YUY2";
  if (IsEqualGUID(g, MFVideoFormat_NV12)) return "NV12";
  if (IsEqualGUID(g, MFVideoFormat_MJPG)) return "MJPG";
  return "other";
}

struct FrameMeta {
  const char* subtype;
  uint32_t width;
  uint32_t height;
  uint32_t stride;
  bool usb;
};

// Writes raw bytes + a <path>.meta sidecar (subtype, size, source info).
// core/ hashes the raw file only; the sidecar is informational.
bool WriteFrame(const char* path, const uint8_t* bytes, size_t len,
                const FrameMeta& meta, const std::string& source) {
  FILE* f = std::fopen(path, "wb");
  if (f == nullptr) return false;
  const size_t n = std::fwrite(bytes, 1, len, f);
  std::fclose(f);
  if (n != len) return false;
  std::string meta_path = std::string(path) + ".meta";
  FILE* m = std::fopen(meta_path.c_str(), "w");
  if (m != nullptr) {
    std::fprintf(m, "subtype=%s\nwidth=%u\nheight=%u\nstride=%u\nbytes=%zu\n",
                 meta.subtype, meta.width, meta.height, meta.stride, len);
    std::fprintf(m, "source=%s\ncapture_source=%s\n", source.c_str(),
                 meta.usb ? "external_usb" : "camera");
    std::fclose(m);
  }
  return true;
}

struct SourceChoice {
  UINT32 index = 0;
  std::string name;
  bool found = false;
  bool usb = false;
  bool saw_virtual = false;
};

std::string DeviceName(IMFActivate* act) {
  LPWSTR wname = nullptr;
  UINT32 wlen = 0;
  if (FAILED(act->GetAllocatedString(MF_DEVSOURCE_ATTRIBUTE_FRIENDLY_NAME,
                                     &wname, &wlen))) {
    return std::string();
  }
  std::string name = Narrow(wname);
  CoTaskMemFree(wname);
  return name;
}

std::string DeviceLink(IMFActivate* act) {
  LPWSTR wlink = nullptr;
  UINT32 llen = 0;
  if (FAILED(act->GetAllocatedString(
          MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK, &wlink,
          &llen))) {
    return std::string();
  }
  std::string link = Narrow(wlink);
  CoTaskMemFree(wlink);
  return link;
}

enum class DeviceKind { kSkip, kIntegrated, kUsb, kVirtual };

// Classifies one enumerated device (name + bus) for the source gate.
DeviceKind ConsiderDevice(IMFActivate* act, UINT32 index, std::string* name) {
  *name = DeviceName(act);
  if (name->empty()) return DeviceKind::kSkip;
  if (IsVirtualSource(*name, DeviceLink(act))) {
    std::fprintf(stderr, "camera[%u]=%s REJECTED virtual/software source\n",
                 index, name->c_str());
    return DeviceKind::kVirtual;
  }
  const bool usb = IsUsbSource(*name);
  std::fprintf(stderr, "camera[%u]=%s usb=%d\n", index, name->c_str(),
               usb ? 1 : 0);
  return usb ? DeviceKind::kUsb : DeviceKind::kIntegrated;
}

// Source gate: integrated camera first, external USB second, virtual never.
SourceChoice ChooseSource(IMFActivate** acts, UINT32 count) {
  SourceChoice choice;
  for (UINT32 i = 0; i < count; ++i) {
    std::string name;
    const DeviceKind kind = ConsiderDevice(acts[i], i, &name);
    if (kind == DeviceKind::kVirtual) choice.saw_virtual = true;
    if (kind == DeviceKind::kIntegrated) {
      choice.index = i;
      choice.name = name;
      choice.found = true;
      break;
    }
    if (kind == DeviceKind::kUsb && !choice.found) {
      choice.index = i;
      choice.name = name;
      choice.found = true;
      choice.usb = true;
    }
  }
  return choice;
}

}  // namespace

int main(int argc, char** argv) {
  const char* out = (argc > 1) ? argv[1] : "..\\..\\target\\frame-001.yuy2";

  if (FAILED(CoInitializeEx(nullptr, COINIT_MULTITHREADED))) {
    std::fprintf(stderr, "ERROR: COM init failed\n");
    return 2;
  }
  if (FAILED(MFStartup(MF_VERSION, MFSTARTUP_LITE))) {
    std::fprintf(stderr, "ERROR: MFStartup failed\n");
    CoUninitialize();
    return 2;
  }

  // Enumerate video capture devices.
  IMFAttributes* attrs = nullptr;
  IMFActivate** acts = nullptr;
  UINT32 count = 0;
  if (FAILED(MFCreateAttributes(&attrs, 1)) ||
      FAILED(attrs->SetGUID(MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE,
                            MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID)) ||
      FAILED(MFEnumDeviceSources(attrs, &acts, &count)) || count == 0) {
    std::fprintf(stderr, "NO_CAMERA: no video capture devices found\n");
    if (acts != nullptr) CoTaskMemFree(acts);
    if (attrs != nullptr) attrs->Release();
    MFShutdown();
    CoUninitialize();
    return 2;
  }

  // Source gate: integrated camera first, external USB second, virtual never.
  SourceChoice choice = ChooseSource(acts, count);
  if (!choice.found) {
    if (choice.saw_virtual) {
      std::fprintf(stderr,
                   "ERR_VIRTUAL_CAMERA_REJECTED: only virtual sources found\n");
    } else {
      std::fprintf(stderr, "NO_CAMERA: devices enumerated but none usable\n");
    }
    CoTaskMemFree(acts);
    attrs->Release();
    MFShutdown();
    CoUninitialize();
    return 2;
  }
  const UINT32 pick = choice.index;
  const bool usb_source = choice.usb;
  std::fprintf(stderr, "picked=%u (%s) usb=%d\n", pick, choice.name.c_str(),
               usb_source ? 1 : 0);

  // Activate the source and open a reader (stage-logged for diagnosis).
  IMFMediaSource* src = nullptr;
  IMFSourceReader* reader = nullptr;
  IMFMediaType* type = nullptr;
  bool written = false;
  HRESULT hr = acts[pick]->ActivateObject(IID_IMFMediaSource, (void**)&src);
  std::fprintf(stderr, "activate=0x%08lX\n", (unsigned long)hr);
  if (SUCCEEDED(hr)) {
    hr = MFCreateSourceReaderFromMediaSource(src, nullptr, &reader);
    std::fprintf(stderr, "reader=0x%08lX\n", (unsigned long)hr);
  }
  if (SUCCEEDED(hr)) {
    if (SUCCEEDED(MFCreateMediaType(&type))) {
      (void)type->SetGUID(MF_MT_MAJOR_TYPE, MFMediaType_Video);
      (void)type->SetGUID(MF_MT_SUBTYPE, MFVideoFormat_YUY2);
      (void)MFSetAttributeSize(type, MF_MT_FRAME_SIZE, kWidth, kHeight);
      hr = reader->SetCurrentMediaType(
          MF_SOURCE_READER_FIRST_VIDEO_STREAM, nullptr, type);
      std::fprintf(stderr, "settype=0x%08lX\n", (unsigned long)hr);
      type->Release();
      type = nullptr;
      if (FAILED(hr)) {
        std::fprintf(stderr, "note: YUY2 rejected, using native type\n");
      }
    }
    hr = reader->GetCurrentMediaType(MF_SOURCE_READER_FIRST_VIDEO_STREAM,
                                     &type);
    std::fprintf(stderr, "gettype=0x%08lX\n", (unsigned long)hr);
    if (SUCCEEDED(hr)) {
      GUID sub{};
      UINT32 w = 0;
      UINT32 h = 0;
      (void)type->GetGUID(MF_MT_SUBTYPE, &sub);
      (void)MFGetAttributeSize(type, MF_MT_FRAME_SIZE, &w, &h);
      const UINT32 stride =
          MFGetAttributeUINT32(type, MF_MT_DEFAULT_STRIDE, 0);

      // Let exposure settle: throw away the first frames.
      // Some MF sources return E_POINTER when optional out-params are NULL,
      // so every ReadSample passes real out-params.
      DWORD sidx = 0;
      DWORD sflags = 0;
      LONGLONG sts = 0;
      for (int i = 0; i < 10; ++i) {
        IMFSample* drop = nullptr;
        sflags = 0;
        hr = reader->ReadSample(MF_SOURCE_READER_FIRST_VIDEO_STREAM, 0, &sidx,
                                &sflags, &sts, &drop);
        if (FAILED(hr)) {
          std::fprintf(stderr, "settle[%d]=0x%08lX\n", i, (unsigned long)hr);
          break;
        }
        if (drop != nullptr) {
          drop->Release();
        }
      }

      // Shutter: read the capture frame.
      IMFSample* sample = nullptr;
      sflags = 0;
      hr = reader->ReadSample(MF_SOURCE_READER_FIRST_VIDEO_STREAM, 0, &sidx,
                              &sflags, &sts, &sample);
      std::fprintf(stderr, "read=0x%08lX sample=%d flags=0x%08lX\n",
                   (unsigned long)hr, sample != nullptr ? 1 : 0,
                   (unsigned long)sflags);
      if (SUCCEEDED(hr) && sample != nullptr) {
        // IMFMediaBuffer owns Lock/Unlock; IMFSample does not.
        IMFMediaBuffer* buf = nullptr;
        if (SUCCEEDED(sample->ConvertToContiguousBuffer(&buf))) {
          BYTE* ptr = nullptr;
          DWORD maxb = 0;
          DWORD cur = 0;
          if (SUCCEEDED(buf->Lock(&ptr, &maxb, &cur))) {
            const FrameMeta meta{SubtypeName(sub), w, h, stride, usb_source};
            written = WriteFrame(out, ptr, cur, meta, choice.name);
            std::fprintf(stderr, "write=%d bytes=%lu\n", written ? 1 : 0,
                         (unsigned long)cur);
            buf->Unlock();
          } else {
            std::fprintf(stderr, "lock=failed\n");
          }
          buf->Release();
        } else {
          std::fprintf(stderr, "convert=failed\n");
        }
        sample->Release();
      }
    }
  }

  if (written) {
    std::fprintf(stderr, "WROTE: %s\n", out);
  } else {
    std::fprintf(stderr, "ERROR: frame not captured\n");
  }

  if (type != nullptr) type->Release();
  if (reader != nullptr) reader->Release();
  if (src != nullptr) src->Release();
  for (UINT32 i = 0; i < count; ++i) acts[i]->Release();
  CoTaskMemFree(acts);
  attrs->Release();
  MFShutdown();
  CoUninitialize();
  return written ? 0 : 2;
}
