---
title: "Market Map"
type: research
status: active
updated: 2026-08-15
canonical: true
tags:
  - market-map
---

# Market map

Conceptual axes:

- horizontal: format-first -> destination-first
- vertical: manual recipe -> automatic adaptation

```text
                    automatic
                       ↑
                       |
        Cloudinary     | Android media
        Smart Converter| calibre
        Clop           |
                       |       SHOEHORN TARGET
-----------------------+--------------------------→ destination-first
                       |
    CloudConvert       | HandBrake preset
    VERT               |
    Convert.to.it      |
    File Converter     |
                       |
                     manual
```

## Groups

### Consumer conversion
VERT, CloudConvert, Convert.to.it, File Converter.

### Smart local media
Smart Converter, Clop, HandBrake.

### Vertical automatic compatibility
Android, calibre, CMS/media internals.

### Developer file infrastructure
Filestack, Uploadcare, Transloadit, ConvertAPI, CloudConvert API.

### Delivery adaptation
Cloudinary.

### Engines
FFmpeg, ImageMagick, libvips and document/PDF engines.

## Opportunity

Shoehorn is the **horizontalization** of adaptation behavior that vertical systems repeatedly implement.

That is a better market thesis than “nobody has thought of automatic conversion.”
