# Electron Shared Texture Test - Multi-Screen

This test demonstrates how to use Electron's offscreen rendering with `SharedTexture` API and render different regions to multiple screens using `myde-display` DRM rendering.

## Overview

The test creates:
1. An **offscreen BrowserWindow** that renders a webpage using GPU-accelerated shared texture
2. **Multiple display windows** (one per physical display)
3. Integration with **myde-display** to split and render texture regions to different screens

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                          Main Process                               │
│                                                                     │
│  ┌─────────────────┐                                                │
│  │  OSR Window     │                                                │
│  │  (offscreen)    │──── paint event with SharedTexture ────┐       │
│  │  1920x1080      │                                        │       │
│  └─────────────────┘                                        │       │
│                                                             ▼       │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                    RegionMapper                                │  │
│  │                                                               │  │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐            │  │
│  │  │Region 0 │ │Region 1 │ │Region 2 │ │Region 3 │            │  │
│  │  │ (0,0)   │ │(960,0)  │ │(0,540)  │ │(960,540)│            │  │
│  │  │ 960x540 │ │ 960x540 │ │ 960x540 │ │ 960x540 │            │  │
│  │  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘            │  │
│  └───────┼───────────┼───────────┼───────────┼───────────────────┘  │
│          │           │           │           │                       │
│          ▼           ▼           ▼           ▼                       │
│  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐           │
│  │ Display 0 │ │ Display 1 │ │ Display 2 │ │ Display 3 │           │
│  │ Screen 0  │ │ Screen 1  │ │ Screen 2  │ │ Screen 3  │           │
│  └───────────┘ └───────────┘ └───────────┘ └───────────┘           │
└─────────────────────────────────────────────────────────────────────┘
```

## Prerequisites

- Linux with DRM support
- Electron 43.0.0
- Node.js 18+
- Access to `/dev/dri/card*` (video group)
- Multiple monitors (optional, works with single monitor too)

## Installation

```bash
cd test/electron-shared-texture
npm install
```

## Running

```bash
npm start
```

## Mapping Modes

### 1. Single Screen
Renders the entire texture to the first screen.

```javascript
mapper.addCustomRegion(0, 0, width, height, 0);
```

### 2. Horizontal Split
Splits the texture horizontally across all screens.

```javascript
mapper.addHorizontalSplit(width, height, [
  { screenId: 0 },
  { screenId: 1 },
  // ...
]);
```

### 3. Vertical Split
Splits the texture vertically across all screens.

```javascript
mapper.addVerticalSplit(width, height, [
  { screenId: 0 },
  { screenId: 1 },
  // ...
]);
```

### 4. Grid (2x2)
Splits the texture into a 2x2 grid.

```javascript
mapper.addGridMapping(width, height, 2, 2, [
  { screenId: 0 },
  { screenId: 1 },
  { screenId: 2 },
  { screenId: 3 },
]);
```

### 5. Custom Regions
Define custom source regions with individual transforms.

```javascript
mapper.addCustomRegion(
  0, 0,                    // Source position
  width / 2, height / 2,  // Source size
  0,                       // Target screen
  TransformUtil.rotation(Math.PI / 4)  // Transform
);
```

## API Reference

### RegionMapper

#### Methods

- `addMapping(sourceRect, target, transform)` - Add a region mapping
- `addGridMapping(sourceWidth, sourceHeight, columns, rows, screens, transforms)` - Add grid mapping
- `addHorizontalSplit(sourceWidth, sourceHeight, screens, transforms)` - Add horizontal split
- `addVerticalSplit(sourceWidth, sourceHeight, screens, transforms)` - Add vertical split
- `addCustomRegion(x, y, width, height, screenId, transform)` - Add custom region
- `getMappings()` - Get all mappings
- `clear()` - Clear all mappings

### RegionMapping

```typescript
interface RegionMapping {
  sourceRect: Rectangle;      // Source region in texture
  target: ScreenTarget;       // Target screen
  transform?: Transform;      // Optional transform
}

interface Rectangle {
  x: number;
  y: number;
  width: number;
  height: number;
}

interface ScreenTarget {
  screenId: number;
  connectorId?: number;
  destX?: number;
  destY?: number;
  destWidth?: number;
  destHeight?: number;
}
```

## Controls

The display window provides buttons to:
- **Single**: Render full texture to single screen
- **Horizontal Split**: Split texture across screens horizontally
- **Vertical Split**: Split texture across screens vertically
- **Grid (2x2)**: Split texture into 2x2 grid
- **Custom Regions**: Apply custom regions with transforms

## Transformations

Each region can have its own transform:

```typescript
// Rotate region 45 degrees
const rotation = TransformUtil.rotation(Math.PI / 4, originX, originY);

// Scale region
const scale = TransformUtil.scale(1.5, 1.5);

// Translate region
const translation = TransformUtil.translation(x, y);

// Flip region
const flip = TransformUtil.flip(true, false); // horizontal flip

// Compose multiple transforms
const combined = TransformUtil.compose(translation, scale, rotation);
```

## Troubleshooting

### Only One Screen Detected

If only one screen is detected:
1. Connect additional monitors
2. Configure displays in system settings
3. The example will still work with single screen mode

### Regions Not Rendering

If regions are not rendering correctly:
1. Check that source regions are within texture bounds
2. Verify screen IDs are valid
3. Check console for error messages

### Performance Issues

- Reduce frame rate: `osrWindow.webContents.setFrameRate(30)`
- Use simpler webpages for testing
- Reduce texture resolution

## Notes

- Source regions must be within texture bounds
- Each region can target any screen
- Transforms are applied per-region
- The texture is released after each frame
- Multiple regions can target the same screen

## References

- [Electron Offscreen Rendering](https://www.electronjs.org/docs/latest/tutorial/offscreen-rendering)
- [Electron SharedTexture API](https://github.com/electron/electron/blob/v43.0.0/shell/common/api/shared_texture/README.md)
- [myde-display API](../../README.md)