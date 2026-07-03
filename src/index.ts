import native from '../index.node';
import type {
  NativeModule,
  DrmDeviceHandle,
  ScreenInfo,
  SharedTextureImportTextureInfo,
  SharedTextureHandle,
  NativePixmap,
  SharedTexturePlane,
  PixelFormat,
  TransformOptions,
  Transform,
  Point,
  RenderOptions,
  BlendMode,
} from './types/native';

const addon: NativeModule = native;

export class DrmDevice {
  private handle: DrmDeviceHandle;

  constructor(devicePath?: string) {
    this.handle = addon.openDrmDevice(devicePath);
  }

  close(): void {
    addon.closeDrmDevice(this.handle);
  }

  getScreenInfo(): ScreenInfo[] {
    return addon.getScreenInfo(this.handle);
  }

  renderSharedTexture(textureInfo: SharedTextureImportTextureInfo, transform?: Transform): void {
    addon.renderToScreen(this.handle, textureInfo, transform);
  }

  renderBuffer(
    buffer: Buffer,
    width: number,
    height: number,
    pixelFormat: PixelFormat = 'rgba',
    transform?: Transform
  ): void {
    addon.renderBufferToScreen(this.handle, buffer, width, height, pixelFormat, transform);
  }

  getHandle(): DrmDeviceHandle {
    return this.handle;
  }
}

export class SharedTexture {
  private textureInfo: SharedTextureImportTextureInfo;

  constructor(textureInfo: SharedTextureImportTextureInfo) {
    this.textureInfo = textureInfo;
  }

  static fromNativePixmap(
    nativePixmap: NativePixmap,
    width: number,
    height: number,
    pixelFormat: PixelFormat = 'bgra',
    options?: {
      visibleRect?: { x: number; y: number; width: number; height: number };
      timestamp?: number;
    }
  ): SharedTexture {
    const handle: SharedTextureHandle = { nativePixmap };
    const textureInfo: SharedTextureImportTextureInfo = {
      pixelFormat,
      codedSize: { width, height },
      handle,
      ...options,
    };
    return new SharedTexture(textureInfo);
  }

  static fromBuffer(
    buffer: Buffer,
    width: number,
    height: number,
    pixelFormat: PixelFormat = 'rgba'
  ): SharedTexture {
    const textureInfo: SharedTextureImportTextureInfo = {
      pixelFormat,
      codedSize: { width, height },
      handle: {},
    };
    return new SharedTexture(textureInfo);
  }

  getTextureInfo(): SharedTextureImportTextureInfo {
    return this.textureInfo;
  }

  getPixelFormat(): PixelFormat {
    return this.textureInfo.pixelFormat;
  }

  getSize(): { width: number; height: number } {
    return this.textureInfo.codedSize;
  }

  getVisibleRect(): { x: number; y: number; width: number; height: number } {
    return this.textureInfo.visibleRect || {
      x: 0,
      y: 0,
      width: this.textureInfo.codedSize.width,
      height: this.textureInfo.codedSize.height,
    };
  }
}

export class TransformUtil {
  static create(options: TransformOptions): Transform {
    return addon.createTransform(options);
  }

  static apply(transform: Transform, point: Point): Point {
    return addon.applyTransform(transform, point);
  }

  static compose(...transforms: Transform[]): Transform {
    return addon.composeTransforms(...transforms);
  }

  static rotation(angle: number, originX?: number, originY?: number): Transform {
    return addon.createTransform({
      rotation: angle,
      originX: originX || 0,
      originY: originY || 0,
    });
  }

  static scale(scaleX: number, scaleY?: number, originX?: number, originY?: number): Transform {
    return addon.createTransform({
      scaleX,
      scaleY: scaleY || scaleX,
      originX: originX || 0,
      originY: originY || 0,
    });
  }

  static translation(x: number, y: number): Transform {
    return addon.createTransform({
      translateX: x,
      translateY: y,
    });
  }
}

export function getScreenInfo(devicePath?: string): ScreenInfo[] {
  const device = new DrmDevice(devicePath);
  try {
    return device.getScreenInfo();
  } finally {
    device.close();
  }
}

export function listDrmDevices(): string[] {
  const devices: string[] = [];
  const fs = require('fs');
  const path = require('path');
  
  try {
    const driPath = '/dev/dri';
    const entries = fs.readdirSync(driPath);
    for (const entry of entries) {
      if (entry.startsWith('card')) {
        devices.push(path.join(driPath, entry));
      }
    }
  } catch (error) {
    // 忽略错误
  }
  
  return devices;
}

export type {
  DrmDeviceHandle,
  ScreenInfo,
  SharedTextureImportTextureInfo,
  SharedTextureHandle,
  NativePixmap,
  SharedTexturePlane,
  PixelFormat,
  TransformOptions,
  Transform,
  Point,
  RenderOptions,
};

export { BlendMode };

export default {
  DrmDevice,
  SharedTexture,
  TransformUtil,
  getScreenInfo,
  listDrmDevices,
  BlendMode,
};