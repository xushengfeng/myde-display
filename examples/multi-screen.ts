/**
 * Multi-screen example demonstrating region-based rendering
 * 
 * This example shows how to:
 * 1. Split a single texture into multiple regions
 * 2. Apply different transforms to each region
 * 3. Render different regions to different screens
 */

import { 
  DrmDevice,
  MultiScreenRenderer,
  SharedTexture,
  RegionMapper,
  TransformUtil, 
  getScreenInfo, 
  listDrmDevices,
  getAllScreens,
  PixelFormat,
  RegionMapping,
} from '../src/index';

async function main() {
  try {
    console.log('=== Multi-Screen Rendering Example ===\n');

    // List all available screens
    console.log('1. Discovering screens...');
    const allScreens = getAllScreens();
    
    let totalScreens = 0;
    for (const [devicePath, screens] of allScreens) {
      console.log(`\nDevice: ${devicePath}`);
      for (const screen of screens) {
        if (screen.isConnected) {
          console.log(`  Screen ${screen.connectorId}: ${screen.width}x${screen.height} @ ${screen.refreshRate}Hz`);
          totalScreens++;
        }
      }
    }
    
    console.log(`\nTotal connected screens: ${totalScreens}`);
    
    if (totalScreens < 2) {
      console.log('This example requires at least 2 screens. Using single screen demo mode.');
    }

    // Create a large texture (e.g., 3840x2160 - 4K)
    console.log('\n2. Creating source texture...');
    const textureWidth = 3840;
    const textureHeight = 2160;
    const textureData = Buffer.alloc(textureWidth * textureHeight * 4);
    
    // Fill with a colorful gradient pattern
    for (let y = 0; y < textureHeight; y++) {
      for (let x = 0; x < textureWidth; x++) {
        const offset = (y * textureWidth + x) * 4;
        
        // Create quadrants with different colors
        const isLeft = x < textureWidth / 2;
        const isTop = y < textureHeight / 2;
        
        if (isTop && isLeft) {
          // Top-left: Red
          textureData[offset] = 255;
          textureData[offset + 1] = 0;
          textureData[offset + 2] = 0;
        } else if (isTop && !isLeft) {
          // Top-right: Green
          textureData[offset] = 0;
          textureData[offset + 1] = 255;
          textureData[offset + 2] = 0;
        } else if (!isTop && isLeft) {
          // Bottom-left: Blue
          textureData[offset] = 0;
          textureData[offset + 1] = 0;
          textureData[offset + 2] = 255;
        } else {
          // Bottom-right: Yellow
          textureData[offset] = 255;
          textureData[offset + 1] = 255;
          textureData[offset + 2] = 0;
        }
        
        textureData[offset + 3] = 255; // Alpha
        
        // Add grid lines
        if (x % 100 === 0 || y % 100 === 0) {
          textureData[offset] = 128;
          textureData[offset + 1] = 128;
          textureData[offset + 2] = 128;
        }
      }
    }
    
    console.log(`Texture created: ${textureWidth}x${textureHeight}`);

    // Example 1: Horizontal Split - Split texture across screens
    console.log('\n3. Example 1: Horizontal Split');
    await exampleHorizontalSplit(textureData, textureWidth, textureHeight);

    // Example 2: Grid Split - Split texture into 2x2 grid
    console.log('\n4. Example 2: Grid Split (2x2)');
    await exampleGridSplit(textureData, textureWidth, textureHeight);

    // Example 3: Custom Regions with transforms
    console.log('\n5. Example 3: Custom Regions with Transforms');
    await exampleCustomRegions(textureData, textureWidth, textureHeight);

    // Example 4: Picture-in-Picture style
    console.log('\n6. Example 4: Picture-in-Picture');
    await examplePictureInPicture(textureData, textureWidth, textureHeight);

    console.log('\n=== Example completed ===');

  } catch (error) {
    console.error('Error:', error);
    process.exit(1);
  }
}

async function exampleHorizontalSplit(
  textureData: Buffer,
  textureWidth: number,
  textureHeight: number
) {
  const devices = listDrmDevices();
  if (devices.length === 0) {
    console.log('  No DRM devices available');
    return;
  }
  
  const device = new DrmDevice(devices[0]);
  const screens = device.getScreenInfo();
  
  // Create texture
  const texture = SharedTexture.fromBuffer(
    textureData,
    textureWidth,
    textureHeight,
    'rgba' as PixelFormat
  );
  
  // Create region mapper for horizontal split
  const mapper = new RegionMapper();
  
  // If we have multiple screens, split horizontally
  // Otherwise, render different regions to same screen
  const screenTargets = screens
    .filter(s => s.isConnected)
    .map((s, i) => ({ screenId: i, connectorId: s.connectorId }));
  
  if (screenTargets.length >= 2) {
    // Split texture horizontally across screens
    mapper.addHorizontalSplit(
      textureWidth,
      textureHeight,
      screenTargets.slice(0, 2),
      [
        TransformUtil.translation(0, 0),
        TransformUtil.translation(0, 0),
      ]
    );
  } else {
    // Single screen: show left half
    mapper.addCustomRegion(
      0, 0,
      textureWidth / 2,
      textureHeight,
      0,
      TransformUtil.scale(0.5, 0.5)
    );
  }
  
  console.log('  Mappings:', mapper.getMappings().length);
  
  // Note: In real usage, you would call:
  // device.renderMultiScreen([device.getHandle()], texture.getTextureInfo(), mapper.getMappings());
  
  device.close();
}

async function exampleGridSplit(
  textureData: Buffer,
  textureWidth: number,
  textureHeight: number
) {
  const devices = listDrmDevices();
  if (devices.length === 0) {
    console.log('  No DRM devices available');
    return;
  }
  
  const device = new DrmDevice(devices[0]);
  const screens = device.getScreenInfo();
  
  const texture = SharedTexture.fromBuffer(
    textureData,
    textureWidth,
    textureHeight,
    'rgba' as PixelFormat
  );
  
  const mapper = new RegionMapper();
  
  // Create 2x2 grid mapping
  const gridScreens = [
    { screenId: 0 },
    { screenId: 0 },
    { screenId: 0 },
    { screenId: 0 },
  ];
  
  mapper.addGridMapping(
    textureWidth,
    textureHeight,
    2,  // columns
    2,  // rows
    gridScreens
  );
  
  console.log('  Grid mappings:', mapper.getMappings().length);
  console.log('  Each region: ' + (textureWidth / 2) + 'x' + (textureHeight / 2));
  
  device.close();
}

async function exampleCustomRegions(
  textureData: Buffer,
  textureWidth: number,
  textureHeight: number
) {
  const devices = listDrmDevices();
  if (devices.length === 0) {
    console.log('  No DRM devices available');
    return;
  }
  
  const device = new DrmDevice(devices[0]);
  
  const texture = SharedTexture.fromBuffer(
    textureData,
    textureWidth,
    textureHeight,
    'rgba' as PixelFormat
  );
  
  const mapper = new RegionMapper();
  
  // Region 1: Top-left corner, rotated 45 degrees
  mapper.addCustomRegion(
    0, 0,
    textureWidth / 2,
    textureHeight / 2,
    0,
    TransformUtil.compose(
      TransformUtil.translation(200, 100),
      TransformUtil.rotation(Math.PI / 4, textureWidth / 4, textureHeight / 4)
    )
  );
  
  // Region 2: Top-right corner, scaled
  mapper.addCustomRegion(
    textureWidth / 2, 0,
    textureWidth / 2,
    textureHeight / 2,
    0,
    TransformUtil.compose(
      TransformUtil.translation(600, 100),
      TransformUtil.scale(0.75, 0.75)
    )
  );
  
  // Region 3: Bottom-left, flipped
  mapper.addCustomRegion(
    0, textureHeight / 2,
    textureWidth / 2,
    textureHeight / 2,
    0,
    TransformUtil.compose(
      TransformUtil.translation(200, 400),
      TransformUtil.flip(true, false)
    )
  );
  
  // Region 4: Bottom-right, no transform
  mapper.addCustomRegion(
    textureWidth / 2, textureHeight / 2,
    textureWidth / 2,
    textureHeight / 2,
    0
  );
  
  console.log('  Custom regions:', mapper.getMappings().length);
  console.log('  Each with different transform applied');
  
  device.close();
}

async function examplePictureInPicture(
  textureData: Buffer,
  textureWidth: number,
  textureHeight: number
) {
  const devices = listDrmDevices();
  if (devices.length === 0) {
    console.log('  No DRM devices available');
    return;
  }
  
  const device = new DrmDevice(devices[0]);
  
  const texture = SharedTexture.fromBuffer(
    textureData,
    textureWidth,
    textureHeight,
    'rgba' as PixelFormat
  );
  
  const mapper = new RegionMapper();
  
  // Main view: Full texture scaled down
  mapper.addCustomRegion(
    0, 0,
    textureWidth,
    textureHeight,
    0,
    TransformUtil.scale(0.5, 0.5)
  );
  
  // PiP view 1: Top-left quadrant, positioned in corner
  mapper.addCustomRegion(
    0, 0,
    textureWidth / 2,
    textureHeight / 2,
    0,
    TransformUtil.compose(
      TransformUtil.translation(600, 400),
      TransformUtil.scale(0.3, 0.3)
    )
  );
  
  // PiP view 2: Bottom-right quadrant, positioned in other corner
  mapper.addCustomRegion(
    textureWidth / 2, textureHeight / 2,
    textureWidth / 2,
    textureHeight / 2,
    0,
    TransformUtil.compose(
      TransformUtil.translation(50, 400),
      TransformUtil.scale(0.3, 0.3)
    )
  );
  
  console.log('  PiP regions:', mapper.getMappings().length);
  
  device.close();
}

// Run the example
main();