const { app, BrowserWindow, ipcMain, screen } = require('electron');
const path = require('path');

// Import myde-display native module
// In real usage, this would be: const { DrmDevice, SharedTexture, TransformUtil, RegionMapper, MultiScreenRenderer } = require('myde-display');
// For testing, we'll simulate the API
const mydeDisplay = {
  DrmDevice: class {
    constructor(devicePath) {
      console.log(`[myde-display] Opening DRM device: ${devicePath || '/dev/dri/card0'}`);
      this.devicePath = devicePath || '/dev/dri/card0';
    }
    
    getScreenInfo() {
      console.log('[myde-display] Getting screen info');
      return [{
        connectorId: 1,
        width: 1920,
        height: 1080,
        refreshRate: 60,
        isConnected: true
      }];
    }
    
    renderSharedTexture(textureInfo, transform) {
      console.log('[myde-display] Rendering shared texture:', {
        pixelFormat: textureInfo.pixelFormat,
        codedSize: textureInfo.codedSize,
        hasNativePixmap: !!textureInfo.handle.nativePixmap
      });
      if (transform) {
        console.log('[myde-display] With transform:', transform.matrix);
      }
    }
    
    renderRegion(textureInfo, sourceRect, destRect, transform) {
      console.log('[myde-display] Rendering region:', {
        source: sourceRect,
        dest: destRect,
        hasTransform: !!transform
      });
    }
    
    renderBuffer(buffer, width, height, pixelFormat, transform) {
      console.log('[myde-display] Rendering buffer:', { width, height, pixelFormat });
    }
    
    close() {
      console.log('[myde-display] DRM device closed');
    }
  },
  MultiScreenRenderer: class {
    constructor() {
      this.devices = new Map();
    }
    
    addDevice(devicePath) {
      if (!this.devices.has(devicePath)) {
        const device = new mydeDisplay.DrmDevice(devicePath);
        this.devices.set(devicePath, device);
      }
      return this.devices.get(devicePath);
    }
    
    renderToMultipleScreens(textureInfo, mappings) {
      console.log('[myde-display] Multi-screen render:', {
        mappingsCount: mappings.length,
        mappings: mappings.map(m => ({
          source: m.sourceRect,
          target: m.target,
          hasTransform: !!m.transform
        }))
      });
    }
    
    closeAll() {
      for (const device of this.devices.values()) {
        device.close();
      }
      this.devices.clear();
    }
  },
  SharedTexture: {
    fromNativePixmap(nativePixmap, width, height, pixelFormat) {
      return {
        getTextureInfo: () => ({
          pixelFormat,
          codedSize: { width, height },
          handle: { nativePixmap }
        })
      };
    }
  },
  RegionMapper: class {
    constructor() {
      this.mappings = [];
    }
    
    addMapping(sourceRect, target, transform) {
      this.mappings.push({ sourceRect, target, transform });
      return this;
    }
    
    addGridMapping(sourceWidth, sourceHeight, columns, rows, screens, transforms) {
      const cellWidth = sourceWidth / columns;
      const cellHeight = sourceHeight / rows;
      
      for (let row = 0; row < rows; row++) {
        for (let col = 0; col < columns; col++) {
          const index = row * columns + col;
          if (index >= screens.length) break;
          
          this.mappings.push({
            sourceRect: {
              x: col * cellWidth,
              y: row * cellHeight,
              width: cellWidth,
              height: cellHeight
            },
            target: screens[index],
            transform: transforms?.[index]
          });
        }
      }
      return this;
    }
    
    addHorizontalSplit(sourceWidth, sourceHeight, screens, transforms) {
      const segmentWidth = sourceWidth / screens.length;
      
      for (let i = 0; i < screens.length; i++) {
        this.mappings.push({
          sourceRect: {
            x: i * segmentWidth,
            y: 0,
            width: segmentWidth,
            height: sourceHeight
          },
          target: screens[i],
          transform: transforms?.[i]
        });
      }
      return this;
    }
    
    addVerticalSplit(sourceWidth, sourceHeight, screens, transforms) {
      const segmentHeight = sourceHeight / screens.length;
      
      for (let i = 0; i < screens.length; i++) {
        this.mappings.push({
          sourceRect: {
            x: 0,
            y: i * segmentHeight,
            width: sourceWidth,
            height: segmentHeight
          },
          target: screens[i],
          transform: transforms?.[i]
        });
      }
      return this;
    }
    
    addCustomRegion(x, y, width, height, screenId, transform) {
      this.mappings.push({
        sourceRect: { x, y, width, height },
        target: { screenId },
        transform
      });
      return this;
    }
    
    getMappings() {
      return [...this.mappings];
    }
    
    clear() {
      this.mappings = [];
    }
  },
  TransformUtil: {
    rotation(angle, originX, originY) {
      const cos = Math.cos(angle);
      const sin = Math.sin(angle);
      const ox = originX || 0;
      const oy = originY || 0;
      return {
        matrix: [
          cos, -sin, ox * (1 - cos) + oy * sin,
          sin, cos, oy * (1 - cos) - ox * sin,
          0, 0, 1
        ]
      };
    },
    scale(sx, sy, originX, originY) {
      const ox = originX || 0;
      const oy = originY || 0;
      return {
        matrix: [
          sx, 0, ox * (1 - sx),
          0, sy || sx, oy * (1 - (sy || sx)),
          0, 0, 1
        ]
      };
    },
    translation(x, y) {
      return {
        matrix: [1, 0, x, 0, 1, y, 0, 0, 1]
      };
    },
    flip(horizontal, vertical) {
      return {
        matrix: [
          horizontal ? -1 : 1, 0, 0,
          0, vertical ? -1 : 1, 0,
          0, 0, 1
        ]
      };
    },
    compose(...transforms) {
      let result = [1, 0, 0, 0, 1, 0, 0, 0, 1];
      for (const t of transforms) {
        const m = t.matrix;
        const r = result;
        result = [
          m[0]*r[0] + m[1]*r[3] + m[2]*r[6],
          m[0]*r[1] + m[1]*r[4] + m[2]*r[7],
          m[0]*r[2] + m[1]*r[5] + m[2]*r[8],
          m[3]*r[0] + m[4]*r[3] + m[5]*r[6],
          m[3]*r[1] + m[4]*r[4] + m[5]*r[7],
          m[3]*r[2] + m[4]*r[5] + m[5]*r[8],
          m[6]*r[0] + m[7]*r[3] + m[8]*r[6],
          m[6]*r[1] + m[7]*r[4] + m[8]*r[7],
          m[6]*r[2] + m[7]*r[5] + m[8]*r[8]
        ];
      }
      return { matrix: result };
    }
  }
};

// Multi-screen renderer instance
let multiRenderer = null;

function createWindow() {
  // Get all available displays
  const displays = screen.getAllDisplays();
  console.log(`Found ${displays.length} display(s):`);
  displays.forEach((d, i) => {
    console.log(`  Display ${i}: ${d.bounds.width}x${d.bounds.height} at (${d.bounds.x}, ${d.bounds.y})`);
  });

  // Create the offscreen rendering window
  const osrWindow = new BrowserWindow({
    width: 1920,
    height: 1080,
    show: false,
    webPreferences: {
      offscreen: {
        useSharedTexture: true  // Enable shared texture mode
      },
      nodeIntegration: false,
      contextIsolation: true,
      preload: path.join(__dirname, 'preload.js')
    }
  });

  // Create display windows for each physical display
  const displayWindows = [];
  for (let i = 0; i < displays.length; i++) {
    const display = displays[i];
    const win = new BrowserWindow({
      x: display.bounds.x,
      y: display.bounds.y,
      width: display.bounds.width,
      height: display.bounds.height,
      fullscreen: true,
      show: true,
      webPreferences: {
        nodeIntegration: false,
        contextIsolation: true,
        preload: path.join(__dirname, 'preload.js')
      }
    });
    displayWindows.push(win);
  }

  // Initialize multi-screen renderer
  multiRenderer = new mydeDisplay.MultiScreenRenderer();
  
  // Add DRM devices
  try {
    multiRenderer.addDevice('/dev/dri/card0');
  } catch (error) {
    console.error('Failed to initialize DRM device:', error);
  }

  // Set frame rate for offscreen rendering
  osrWindow.webContents.setFrameRate(30);

  // Region mapping configuration
  let currentMappingMode = 'single'; // 'single', 'horizontal', 'vertical', 'grid', 'custom'
  
  // Handle paint events from offscreen renderer
  osrWindow.webContents.on('paint', (event, dirty, texture) => {
    if (!texture) {
      console.warn('No texture received in paint event');
      return;
    }

    const textureInfo = texture.textureInfo;
    const textureWidth = textureInfo.codedSize.width;
    const textureHeight = textureInfo.codedSize.height;

    // Create region mapper
    const mapper = new mydeDisplay.RegionMapper();

    switch (currentMappingMode) {
      case 'single':
        // Render full texture to first screen
        mapper.addCustomRegion(0, 0, textureWidth, textureHeight, 0);
        break;

      case 'horizontal':
        // Split texture horizontally across screens
        const hScreens = displays.map((_, i) => ({ screenId: i }));
        mapper.addHorizontalSplit(textureWidth, textureHeight, hScreens);
        break;

      case 'vertical':
        // Split texture vertically across screens
        const vScreens = displays.map((_, i) => ({ screenId: i }));
        mapper.addVerticalSplit(textureWidth, textureHeight, vScreens);
        break;

      case 'grid':
        // Split texture into 2x2 grid
        const gridScreens = [
          { screenId: 0 },
          { screenId: displays.length > 1 ? 1 : 0 },
          { screenId: displays.length > 2 ? 2 : 0 },
          { screenId: displays.length > 3 ? 3 : 0 },
        ];
        mapper.addGridMapping(textureWidth, textureHeight, 2, 2, gridScreens);
        break;

      case 'custom':
        // Custom regions with transforms
        mapper.addCustomRegion(
          0, 0,
          textureWidth / 2, textureHeight / 2,
          0,
          mydeDisplay.TransformUtil.rotation(Math.PI / 6, textureWidth / 4, textureHeight / 4)
        );
        
        if (displays.length > 1) {
          mapper.addCustomRegion(
            textureWidth / 2, 0,
            textureWidth / 2, textureHeight / 2,
            1,
            mydeDisplay.TransformUtil.scale(0.8, 0.8)
          );
        }
        
        mapper.addCustomRegion(
          0, textureHeight / 2,
          textureWidth, textureHeight / 2,
          0,
          mydeDisplay.TransformUtil.translation(0, 0)
        );
        break;
    }

    // Get mappings and render
    const mappings = mapper.getMappings();
    multiRenderer.renderToMultipleScreens(textureInfo, mappings);

    // Send to display windows for visualization
    displayWindows.forEach((win, i) => {
      win.webContents.send('texture-update', {
        screenId: i,
        textureInfo: textureInfo,
        dirty: dirty,
        mappingMode: currentMappingMode
      });
    });

    // Release the texture
    texture.release();
  });

  // Handle IPC from renderer
  ipcMain.on('set-mapping-mode', (event, mode) => {
    currentMappingMode = mode;
    console.log(`Mapping mode changed to: ${mode}`);
  });

  ipcMain.on('get-screen-info', (event) => {
    const screenInfos = displays.map((d, i) => ({
      id: i,
      bounds: d.bounds,
      workArea: d.workArea,
      scaleFactor: d.scaleFactor
    }));
    event.reply('screen-info', screenInfos);
  });

  ipcMain.on('get-displays', (event) => {
    event.reply('displays', displays);
  });

  // Load a webpage to render
  osrWindow.loadURL('https://github.com');
  
  // Load display pages
  displayWindows.forEach((win, i) => {
    win.loadFile(path.join(__dirname, 'display.html'));
  });

  return { osrWindow, displayWindows };
}

// App lifecycle
app.whenReady().then(() => {
  const windows = createWindow();

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow();
    }
  });
});

app.on('window-all-closed', () => {
  // Cleanup
  if (multiRenderer) {
    multiRenderer.closeAll();
    multiRenderer = null;
  }
  
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

// Handle app termination
app.on('before-quit', () => {
  if (multiRenderer) {
    multiRenderer.closeAll();
    multiRenderer = null;
  }
});