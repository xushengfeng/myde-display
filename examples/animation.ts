/**
 * Animation example demonstrating SharedTexture updates and real-time rendering
 * 
 * This example shows how to:
 * 1. Create a SharedTexture
 * 2. Update texture data in real-time
 * 3. Apply dynamic transformations
 * 4. Create smooth animations
 */

import { 
  DrmDevice, 
  SharedTexture,
  TransformUtil, 
  getScreenInfo, 
  listDrmDevices,
  PixelFormat,
} from '../src/index';

class AnimationDemo {
  private device: DrmDevice;
  private textureData: Buffer;
  private width: number;
  private height: number;
  private frameCount: number = 0;
  private startTime: number = Date.now();

  constructor(devicePath: string) {
    console.log('Initializing animation demo...');
    
    // Create device
    this.device = new DrmDevice(devicePath);
    
    // Get screen info for positioning
    const screens = this.device.getScreenInfo();
    if (screens.length === 0) {
      throw new Error('No screens found');
    }
    
    const screen = screens[0];
    console.log(`Screen: ${screen.width}x${screen.height} @ ${screen.refreshRate}Hz`);
    
    // Create texture buffer (256x256)
    this.width = 256;
    this.height = 256;
    this.textureData = Buffer.alloc(this.width * this.height * 4);
    
    console.log('Animation demo initialized');
  }

  // Generate a colorful pattern
  private generatePattern(time: number): Buffer {
    const data = Buffer.alloc(this.width * this.height * 4);
    
    for (let y = 0; y < this.height; y++) {
      for (let x = 0; x < this.width; x++) {
        const offset = (y * this.width + x) * 4;
        
        // Create a moving gradient pattern
        const hue = ((x + y + time * 50) % 360) / 360;
        const saturation = 0.8;
        const value = 0.9;
        
        // Convert HSV to RGB
        const c = value * saturation;
        const x2 = c * (1 - Math.abs(((hue * 6) % 2) - 1));
        const m = value - c;
        
        let r, g, b;
        if (hue < 1/6) {
          r = c; g = x2; b = 0;
        } else if (hue < 2/6) {
          r = x2; g = c; b = 0;
        } else if (hue < 3/6) {
          r = 0; g = c; b = x2;
        } else if (hue < 4/6) {
          r = 0; g = x2; b = c;
        } else if (hue < 5/6) {
          r = x2; g = 0; b = c;
        } else {
          r = c; g = 0; b = x2;
        }
        
        data[offset] = Math.floor((r + m) * 255);     // R
        data[offset + 1] = Math.floor((g + m) * 255); // G
        data[offset + 2] = Math.floor((b + m) * 255); // B
        data[offset + 3] = 255;                        // A
      }
    }
    
    return data;
  }

  // Calculate dynamic transformation
  private calculateTransform(time: number): Transform {
    const screens = this.device.getScreenInfo();
    const screen = screens[0];
    
    // Rotation: continuous rotation
    const rotation = time * 0.5; // radians per second
    
    // Scale: pulsating scale
    const scale = 1.0 + 0.3 * Math.sin(time * 2);
    
    // Position: circular motion
    const radius = 100;
    const centerX = screen.width / 2;
    const centerY = screen.height / 2;
    const x = centerX + radius * Math.cos(time) - (this.width * scale) / 2;
    const y = centerY + radius * Math.sin(time) - (this.height * scale) / 2;
    
    // Create transforms
    const rotationTransform = TransformUtil.rotation(
      rotation,
      this.width / 2,
      this.height / 2
    );
    
    const scaleTransform = TransformUtil.scale(scale, scale);
    
    const translationTransform = TransformUtil.translation(x, y);
    
    // Compose: translate * scale * rotate
    return TransformUtil.compose(
      translationTransform,
      scaleTransform,
      rotationTransform
    );
  }

  // Run animation loop
  async run(durationSeconds: number = 10): Promise<void> {
    console.log(`Starting animation for ${durationSeconds} seconds...`);
    
    const startTime = Date.now();
    const endTime = startTime + durationSeconds * 1000;
    
    // Target 60 FPS
    const frameInterval = 1000 / 60;
    
    while (Date.now() < endTime) {
      const frameStart = Date.now();
      
      // Calculate current time
      const time = (Date.now() - startTime) / 1000;
      
      // Generate new pattern
      this.textureData = this.generatePattern(time);
      
      // Calculate transform
      const transform = this.calculateTransform(time);
      
      // Create SharedTexture and render
      const texture = SharedTexture.fromBuffer(
        this.textureData,
        this.width,
        this.height,
        'rgba' as PixelFormat
      );
      
      // Render to screen
      this.device.renderBuffer(
        this.textureData,
        this.width,
        this.height,
        'rgba' as PixelFormat,
        transform
      );
      
      // Update frame counter
      this.frameCount++;
      
      // Calculate FPS every second
      if (this.frameCount % 60 === 0) {
        const elapsed = (Date.now() - startTime) / 1000;
        const fps = this.frameCount / elapsed;
        console.log(`Frame ${this.frameCount}, FPS: ${fps.toFixed(1)}`);
      }
      
      // Wait for next frame
      const frameTime = Date.now() - frameStart;
      const sleepTime = Math.max(0, frameInterval - frameTime);
      
      if (sleepTime > 0) {
        await new Promise(resolve => setTimeout(resolve, sleepTime));
      }
    }
    
    console.log(`Animation complete. Total frames: ${this.frameCount}`);
  }

  // Cleanup
  destroy(): void {
    console.log('Cleaning up...');
    this.device.close();
    console.log('Cleanup complete');
  }
}

async function main() {
  try {
    console.log('=== Animation Demo ===\n');
    
    // List devices
    const devices = listDrmDevices();
    if (devices.length === 0) {
      console.error('No DRM devices found');
      process.exit(1);
    }
    
    console.log('Using device:', devices[0]);
    
    // Create and run demo
    const demo = new AnimationDemo(devices[0]);
    
    // Run for 10 seconds
    await demo.run(10);
    
    // Cleanup
    demo.destroy();
    
    console.log('\n=== Demo completed ===');
    
  } catch (error) {
    console.error('Error:', error);
    process.exit(1);
  }
}

// Run the demo
main();