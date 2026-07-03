export type PixelFormat = 'bgra' | 'rgba' | 'rgbaf16' | 'nv12' | 'nv16' | 'p010le';

export interface SharedTexturePlane {
  stride: number;
  offset: number;
  size: number;
  fd: number;
}

export interface NativePixmap {
  planes: SharedTexturePlane[];
  modifier: string;
  supportsZeroCopyWebGpuImport: boolean;
}

export interface SharedTextureHandle {
  nativePixmap?: NativePixmap;
}

export interface SharedTextureImportTextureInfo {
  pixelFormat: PixelFormat;
  codedSize: { width: number; height: number };
  visibleRect?: { x: number; y: number; width: number; height: number };
  timestamp?: number;
  handle: SharedTextureHandle;
}

export interface NativeModule {
  openDrmDevice(devicePath?: string): DrmDeviceHandle;
  closeDrmDevice(handle: DrmDeviceHandle): void;
  getScreenInfo(handle: DrmDeviceHandle): ScreenInfo[];
  renderToScreen(handle: DrmDeviceHandle, textureInfo: SharedTextureImportTextureInfo, transform?: Transform): void;
  renderBufferToScreen(handle: DrmDeviceHandle, buffer: Buffer, width: number, height: number, pixelFormat: PixelFormat, transform?: Transform): void;
  createTransform(options: TransformOptions): Transform;
  applyTransform(transform: Transform, point: Point): Point;
  composeTransforms(...transforms: Transform[]): Transform;
}

export interface DrmDeviceHandle {
  id: string;
  devicePath: string;
}

export interface ScreenInfo {
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

export interface DisplayMode {
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

export interface TransformOptions {
  rotation?: number;
  scaleX?: number;
  scaleY?: number;
  translateX?: number;
  translateY?: number;
  originX?: number;
  originY?: number;
}

export interface Transform {
  matrix: number[];
}

export interface Point {
  x: number;
  y: number;
}

export interface RenderOptions {
  x?: number;
  y?: number;
  width?: number;
  height?: number;
  opacity?: number;
  blendMode?: BlendMode;
}

export enum BlendMode {
  None = 'none',
  Alpha = 'alpha',
  PremultipliedAlpha = 'premultiplied-alpha',
  Additive = 'additive',
}