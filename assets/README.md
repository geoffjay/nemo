# Assets

This directory contains assets for building the macOS `.app` bundle.

## Files

- `Info.plist.template` - macOS app manifest template (version placeholder replaced at build time)
- `entitlements.plist` - Code signing entitlements for sandboxed distribution
- `AppIcon.icns` - Application icon (you need to create this)

## Creating AppIcon.icns

To create the app icon, you need a 1024x1024 PNG image. Then use one of these methods:

### Using iconutil (built-in)

```bash
# Create iconset directory
mkdir AppIcon.iconset

# Create required sizes (from your 1024x1024 source)
sips -z 16 16     icon_1024.png --out AppIcon.iconset/icon_16x16.png
sips -z 32 32     icon_1024.png --out AppIcon.iconset/icon_16x16@2x.png
sips -z 32 32     icon_1024.png --out AppIcon.iconset/icon_32x32.png
sips -z 64 64     icon_1024.png --out AppIcon.iconset/icon_32x32@2x.png
sips -z 128 128   icon_1024.png --out AppIcon.iconset/icon_128x128.png
sips -z 256 256   icon_1024.png --out AppIcon.iconset/icon_128x128@2x.png
sips -z 256 256   icon_1024.png --out AppIcon.iconset/icon_256x256.png
sips -z 512 512   icon_1024.png --out AppIcon.iconset/icon_256x256@2x.png
sips -z 512 512   icon_1024.png --out AppIcon.iconset/icon_512x512.png
sips -z 1024 1024 icon_1024.png --out AppIcon.iconset/icon_512x512@2x.png

# Convert to icns
iconutil -c icns AppIcon.iconset -o AppIcon.icns
rm -rf AppIcon.iconset
```

### Using makeicns (if installed)

```bash
makeicns -in icon_1024.png -out AppIcon.icns
```

Place the resulting `AppIcon.icns` in this directory.
