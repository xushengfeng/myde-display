/**
 * Basic example demonstrating myde-display SharedTexture API
 * 
 * This example shows how to:
 * 1. Get screen information
 * 2. Create a DRM device
 * 3. Create SharedTexture with buffer
 * 4. Apply geometric transformations
 * 5. Render to screen
 */

import { 
  DrmDevice, 
  SharedTexture,
  TransformUtil, 
  getScreenInfo, 
  listDrmDevices,
  PixelFormat,
} from '../src/index';

async function main() {
  try {
    console.log('=== myde-display SharedTexture Example ===\n');

    // List available DRM devices
    console.log('1. Listing DRM devices...');
    const devices = listDrmDevices();
    console.log('Available devices:', devices);
    console.log('');

    if (devices.length === 0) {
      console.error('No DRM devices found. Make sure you have DRM support enabled.');
      process.exit(1);
    }

    // Get screen information
    console.log('2. Getting screen information...');
    const screens = getScreenInfo(devices[0]);
    console.log('Screens found:', screens.length);
    
    for (const screen of screens) {
      console.log(`\nScreen ${screen.connectorId}:`);
      console.log(`  Resolution: ${screen.width}x${screen.height}`);
      console.log(`  Refresh Rate: ${screen.refreshRate}Hz`);
      console.log(`  Connected: ${screen.isConnected}`);
      console.log(`  Physical Size: ${screen.physicalWidth}x${screen.physicalHeight}mm`);
      console.log(`  Subpixel: ${screen.subpixel}`);
      console.log(`  Connection: ${screen.connection}`);
      console.log(`  Available Modes: ${screen.modes.length}`);
      
      if (screen.modes.length > 0) {
        console.log('  First mode:', screen.modes[0]);
      }
    }

    // Create DRM device
    console.log('\n3. Creating DRM device...');
    const device = new DrmDevice(devices[0]);
    console.log('DRM device created successfully');

    // Create a simple texture (red square)
    console.log('\n4. Creating SharedTexture from buffer...');
    const textureWidth = 256;
    const textureHeight = 256;
    const textureData = Buffer.alloc(textureWidth * textureHeight * 4);
    
    // Fill with red color (RGBA)
    for (let i = 0; i < textureData.length; i += 4) {
      textureData[i] = 255;     // R
      textureData[i + 1] = 0;   // G
      textureData[i + 2] = 0;   // B
      textureData[i + 3] = 255; // A
    }
    
    // Create SharedTexture using fromBuffer helper
    const texture = SharedTexture.fromBuffer(
      textureData,
      textureWidth,
      textureHeight,
      'rgba' as PixelFormat
    );
    console.log(`SharedTexture created: ${texture.getSize().width}x${texture.getSize().height}`);
    console.log(`Pixel format: ${texture.getPixelFormat()}`);

    // Apply transformations
    console.log('\n5. Applying transformations...');
    
    // Create a 45-degree rotation around the center
    const rotation = TransformUtil.rotation(
      Math.PI / 4,  // 45 degrees
      textureWidth / 2,  // origin X
      textureHeight / 2  // origin Y
    );
    
    // Scale to 1.5x
    const scale = TransformUtil.scale(1.5, 1.5);
    
    // Translate to center of screen
    const screenInfo = screens[0];
    const translation = TransformUtil.translation(
      (screenInfo.width - textureWidth * 1.5) / 2,
      (screenInfo.height - textureHeight * 1.5) / 2
    );
    
    // Compose transformations (applied right-to-left)
    const transform = TransformUtil.compose(translation, scale, rotation);
    console.log('Transform matrix:', transform.matrix);

    // Apply transform to a test point
    const testPoint = { x: 0, y: 0 };
    const transformedPoint = TransformUtil.apply(transform, testPoint);
    console.log(`Point (0,0) transformed to: (${transformedPoint.x.toFixed(2)}, ${transformedPoint.y.toFixed(2)})`);

    // Render using renderBuffer (simplified API)
    console.log('\n6. Rendering buffer to screen...');
    device.renderBuffer(
      textureData,
      textureWidth,
      textureHeight,
      'rgba' as PixelFormat,
      transform
    );
    console.log('Buffer rendered successfully');

    // Example of using SharedTexture with nativePixmap (for DMA-BUF)
    console.log('\n7. Creating SharedTexture with nativePixmap...');
    
    // Example nativePixmap structure (would come from actual DMA-BUF allocation)
    const exampleNativePixmap = {
      planes: [
        {
          stride: textureWidth * 4,
          offset: 0,
          size: textureWidth * textureHeight * 4,
          fd: -1, // Would be actual file descriptor
        }
      ],
      modifier: '0', // DRM_FORMAT_MOD_LINEAR
      supportsZeroCopyWebGpuImport: false,
    };
    
    const sharedTexture = SharedTexture.fromNativePixmap(
      exampleNativePixmap,
      textureWidth,
      textureHeight,
      'bgra' as PixelFormat
    );
    console.log('SharedTexture with nativePixmap created');
    console.log('Texture info:', sharedTexture.getTextureInfo());

    // Wait a bit before cleanup
    console.log('\n8. Waiting 2 seconds before cleanup...');
    await new Promise(resolve => setTimeout(resolve, 2000));

    // Cleanup
    console.log('\n9. Cleaning up...');
    device.close();
    console.log('Cleanup complete');

    console.log('\n=== Example completed successfully ===');

  } catch (error) {
    console.error('Error:', error);
    process.exit(1);
  }
}

// Run the example
main();