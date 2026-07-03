const { app, BrowserWindow, ipcMain } = require('electron');
const path = require('path');

// Import myde-display native module
// In real usage, this would be: const { DrmDevice, SharedTexture, TransformUtil } = require('myde-display');
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
    
    renderBuffer(buffer, width, height, pixelFormat, transform) {
      console.log('[myde-display] Rendering buffer:', { width, height, pixelFormat });
    }
    
    close() {
      console.log('[myde-display] DRM device closed');
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

// Create DRM device instance
let drmDevice = null;

function createWindow() {
  // Create the offscreen rendering window
  const osrWindow = new BrowserWindow({
    width: 800,
    height: 600,
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

  // Create a display window to show the rendered result (optional)
  const displayWindow = new BrowserWindow({
    width: 800,
    height: 600,
    show: true,
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      preload: path.join(__dirname, 'preload.js')
    }
  });

  // Initialize DRM device
  try {
    drmDevice = new mydeDisplay.DrmDevice('/dev/dri/card0');
    const screens = drmDevice.getScreenInfo();
    console.log('Available screens:', screens);
  } catch (error) {
    console.error('Failed to initialize DRM device:', error);
  }

  // Set frame rate for offscreen rendering
  osrWindow.webContents.setFrameRate(30);

  // Handle paint events from offscreen renderer
  osrWindow.webContents.on('paint', (event, dirty, texture) => {
    if (!texture) {
      console.warn('No texture received in paint event');
      return;
    }

    console.log('Paint event received:', {
      dirty,
      textureInfo: texture.textureInfo
    });

    // Get the shared texture info from Electron
    const textureInfo = texture.textureInfo;

    // Create SharedTexture from Electron's texture info
    const sharedTexture = mydeDisplay.SharedTexture.fromNativePixmap(
      textureInfo.handle.nativePixmap,
      textureInfo.codedSize.width,
      textureInfo.codedSize.height,
      textureInfo.pixelFormat
    );

    // Apply some transformations (e.g., rotate 45 degrees)
    const rotation = mydeDisplay.TransformUtil.rotation(
      Math.PI / 4,  // 45 degrees
      textureInfo.codedSize.width / 2,
      textureInfo.codedSize.height / 2
    );

    const scale = mydeDisplay.TransformUtil.scale(1.2, 1.2);

    const translation = mydeDisplay.TransformUtil.translation(100, 50);

    const transform = mydeDisplay.TransformUtil.compose(
      translation,
      scale,
      rotation
    );

    // Render to DRM display
    if (drmDevice) {
      try {
        drmDevice.renderSharedTexture(sharedTexture.getTextureInfo(), transform);
      } catch (error) {
        console.error('Failed to render to DRM:', error);
      }
    }

    // Also send to display window for visualization
    displayWindow.webContents.send('texture-update', {
      textureInfo: textureInfo,
      dirty: dirty
    });

    // Release the texture when done
    texture.release();
  });

  // Load a webpage to render
  osrWindow.loadURL('https://github.com');
  
  // Load display page
  displayWindow.loadFile(path.join(__dirname, 'display.html'));

  // Handle IPC from renderer
  ipcMain.on('get-screen-info', (event) => {
    if (drmDevice) {
      const screens = drmDevice.getScreenInfo();
      event.reply('screen-info', screens);
    }
  });

  ipcMain.on('apply-transform', (event, transformOptions) => {
    console.log('Applying transform:', transformOptions);
    // Transform will be applied in next paint event
  });

  return { osrWindow, displayWindow };
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
  // Cleanup DRM device
  if (drmDevice) {
    drmDevice.close();
    drmDevice = null;
  }
  
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

// Handle app termination
app.on('before-quit', () => {
  if (drmDevice) {
    drmDevice.close();
    drmDevice = null;
  }
});