# Electron Shared Texture Test

This test demonstrates how to use Electron's offscreen rendering with `SharedTexture` API and render the output using `myde-display` DRM rendering.

## Overview

The test creates:
1. An **offscreen BrowserWindow** that renders a webpage using GPU-accelerated shared texture
2. A **display BrowserWindow** that shows the rendering status and controls
3. Integration with **myde-display** to render the texture to a DRM display

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Main Process                           │
│                                                             │
│  ┌─────────────────┐      ┌─────────────────────────────┐  │
│  │  OSR Window     │      │  myde-display               │  │
│  │  (offscreen)    │─────▶│  - DrmDevice                │  │
│  │                 │      │  - SharedTexture            │  │
│  │  webPreferences:│      │  - TransformUtil            │  │
│  │    offscreen:   │      │                             │  │
│  │      useShared  │      │  Renders to /dev/dri/card0  │  │
│  │      Texture:   │      └─────────────────────────────┘  │
│  │      true       │                                        │
│  └─────────────────┘                                        │
│           │                                                 │
│           │ paint event with texture                        │
│           ▼                                                 │
│  ┌─────────────────┐                                        │
│  │ Display Window  │                                        │
│  │ (shows status)  │                                        │
│  └─────────────────┘                                        │
└─────────────────────────────────────────────────────────────┘
```

## Prerequisites

- Linux with DRM support
- Electron 43.0.0
- Node.js 18+
- Access to `/dev/dri/card*` (video group)

## Installation

```bash
cd test/electron-shared-texture
npm install
```

## Running

```bash
npm start
```

## How It Works

### 1. Offscreen Rendering with Shared Texture

```javascript
const osrWindow = new BrowserWindow({
  webPreferences: {
    offscreen: {
      useSharedTexture: true  // Enable GPU shared texture
    }
  }
});
```

### 2. Paint Event Handler

```javascript
osrWindow.webContents.on('paint', (event, dirty, texture) => {
  // texture.textureInfo contains the SharedTextureImportTextureInfo
  // texture.textureInfo.handle contains SharedTextureHandle
  // texture.textureInfo.handle.nativePixmap contains NativePixmap (Linux)
});
```

### 3. NativePixmap Structure (Linux)

```javascript
{
  planes: [
    {
      stride: number,  // Stride in bytes
      offset: number,  // Offset in bytes
      size: number,    // Plane size in bytes
      fd: number       // DMA-BUF file descriptor
    }
  ],
  modifier: string,    // DRM format modifier
  supportsZeroCopyWebGpuImport: boolean
}
```

### 4. Rendering with myde-display

```javascript
const { DrmDevice, SharedTexture, TransformUtil } = require('myde-display');

// Create DRM device
const device = new DrmDevice('/dev/dri/card0');

// Create SharedTexture from Electron's texture info
const texture = SharedTexture.fromNativePixmap(
  textureInfo.handle.nativePixmap,
  textureInfo.codedSize.width,
  textureInfo.codedSize.height,
  textureInfo.pixelFormat
);

// Apply transformations
const transform = TransformUtil.rotation(Math.PI / 4);

// Render to DRM display
device.renderSharedTexture(texture.getTextureInfo(), transform);
```

## Supported Pixel Formats

- `bgra` - 32bpp BGRA (byte-order), 1 plane
- `rgba` - 32bpp RGBA (byte-order), 1 plane
- `rgbaf16` - Half float RGBA, 1 plane
- `nv12` - 12bpp with Y plane followed by a 2x2 interleaved UV plane
- `nv16` - 16bpp with Y plane followed by a 2x1 interleaved UV plane
- `p010le` - 4:2:0 10-bit YUV (little-endian)

## Transformations

The `TransformUtil` supports:

- **Rotation**: Arbitrary angle rotation (not just 90° multiples)
- **Scaling**: Non-uniform scaling (different X/Y factors)
- **Translation**: Moving the texture position
- **Composition**: Combining multiple transforms

## Controls

The display window provides buttons to:
- Rotate texture 45°
- Scale texture 1.5x
- Reset transform
- Get screen information

## Troubleshooting

### No Texture Received

If the paint event doesn't receive a texture:
1. Ensure GPU acceleration is available
2. Check that `useSharedTexture: true` is set
3. Verify the webpage is loading correctly

### DRM Rendering Fails

If DRM rendering fails:
1. Check permissions on `/dev/dri/card*`
2. Ensure a display is connected
3. Verify the pixel format is supported

### Performance Issues

- Reduce frame rate: `osrWindow.webContents.setFrameRate(30)`
- Use simpler webpages for testing
- Ensure the DRM device supports the requested resolution

## Notes

- The offscreen window is always created as frameless
- Only the dirty area is passed to the paint event
- When nothing is happening on the webpage, no frames are generated
- The shared texture must be released after use

## References

- [Electron Offscreen Rendering](https://www.electronjs.org/docs/latest/tutorial/offscreen-rendering)
- [Electron SharedTexture API](https://github.com/electron/electron/blob/v43.0.0/shell/common/api/shared_texture/README.md)
- [myde-display API](../../README.md)