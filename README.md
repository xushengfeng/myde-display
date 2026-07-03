# myde-display

A Node.js native module for DRM rendering and screen management, built with Neon (Rust bindings for Node.js).

## Features

- **DRM Rendering**: Render textures directly to the screen using Linux DRM (Direct Rendering Manager) without a desktop environment
- **SharedTexture API**: Compatible with Electron's SharedTexture handle structure for DMA-BUF support
- **Geometric Transformations**: Apply arbitrary rotations, scaling, and translations to textures
- **Screen Information**: Retrieve detailed information about connected displays
- **Multiple Pixel Formats**: Support for RGBA, BGRA, NV12, NV16, P010LE, and RGBAF16
- **High Performance**: Built with Rust for optimal performance and memory safety

## Installation

### Prerequisites

- Node.js (v16 or later)
- Rust (stable toolchain)
- Linux with DRM support
- Development libraries:
  - `libdrm-dev`
  - `libgbm-dev`
  - `libudev-dev`

### Install via wget

```bash
# Download the package
wget https://github.com/your-username/myde-display/releases/latest/download/myde-display-linux-x64.tar.gz

# Extract
tar -xzf myde-display-linux-x64.tar.gz

# Install
cd myde-display
npm install
```

### Install from source

```bash
# Clone the repository
git clone https://github.com/your-username/myde-display.git
cd myde-display

# Install dependencies
pnpm install

# Build
pnpm run build
```

## Usage

### Basic Example with SharedTexture

```typescript
import { 
  DrmDevice, 
  SharedTexture, 
  TransformUtil, 
  getScreenInfo,
  PixelFormat 
} from 'myde-display';

// Get screen information
const screens = getScreenInfo();
console.log('Available screens:', screens);

// Create a DRM device
const device = new DrmDevice('/dev/dri/card0');

// Create a texture (RGBA format)
const width = 1920;
const height = 1080;
const data = Buffer.alloc(width * height * 4); // RGBA

// Fill with a gradient
for (let y = 0; y < height; y++) {
  for (let x = 0; x < width; x++) {
    const offset = (y * width + x) * 4;
    data[offset] = Math.floor(x / width * 255);     // R
    data[offset + 1] = Math.floor(y / height * 255); // G
    data[offset + 2] = 128;                          // B
    data[offset + 3] = 255;                          // A
  }
}

// Apply transformations
const rotation = TransformUtil.rotation(Math.PI / 4); // 45 degrees
const scale = TransformUtil.scale(0.5, 0.5);
const translation = TransformUtil.translation(100, 100);

// Compose transformations
const transform = TransformUtil.compose(translation, scale, rotation);

// Render buffer to screen
device.renderBuffer(data, width, height, 'rgba' as PixelFormat, transform);

// Cleanup
device.close();
```

### Using SharedTexture with DMA-BUF (nativePixmap)

```typescript
import { 
  DrmDevice, 
  SharedTexture, 
  TransformUtil,
  PixelFormat 
} from 'myde-display';

// Create DRM device
const device = new DrmDevice('/dev/dri/card0');

// NativePixmap structure (from DMA-BUF allocation)
const nativePixmap = {
  planes: [
    {
      stride: 1920 * 4,  // Stride in bytes
      offset: 0,         // Offset in bytes
      size: 1920 * 1080 * 4,  // Plane size in bytes
      fd: 5,             // File descriptor for DMA-BUF
    }
  ],
  modifier: '0',  // DRM_FORMAT_MOD_LINEAR
  supportsZeroCopyWebGpuImport: false,
};

// Create SharedTexture from nativePixmap
const texture = SharedTexture.fromNativePixmap(
  nativePixmap,
  1920,
  1080,
  'bgra' as PixelFormat
);

// Get texture info
console.log('Texture info:', texture.getTextureInfo());
console.log('Pixel format:', texture.getPixelFormat());
console.log('Size:', texture.getSize());

// Render with transform
const transform = TransformUtil.rotation(Math.PI / 6);
device.renderSharedTexture(texture.getTextureInfo(), transform);

device.close();
```

### Screen Information

```typescript
import { getScreenInfo, listDrmDevices } from 'myde-display';

// List all DRM devices
const devices = listDrmDevices();
console.log('DRM devices:', devices);

// Get detailed screen information
const screens = getScreenInfo('/dev/dri/card0');
for (const screen of screens) {
  console.log(`Screen ${screen.connectorId}:`);
  console.log(`  Resolution: ${screen.width}x${screen.height}`);
  console.log(`  Refresh Rate: ${screen.refreshRate}Hz`);
  console.log(`  Connected: ${screen.isConnected}`);
  console.log(`  Physical Size: ${screen.physicalWidth}x${screen.physicalHeight}mm`);
  console.log(`  Modes: ${screen.modes.length}`);
}
```

### Geometric Transformations

```typescript
import { TransformUtil } from 'myde-display';

// Create individual transforms
const rotation = TransformUtil.rotation(Math.PI / 6); // 30 degrees
const scale = TransformUtil.scale(2.0, 2.0);
const translation = TransformUtil.translation(50, 100);

// Compose transforms (applied right-to-left)
const combined = TransformUtil.compose(translation, scale, rotation);

// Apply transform to a point
const originalPoint = { x: 100, y: 100 };
const transformedPoint = TransformUtil.apply(combined, originalPoint);
console.log('Transformed point:', transformedPoint);

// Create transform with custom origin
const rotationAroundCenter = TransformUtil.rotation(
  Math.PI / 4,
  960,  // origin X
  540   // origin Y
);
```

## API Reference

### Classes

#### `DrmDevice`

Represents a DRM device for rendering.

**Constructor:**
- `new DrmDevice(devicePath?: string)` - Opens a DRM device (default: `/dev/dri/card0`)

**Methods:**
- `getScreenInfo(): ScreenInfo[]` - Gets information about connected screens
- `renderSharedTexture(textureInfo: SharedTextureImportTextureInfo, transform?: Transform): void` - Renders a SharedTexture to the screen
- `renderBuffer(buffer: Buffer, width: number, height: number, pixelFormat?: PixelFormat, transform?: Transform): void` - Renders a buffer to the screen
- `close(): void` - Closes the device

#### `SharedTexture`

Represents a shared texture compatible with Electron's SharedTexture API.

**Static Methods:**
- `SharedTexture.fromBuffer(buffer: Buffer, width: number, height: number, pixelFormat?: PixelFormat): SharedTexture` - Creates from buffer
- `SharedTexture.fromNativePixmap(nativePixmap: NativePixmap, width: number, height: number, pixelFormat?: PixelFormat): SharedTexture` - Creates from DMA-BUF nativePixmap

**Methods:**
- `getTextureInfo(): SharedTextureImportTextureInfo` - Gets the texture info
- `getPixelFormat(): PixelFormat` - Gets the pixel format
- `getSize(): { width: number; height: number }` - Gets the texture size
- `getVisibleRect(): { x: number; y: number; width: number; height: number }` - Gets the visible rectangle

#### `TransformUtil`

Utility class for geometric transformations.

**Static Methods:**
- `TransformUtil.create(options: TransformOptions): Transform` - Creates a transform from options
- `TransformUtil.rotation(angle: number, originX?: number, originY?: number): Transform` - Creates rotation transform
- `TransformUtil.scale(scaleX: number, scaleY?: number, originX?: number, originY?: number): Transform` - Creates scale transform
- `TransformUtil.translation(x: number, y: number): Transform` - Creates translation transform
- `TransformUtil.apply(transform: Transform, point: Point): Point` - Applies transform to a point
- `TransformUtil.compose(...transforms: Transform[]): Transform` - Composes multiple transforms

### Functions

- `getScreenInfo(devicePath?: string): ScreenInfo[]` - Gets screen information
- `listDrmDevices(): string[]` - Lists available DRM devices

### Types

#### `SharedTextureImportTextureInfo`

Compatible with Electron's SharedTextureImportTextureInfo structure.

```typescript
interface SharedTextureImportTextureInfo {
  pixelFormat: PixelFormat;
  codedSize: { width: number; height: number };
  visibleRect?: { x: number; y: number; width: number; height: number };
  timestamp?: number;
  handle: SharedTextureHandle;
}
```

#### `SharedTextureHandle`

Compatible with Electron's SharedTextureHandle structure (Linux).

```typescript
interface SharedTextureHandle {
  nativePixmap?: NativePixmap;
}
```

#### `NativePixmap`

```typescript
interface NativePixmap {
  planes: SharedTexturePlane[];
  modifier: string;
  supportsZeroCopyWebGpuImport: boolean;
}
```

#### `SharedTexturePlane`

```typescript
interface SharedTexturePlane {
  stride: number;
  offset: number;
  size: number;
  fd: number;
}
```

#### `PixelFormat`

```typescript
type PixelFormat = 'bgra' | 'rgba' | 'rgbaf16' | 'nv12' | 'nv16' | 'p010le';
```

#### `ScreenInfo`

```typescript
interface ScreenInfo {
  connectorId: number;
  encoderId: number;
  crtcId: number;
  width: number;
  height: number;
  refreshRate: number;
  isConnected: boolean;
  modes: DisplayMode[];
  physicalWidth: number;
  physicalHeight: number;
  subpixel: string;
  connection: string;
}
```

#### `DisplayMode`

```typescript
interface DisplayMode {
  clock: number;
  hdisplay: number;
  hsyncStart: number;
  hsyncEnd: number;
  htotal: number;
  hskew: number;
  vdisplay: number;
  vsyncStart: number;
  vsyncEnd: number;
  vtotal: number;
  vscan: number;
  vrefresh: number;
  flags: number;
  type: number;
  name: string;
}
```

#### `TransformOptions`

```typescript
interface TransformOptions {
  rotation?: number;      // Rotation angle in radians
  scaleX?: number;        // X-axis scale factor
  scaleY?: number;        // Y-axis scale factor
  translateX?: number;    // X-axis translation
  translateY?: number;    // Y-axis translation
  originX?: number;       // Transform origin X
  originY?: number;       // Transform origin Y
}
```

#### `Transform`

```typescript
interface Transform {
  matrix: number[]; // 3x3 transformation matrix (row-major)
}
```

#### `Point`

```typescript
interface Point {
  x: number;
  y: number;
}
```

## Building

```bash
# Debug build
pnpm run build

# Release build
pnpm run build:rust:release

# Clean build artifacts
pnpm run clean
```

## Requirements

- Linux kernel with DRM support
- Access to `/dev/dri/card*` devices (typically requires `video` group membership)
- Rust toolchain (stable)
- Node.js v16+

## Troubleshooting

### Permission Denied

If you get permission errors when accessing DRM devices:

```bash
# Add your user to the video group
sudo usermod -a -G video $USER

# Log out and log back in for changes to take effect
```

### No Connected Display

Make sure:
1. A display is physically connected
2. The display is powered on
3. The correct DRM device is being used (`/dev/dri/card0`, `/dev/dri/card1`, etc.)

### Build Errors

Ensure all development dependencies are installed:

```bash
# Ubuntu/Debian
sudo apt-get install libdrm-dev libgbm-dev libudev-dev

# Fedora/RHEL
sudo dnf install libdrm-devel mesa-libgbm-devel systemd-devel

# Arch Linux
sudo pacman -S libdrm mesa libsystemd
```

## Testing

### Electron Shared Texture Test

A test project demonstrating integration with Electron's offscreen rendering using SharedTexture API.

```bash
cd test/electron-shared-texture
npm install
npm start
```

This test:
1. Creates an Electron window with offscreen rendering enabled
2. Captures the rendered frames as SharedTexture
3. Uses myde-display to render the texture to a DRM display with transformations

See [test/electron-shared-texture/README.md](test/electron-shared-texture/README.md) for details.

## License

Apache License 2.0

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgments

- [Neon](https://neon-bindings.com/) - Rust bindings for Node.js
- [drm-rs](https://github.com/Smithay/drm-rs) - Rust DRM library
- [gbm-rs](https://github.com/Smithay/gbm-rs) - Rust GBM library
- [nalgebra](https://nalgebra.org/) - Linear algebra library for Rust
- [Electron SharedTexture API](https://www.electronjs.org/docs/latest/api/structures/shared-texture-handle) - API design reference