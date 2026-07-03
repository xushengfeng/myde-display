use anyhow::{Result, anyhow};
use drm::control::Device as ControlDevice;
use drm::Device;
use drm_fourcc::DrmFormat;
use gbm::Device as GbmDevice;
use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::sync::Arc;

pub struct DrmDevice {
    file: File,
}

impl AsRawFd for DrmDevice {
    fn as_raw_fd(&self) -> std::os::unix::io::RawFd {
        self.file.as_raw_fd()
    }
}

impl Device for DrmDevice {}
impl ControlDevice for DrmDevice {}

pub struct ScreenInfo {
    pub connector_id: u32,
    pub encoder_id: u32,
    pub crtc_id: u32,
    pub width: u32,
    pub height: u32,
    pub refresh_rate: f64,
    pub is_connected: bool,
    pub modes: Vec<DisplayMode>,
    pub physical_width: u32,
    pub physical_height: u32,
    pub subpixel: String,
    pub connection: String,
}

pub struct DisplayMode {
    pub clock: u32,
    pub hdisplay: u16,
    pub hsync_start: u16,
    pub hsync_end: u16,
    pub htotal: u16,
    pub hskew: u16,
    pub vdisplay: u16,
    pub vsync_start: u16,
    pub vsync_end: u16,
    pub vtotal: u16,
    pub vscan: u16,
    pub vrefresh: u32,
    pub flags: u32,
    pub type_: u32,
    pub name: String,
}

pub struct SharedTexturePlane {
    pub stride: u32,
    pub offset: u64,
    pub size: u64,
    pub fd: i32,
}

pub struct NativePixmap {
    pub planes: Vec<SharedTexturePlane>,
    pub modifier: String,
    pub supportsZeroCopyWebGpuImport: bool,
}

pub struct SharedTextureHandle {
    pub nativePixmap: Option<NativePixmap>,
}

pub enum PixelFormat {
    Bgra,
    Rgba,
    RgbaF16,
    Nv12,
    Nv16,
    P010Le,
}

pub struct DrmRenderer {
    device: Arc<DrmDevice>,
    gbm: GbmDevice<DrmDevice>,
    current_connector: Option<drm::control::connector::Handle>,
    current_crtc: Option<drm::control::crtc::Handle>,
    current_mode: Option<drm::control::Mode>,
}

impl DrmRenderer {
    pub fn new(device_path: &str) -> Result<Self> {
        let path = Path::new(device_path);
        if !path.exists() {
            return Err(anyhow!("Device path does not exist: {}", device_path));
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)?;

        let device = DrmDevice { file };
        let gbm = GbmDevice::new(device.clone())?;

        let mut renderer = DrmRenderer {
            device: Arc::new(device),
            gbm,
            current_connector: None,
            current_crtc: None,
            current_mode: None,
        };

        renderer.initialize()?;
        Ok(renderer)
    }

    fn initialize(&mut self) -> Result<()> {
        let resources = self.device.resource_handles()?;
        
        for connector_handle in resources.connectors() {
            let connector = self.device.get_connector(*connector_handle, true)?;
            
            if connector.state() == drm::control::connector::State::Connected {
                let encoder = connector.current_encoder()
                    .and_then(|encoder_handle| self.device.get_encoder(encoder_handle).ok());
                
                if let Some(encoder) = encoder {
                    let crtc = encoder.crtc();
                    
                    if let Some(mode) = connector.modes().first() {
                        self.current_connector = Some(*connector_handle);
                        self.current_crtc = Some(crtc);
                        self.current_mode = Some(*mode);
                        return Ok(());
                    }
                }
            }
        }
        
        Err(anyhow!("No connected display found"))
    }

    pub fn get_screen_info(&self) -> Vec<ScreenInfo> {
        let mut screens = Vec::new();
        
        if let Ok(resources) = self.device.resource_handles() {
            for connector_handle in resources.connectors() {
                if let Ok(connector) = self.device.get_connector(*connector_handle, true) {
                    let encoder = connector.current_encoder()
                        .and_then(|encoder_handle| self.device.get_encoder(encoder_handle).ok());
                    
                    let (encoder_id, crtc_id) = if let Some(encoder) = encoder {
                        (encoder.handle().raw(), encoder.crtc().raw())
                    } else {
                        (0, 0)
                    };
                    
                    let modes: Vec<DisplayMode> = connector.modes().iter().map(|mode| {
                        DisplayMode {
                            clock: mode.clock(),
                            hdisplay: mode.hdisplay(),
                            hsync_start: mode.hsync_start(),
                            hsync_end: mode.hsync_end(),
                            htotal: mode.htotal(),
                            hskew: mode.hskew(),
                            vdisplay: mode.vdisplay(),
                            vsync_start: mode.vsync_start(),
                            vsync_end: mode.vsync_end(),
                            vtotal: mode.vtotal(),
                            vscan: mode.vscan(),
                            vrefresh: mode.vrefresh(),
                            flags: mode.flags(),
                            type_: mode.mode_type(),
                            name: mode.name().to_string(),
                        }
                    }).collect();
                    
                    let connection = match connector.state() {
                        drm::control::connector::State::Connected => "connected",
                        drm::control::connector::State::Disconnected => "disconnected",
                        drm::control::connector::State::Unknown => "unknown",
                    };
                    
                    let subpixel = match connector.subpixel() {
                        drm::control::connector::SubPixel::Unknown => "unknown",
                        drm::control::connector::SubPixel::HorizontalRgb => "horizontal_rgb",
                        drm::control::connector::SubPixel::HorizontalBgr => "horizontal_bgr",
                        drm::control::connector::SubPixel::VerticalRgb => "vertical_rgb",
                        drm::control::connector::SubPixel::VerticalBgr => "vertical_bgr",
                        drm::control::connector::SubPixel::None => "none",
                    };
                    
                    let (width, height, refresh_rate) = if let Some(mode) = modes.first() {
                        (mode.hdisplay as u32, mode.vdisplay as u32, mode.vrefresh as f64)
                    } else {
                        (0, 0, 0.0)
                    };
                    
                    screens.push(ScreenInfo {
                        connector_id: connector_handle.raw(),
                        encoder_id,
                        crtc_id,
                        width,
                        height,
                        refresh_rate,
                        is_connected: connector.state() == drm::control::connector::State::Connected,
                        modes,
                        physical_width: connector.size().0,
                        physical_height: connector.size().1,
                        subpixel: subpixel.to_string(),
                        connection: connection.to_string(),
                    });
                }
            }
        }
        
        screens
    }

    pub fn render_shared_texture(
        &mut self,
        handle: &SharedTextureHandle,
        width: u32,
        height: u32,
        pixel_format: PixelFormat,
        transform: Option<[f64; 9]>,
    ) -> Result<()> {
        if let Some(native_pixmap) = &handle.nativePixmap {
            self.render_native_pixmap(native_pixmap, width, height, pixel_format, transform)
        } else {
            Err(anyhow!("No valid texture handle provided"))
        }
    }

    fn render_native_pixmap(
        &mut self,
        pixmap: &NativePixmap,
        width: u32,
        height: u32,
        pixel_format: PixelFormat,
        transform: Option<[f64; 9]>,
    ) -> Result<()> {
        let connector = self.current_connector.ok_or_else(|| anyhow!("No connector"))?;
        let crtc = self.current_crtc.ok_or_else(|| anyhow!("No CRTC"))?;
        let mode = self.current_mode.ok_or_else(|| anyhow!("No mode"))?;
        
        let (screen_width, screen_height) = (mode.hdisplay() as u32, mode.vdisplay() as u32);
        
        let mut buffer = self.gbm.create_buffer_object::<()>(
            screen_width,
            screen_height,
            gbm::Format::Xrgb8888,
            gbm::BufferObjectFlags::SCANOUT | gbm::BufferObjectFlags::WRITE,
        )?;
        
        {
            let mut mapping = buffer.map_mut(&self.gbm)?;
            let mapping_slice = mapping.as_mut();
            
            // Clear buffer
            for pixel in mapping_slice.chunks_exact_mut(4) {
                pixel[0] = 0;
                pixel[1] = 0;
                pixel[2] = 0;
                pixel[3] = 0;
            }
            
            // Map the DMA-BUF planes
            let plane_data = self.map_dma_buf_planes(pixmap)?;
            
            // Render the texture
            self.render_mapped_data(
                mapping_slice,
                &plane_data,
                width,
                height,
                screen_width,
                screen_height,
                &pixel_format,
                transform,
            );
        }
        
        let framebuffer = self.gbm.add_framebuffer(&buffer, 32, 32)?;
        
        self.device.set_crtc(
            crtc,
            Some(framebuffer),
            (0, 0),
            &[connector],
            Some(mode),
        )?;
        
        Ok(())
    }

    fn map_dma_buf_planes(&self, pixmap: &NativePixmap) -> Result<Vec<Vec<u8>>> {
        use std::os::unix::io::FromRawFd;
        
        let mut plane_data = Vec::new();
        
        for plane in &pixmap.planes {
            unsafe {
                let file = File::from_raw_fd(plane.fd);
                let mut data = vec![0u8; plane.size as usize];
                
                // Read from the DMA-BUF file descriptor
                use std::io::Read;
                let mut reader = std::io::BufReader::new(&file);
                reader.read_exact(&mut data)?;
                
                plane_data.push(data);
                
                // Prevent the file from being closed
                std::mem::forget(file);
            }
        }
        
        Ok(plane_data)
    }

    fn render_mapped_data(
        &self,
        dst: &mut [u8],
        plane_data: &[Vec<u8>],
        src_width: u32,
        src_height: u32,
        dst_width: u32,
        dst_height: u32,
        pixel_format: &PixelFormat,
        transform: Option<[f64; 9]>,
    ) {
        let bytes_per_pixel = match pixel_format {
            PixelFormat::Bgra | PixelFormat::Rgba => 4,
            PixelFormat::RgbaF16 => 8,
            PixelFormat::Nv12 | PixelFormat::Nv16 => 1,
            PixelFormat::P010Le => 2,
        };
        
        let plane_stride = if let Some(plane) = plane_data.first() {
            (src_width * bytes_per_pixel) as usize
        } else {
            return;
        };
        
        for y in 0..dst_height {
            for x in 0..dst_width {
                let (src_x, src_y) = if let Some(matrix) = transform {
                    self.apply_transform_to_point(matrix, x as f64, y as f64)
                } else {
                    (x as f64, y as f64)
                };
                
                let src_x = src_x as i32;
                let src_y = src_y as i32;
                
                if src_x >= 0 && src_x < src_width as i32 && src_y >= 0 && src_y < src_height as i32 {
                    let src_offset = (src_y as u32 * src_width + src_x as u32) * bytes_per_pixel;
                    let dst_offset = ((y * dst_width + x) * 4) as usize;
                    
                    if let Some(plane) = plane_data.first() {
                        if (src_offset + bytes_per_pixel) as usize <= plane.len() && dst_offset + 3 < dst.len() {
                            match pixel_format {
                                PixelFormat::Rgba => {
                                    dst[dst_offset] = plane[src_offset as usize];
                                    dst[dst_offset + 1] = plane[src_offset as usize + 1];
                                    dst[dst_offset + 2] = plane[src_offset as usize + 2];
                                    dst[dst_offset + 3] = plane[src_offset as usize + 3];
                                }
                                PixelFormat::Bgra => {
                                    dst[dst_offset] = plane[src_offset as usize + 2];
                                    dst[dst_offset + 1] = plane[src_offset as usize + 1];
                                    dst[dst_offset + 2] = plane[src_offset as usize];
                                    dst[dst_offset + 3] = plane[src_offset as usize + 3];
                                }
                                _ => {
                                    // Simplified handling for other formats
                                    dst[dst_offset] = plane[src_offset as usize];
                                    dst[dst_offset + 1] = plane[src_offset as usize];
                                    dst[dst_offset + 2] = plane[src_offset as usize];
                                    dst[dst_offset + 3] = 255;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn render_buffer(
        &mut self,
        data: &[u8],
        width: u32,
        height: u32,
        pixel_format: PixelFormat,
        transform: Option<[f64; 9]>,
    ) -> Result<()> {
        let connector = self.current_connector.ok_or_else(|| anyhow!("No connector"))?;
        let crtc = self.current_crtc.ok_or_else(|| anyhow!("No CRTC"))?;
        let mode = self.current_mode.ok_or_else(|| anyhow!("No mode"))?;
        
        let (screen_width, screen_height) = (mode.hdisplay() as u32, mode.vdisplay() as u32);
        
        let mut buffer = self.gbm.create_buffer_object::<()>(
            screen_width,
            screen_height,
            gbm::Format::Xrgb8888,
            gbm::BufferObjectFlags::SCANOUT | gbm::BufferObjectFlags::WRITE,
        )?;
        
        {
            let mut mapping = buffer.map_mut(&self.gbm)?;
            let mapping_slice = mapping.as_mut();
            
            // Clear buffer
            for pixel in mapping_slice.chunks_exact_mut(4) {
                pixel[0] = 0;
                pixel[1] = 0;
                pixel[2] = 0;
                pixel[3] = 0;
            }
            
            // Render the buffer
            self.render_buffer_data(
                mapping_slice,
                data,
                width,
                height,
                screen_width,
                screen_height,
                &pixel_format,
                transform,
            );
        }
        
        let framebuffer = self.gbm.add_framebuffer(&buffer, 32, 32)?;
        
        self.device.set_crtc(
            crtc,
            Some(framebuffer),
            (0, 0),
            &[connector],
            Some(mode),
        )?;
        
        Ok(())
    }

    fn render_buffer_data(
        &self,
        dst: &mut [u8],
        src: &[u8],
        src_width: u32,
        src_height: u32,
        dst_width: u32,
        dst_height: u32,
        pixel_format: &PixelFormat,
        transform: Option<[f64; 9]>,
    ) {
        let bytes_per_pixel = match pixel_format {
            PixelFormat::Bgra | PixelFormat::Rgba => 4,
            PixelFormat::RgbaF16 => 8,
            PixelFormat::Nv12 | PixelFormat::Nv16 => 1,
            PixelFormat::P010Le => 2,
        };
        
        for y in 0..dst_height {
            for x in 0..dst_width {
                let (src_x, src_y) = if let Some(matrix) = transform {
                    self.apply_transform_to_point(matrix, x as f64, y as f64)
                } else {
                    (x as f64, y as f64)
                };
                
                let src_x = src_x as i32;
                let src_y = src_y as i32;
                
                if src_x >= 0 && src_x < src_width as i32 && src_y >= 0 && src_y < src_height as i32 {
                    let src_offset = ((src_y as u32 * src_width + src_x as u32) * bytes_per_pixel) as usize;
                    let dst_offset = ((y * dst_width + x) * 4) as usize;
                    
                    if src_offset + bytes_per_pixel as usize <= src.len() && dst_offset + 3 < dst.len() {
                        match pixel_format {
                            PixelFormat::Rgba => {
                                dst[dst_offset] = src[src_offset];
                                dst[dst_offset + 1] = src[src_offset + 1];
                                dst[dst_offset + 2] = src[src_offset + 2];
                                dst[dst_offset + 3] = src[src_offset + 3];
                            }
                            PixelFormat::Bgra => {
                                dst[dst_offset] = src[src_offset + 2];
                                dst[dst_offset + 1] = src[src_offset + 1];
                                dst[dst_offset + 2] = src[src_offset];
                                dst[dst_offset + 3] = src[src_offset + 3];
                            }
                            _ => {
                                dst[dst_offset] = src[src_offset];
                                dst[dst_offset + 1] = src[src_offset];
                                dst[dst_offset + 2] = src[src_offset];
                                dst[dst_offset + 3] = 255;
                            }
                        }
                    }
                }
            }
        }
    }

    fn apply_transform_to_point(&self, matrix: [f64; 9], x: f64, y: f64) -> (f64, f64) {
        let w = matrix[6] * x + matrix[7] * y + matrix[8];
        if w.abs() < f64::EPSILON {
            return (0.0, 0.0);
        }
        
        let new_x = (matrix[0] * x + matrix[1] * y + matrix[2]) / w;
        let new_y = (matrix[3] * x + matrix[4] * y + matrix[5]) / w;
        
        (new_x, new_y)
    }
}