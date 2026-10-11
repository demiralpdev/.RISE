# System Format Registration

Registers `.rise` as a first-class system format — the same way installing a
codec makes a new media type visible to every application. After install, the
OS itself knows what a `.rise` file is: file managers show proper icons and
descriptions, media viewers open it directly, and search indexes it under the
right type.

`.rise` files remain byte-compatible JPEG (image) or MP4 (video) — the
registration only teaches the OS to recognize the extension as a sealed
variant of those base types.

## macOS

```bash
./macos/install.sh
```

Installs `RISE Format.app` to `/Applications` and registers two UTIs with
LaunchServices:

| Extension | UTI | Conforms to | MIME |
|---|---|---|---|
| `.rise` | `org.rise.sealed-image` | `public.jpeg`, `public.image` | `image/jpeg` |
| `.rvid` | `org.rise.sealed-video` | `public.mpeg-4`, `public.movie` | `video/mp4` |

Verified result — a `.rise` file now reports:

```
kMDItemContentType     = "org.rise.sealed-image"
kMDItemContentTypeTree = (
    "public.item", "public.content", "public.image",
    "public.data", "public.jpeg", "org.rise.sealed-image"
)
```

Practical effect: Finder treats `.rise` as an image, Preview opens it,
Spotlight indexes it, and any app that builds its format lists from system
UTIs will offer it.

## Linux

```bash
sudo ./linux/install.sh
```

Writes a shared-mime-info entry (`/usr/share/mime/packages/rise.xml`) and
refreshes the MIME database. `.rise` then resolves to `image/jpeg` /
`video/mp4` in every freedesktop-compliant file manager, dialog, and
application.

## Windows

```powershell
# run as Administrator
./windows/install.ps1
```

Registry entries under `HKLM:\SOFTWARE\Classes`:

- `.rise` → `PerceivedType = image`, `ContentType = image/jpeg`
- `RISE.SealedFile` → file type description, open verb
- Explorer `KindMap` → `.rise` kind = picture

Explorer then shows `.rise` files as pictures, and the property system treats
them as image content.

## Limits

OS registration covers everything that reads formats from the system. Apps
with **hardcoded** format lists (e.g. OBS Studio's recorder format dropdown)
do not read system format lists and need per-app integration — see
`bindings/obs/`. This is the same for every codec: installing ProRes on macOS
does not add it to an app's private dropdown either.
