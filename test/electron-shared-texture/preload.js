const { contextBridge, ipcRenderer } = require('electron');

// Expose protected methods that allow the renderer process to use
// ipcRenderer without exposing the entire object
contextBridge.exposeInMainWorld('electronAPI', {
  // Screen info
  getScreenInfo: () => {
    ipcRenderer.send('get-screen-info');
  },
  onScreenInfo: (callback) => {
    ipcRenderer.on('screen-info', (event, screens) => {
      callback(screens);
    });
  },
  
  // Transform controls
  applyTransform: (transformOptions) => {
    ipcRenderer.send('apply-transform', transformOptions);
  },
  
  // Texture updates
  onTextureUpdate: (callback) => {
    ipcRenderer.on('texture-update', (event, data) => {
      callback(data);
    });
  },
  
  // Utility
  removeAllListeners: (channel) => {
    ipcRenderer.removeAllListeners(channel);
  }
});