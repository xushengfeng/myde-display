import native from '../index.node';
import { BlendMode } from './types/native';
import type {
  NativeModule,
  DrmDeviceHandle,
  ScreenInfo,
  SharedTextureImportTextureInfo,
  SharedTextureHandle,
  NativePixmap,
  SharedTexturePlane,
  PixelFormat,
  Rectangle,
  ScreenTarget,
  RegionMapping,
  TransformOptions,
  Transform,
  Point,
  RenderOptions,
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

  renderRegion(
    textureInfo: SharedTextureImportTextureInfo,
    sourceRect: Rectangle,
    destRect: Rectangle,
    transform?: Transform
  ): void {
    addon.renderRegionToScreen(this.handle, textureInfo, sourceRect, destRect, transform);
  }

  getHandle(): DrmDeviceHandle {
    return this.handle;
  }
}

export class MultiScreenRenderer {
  private devices: Map<string, DrmDevice> = new Map();

  addDevice(devicePath: string): DrmDevice {
    if (!this.devices.has(devicePath)) {
      const device = new DrmDevice(devicePath);
      this.devices.set(devicePath, device);
    }
    return this.devices.get(devicePath)!;
  }

  removeDevice(devicePath: string): void {
    const device = this.devices.get(devicePath);
    if (device) {
      device.close();
      this.devices.delete(devicePath);
    }
  }

  getDevice(devicePath: string): DrmDevice | undefined {
    return this.devices.get(devicePath);
  }

  getAllDevices(): DrmDevice[] {
    return Array.from(this.devices.values());
  }

  renderToMultipleScreens(
    textureInfo: SharedTextureImportTextureInfo,
    mappings: RegionMapping[]
  ): void {
    const handles = Array.from(this.devices.values()).map(d => d.getHandle());
    addon.renderMultiScreen(handles, textureInfo, mappings);
  }

  closeAll(): void {
    for (const device of this.devices.values()) {
      device.close();
    }
    this.devices.clear();
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

export class RegionMapper {
  private mappings: RegionMapping[] = [];

  addMapping(
    sourceRect: Rectangle,
    target: ScreenTarget,
    transform?: Transform
  ): this {
    this.mappings.push({
      sourceRect,
      target,
      transform,
    });
    return this;
  }

  addGridMapping(
    sourceWidth: number,
    sourceHeight: number,
    columns: number,
    rows: number,
    screens: ScreenTarget[],
    transforms?: Transform[]
  ): this {
    const cellWidth = sourceWidth / columns;
    const cellHeight = sourceHeight / rows;

    for (let row = 0; row < rows; row++) {
      for (let col = 0; col < columns; col++) {
        const index = row * columns + col;
        if (index >= screens.length) break;

        const sourceRect: Rectangle = {
          x: col * cellWidth,
          y: row * cellHeight,
          width: cellWidth,
          height: cellHeight,
        };

        this.mappings.push({
          sourceRect,
          target: screens[index],
          transform: transforms?.[index],
        });
      }
    }

    return this;
  }

  addHorizontalSplit(
    sourceWidth: number,
    sourceHeight: number,
    screens: ScreenTarget[],
    transforms?: Transform[]
  ): this {
    const segmentWidth = sourceWidth / screens.length;

    for (let i = 0; i < screens.length; i++) {
      const sourceRect: Rectangle = {
        x: i * segmentWidth,
        y: 0,
        width: segmentWidth,
        height: sourceHeight,
      };

      this.mappings.push({
        sourceRect,
        target: screens[i],
        transform: transforms?.[i],
      });
    }

    return this;
  }

  addVerticalSplit(
    sourceWidth: number,
    sourceHeight: number,
    screens: ScreenTarget[],
    transforms?: Transform[]
  ): this {
    const segmentHeight = sourceHeight / screens.length;

    for (let i = 0; i < screens.length; i++) {
      const sourceRect: Rectangle = {
        x: 0,
        y: i * segmentHeight,
        width: sourceWidth,
        height: segmentHeight,
      };

      this.mappings.push({
        sourceRect,
        target: screens[i],
        transform: transforms?.[i],
      });
    }

    return this;
  }

  addCustomRegion(
    x: number,
    y: number,
    width: number,
    height: number,
    screenId: number,
    transform?: Transform
  ): this {
    this.mappings.push({
      sourceRect: { x, y, width, height },
      target: { screenId },
      transform,
    });
    return this;
  }

  getMappings(): RegionMapping[] {
    return [...this.mappings];
  }

  clear(): void {
    this.mappings = [];
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

  static flip(horizontal: boolean = false, vertical: boolean = false): Transform {
    return addon.createTransform({
      scaleX: horizontal ? -1 : 1,
      scaleY: vertical ? -1 : 1,
    });
  }

  static crop(x: number, y: number, width: number, height: number): Transform {
    return addon.createTransform({
      translateX: -x,
      translateY: -y,
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

export function getAllScreens(): Map<string, ScreenInfo[]> {
  const screensMap = new Map<string, ScreenInfo[]>();
  const devices = listDrmDevices();
  
  for (const devicePath of devices) {
    try {
      const screens = getScreenInfo(devicePath);
      screensMap.set(devicePath, screens);
    } catch (error) {
      // Skip devices that can't be opened
    }
  }
  
  return screensMap;
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

export function createRegionMapping(
  sourceX: number,
  sourceY: number,
  sourceWidth: number,
  sourceHeight: number,
  screenId: number,
  transform?: Transform
): RegionMapping {
  return {
    sourceRect: {
      x: sourceX,
      y: sourceY,
      width: sourceWidth,
      height: sourceHeight,
    },
    target: {
      screenId,
    },
    transform,
  };
}

export type {
  DrmDeviceHandle,
  ScreenInfo,
  SharedTextureImportTextureInfo,
  SharedTextureHandle,
  NativePixmap,
  SharedTexturePlane,
  PixelFormat,
  Rectangle,
  ScreenTarget,
  RegionMapping,
  TransformOptions,
  Transform,
  Point,
  RenderOptions,
};

export { BlendMode };

export default {
  DrmDevice,
  MultiScreenRenderer,
  SharedTexture,
  RegionMapper,
  TransformUtil,
  getScreenInfo,
  getAllScreens,
  listDrmDevices,
  createRegionMapping,
  BlendMode,
};