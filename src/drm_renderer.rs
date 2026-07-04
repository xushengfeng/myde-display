use anyhow::{Result, anyhow};
use drm::control::Device as ControlDevice;
use drm::Device;
use gbm::Device as GbmDevice;
use std::fs::{File, OpenOptions};
use std::os::unix::io::{AsFd, AsRawFd, BorrowedFd};
use std::path::Path;
use std::sync::Arc;

pub struct DrmDevice {
    file: File,
}

impl AsFd for DrmDevice {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.file.as_fd()
    }
}

impl AsRawFd for DrmDevice {
    fn as_raw_fd(&self) -> std::os::unix::io::RawFd {
        self.file.as_raw_fd()
    }
}

impl Clone for DrmDevice {
    fn clone(&self) -> Self {
        DrmDevice {
            file: self.file.try_clone().expect("Failed to clone file descriptor"),
        }
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
    pub supports_zero_copy_webgpu_import: bool,
}

pub struct SharedTextureHandle {
    pub native_pixmap: Option<NativePixmap>,
}

pub struct SharedTextureImportTextureInfo {
    pub pixel_format: PixelFormat,
    pub coded_width: u32,
    pub coded_height: u32,
    pub visible_rect: Option<Rectangle>,
    pub handle: SharedTextureHandle,
}

#[derive(Clone)]
pub struct Rectangle {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy)]
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
                        self.current_crtc = crtc;
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
                        let enc_id: u32 = encoder.handle().into();
                        let crtc_id: u32 = encoder.crtc().map(|c| c.into()).unwrap_or(0);
                        (enc_id, crtc_id)
                    } else {
                        (0, 0)
                    };
                    
                    let modes: Vec<DisplayMode> = connector.modes().iter().map(|mode| {
                        let (hdisplay, vdisplay) = mode.size();
                        let (hsync_start, hsync_end, htotal) = mode.hsync();
                        let (vsync_start, vsync_end, vtotal) = mode.vsync();
                        DisplayMode {
                            clock: mode.clock(),
                            hdisplay,
                            hsync_start,
                            hsync_end,
                            htotal,
                            hskew: mode.hskew(),
                            vdisplay,
                            vsync_start,
                            vsync_end,
                            vtotal,
                            vscan: mode.vscan(),
                            vrefresh: mode.vrefresh(),
                            flags: mode.flags().bits(),
                            type_: mode.mode_type().bits(),
                            name: mode.name().to_string_lossy().into_owned(),
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
                        _ => "unknown",
                    };
                    
                    let (width, height, refresh_rate) = if let Some(mode) = modes.first() {
                        (mode.hdisplay as u32, mode.vdisplay as u32, mode.vrefresh as f64)
                    } else {
                        (0, 0, 0.0)
                    };
                    
                    let connector_id: u32 = (*connector_handle).into();
                    let (physical_width, physical_height) = connector.size().unwrap_or((0, 0));
                    
                    screens.push(ScreenInfo {
                        connector_id,
                        encoder_id,
                        crtc_id,
                        width,
                        height,
                        refresh_rate,
                        is_connected: connector.state() == drm::control::connector::State::Connected,
                        modes,
                        physical_width,
                        physical_height,
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
        texture_info: &SharedTextureImportTextureInfo,
        transform: Option<[f64; 9]>,
    ) -> Result<()> {
        if let Some(native_pixmap) = &texture_info.handle.native_pixmap {
            self.render_native_pixmap(
                native_pixmap,
                texture_info.coded_width,
                texture_info.coded_height,
                texture_info.pixel_format,
                transform,
            )
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
        
        let (hdisplay, vdisplay) = mode.size();
        let (screen_width, screen_height) = (hdisplay as u32, vdisplay as u32);
        
        let mut buffer = self.gbm.create_buffer_object::<()>(
            screen_width,
            screen_height,
            gbm::Format::Xrgb8888,
            gbm::BufferObjectFlags::SCANOUT | gbm::BufferObjectFlags::WRITE,
        )?;
        
        let plane_data = self.map_dma_buf_planes(pixmap)?;
        
        buffer.map_mut(0, 0, screen_width, screen_height, |mapping| {
            let mapping_slice = mapping.buffer_mut();
            
            for pixel in mapping_slice.chunks_exact_mut(4) {
                pixel[0] = 0;
                pixel[1] = 0;
                pixel[2] = 0;
                pixel[3] = 0;
            }
            
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
        })?;
        
        let framebuffer = self.device.add_framebuffer(&buffer, 32, 32)?;
        
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
                
                use std::io::Read;
                let mut reader = std::io::BufReader::new(&file);
                reader.read_exact(&mut data)?;
                
                plane_data.push(data);
                
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
        
        let _plane_stride = if let Some(plane) = plane_data.first() {
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

    pub fn render_region(
        &mut self,
        texture_info: &SharedTextureImportTextureInfo,
        source_rect: &Rectangle,
        dest_rect: &Rectangle,
        transform: Option<[f64; 9]>,
    ) -> Result<()> {
        let connector = self.current_connector.ok_or_else(|| anyhow!("No connector"))?;
        let crtc = self.current_crtc.ok_or_else(|| anyhow!("No CRTC"))?;
        let mode = self.current_mode.ok_or_else(|| anyhow!("No mode"))?;
        
        let (hdisplay, vdisplay) = mode.size();
        let (screen_width, screen_height) = (hdisplay as u32, vdisplay as u32);
        
        let src_x = source_rect.x.min(texture_info.coded_width);
        let src_y = source_rect.y.min(texture_info.coded_height);
        let src_width = source_rect.width.min(texture_info.coded_width - src_x);
        let src_height = source_rect.height.min(texture_info.coded_height - src_y);
        
        let dst_x = dest_rect.x.min(screen_width);
        let dst_y = dest_rect.y.min(screen_height);
        let dst_width = dest_rect.width.min(screen_width - dst_x);
        let dst_height = dest_rect.height.min(screen_height - dst_y);
        
        let mut buffer = self.gbm.create_buffer_object::<()>(
            screen_width,
            screen_height,
            gbm::Format::Xrgb8888,
            gbm::BufferObjectFlags::SCANOUT | gbm::BufferObjectFlags::WRITE,
        )?;
        
        let native_pixmap_data = if let Some(native_pixmap) = &texture_info.handle.native_pixmap {
            Some(self.map_dma_buf_planes(native_pixmap)?)
        } else {
            None
        };
        
        let coded_width = texture_info.coded_width;
        let coded_height = texture_info.coded_height;
        let pixel_format = texture_info.pixel_format;
        
        buffer.map_mut(0, 0, screen_width, screen_height, |mapping| {
            let mapping_slice = mapping.buffer_mut();
            
            for pixel in mapping_slice.chunks_exact_mut(4) {
                pixel[0] = 0;
                pixel[1] = 0;
                pixel[2] = 0;
                pixel[3] = 0;
            }
            
            if let Some(plane_data) = &native_pixmap_data {
                self.render_region_from_mapped_data(
                    mapping_slice,
                    plane_data,
                    src_x,
                    src_y,
                    src_width,
                    src_height,
                    dst_x,
                    dst_y,
                    dst_width,
                    dst_height,
                    coded_width,
                    coded_height,
                    screen_width,
                    screen_height,
                    &pixel_format,
                    transform,
                );
            }
        })?;
        
        let framebuffer = self.device.add_framebuffer(&buffer, 32, 32)?;
        
        self.device.set_crtc(
            crtc,
            Some(framebuffer),
            (0, 0),
            &[connector],
            Some(mode),
        )?;
        
        Ok(())
    }

    fn render_region_from_mapped_data(
        &self,
        dst: &mut [u8],
        plane_data: &[Vec<u8>],
        src_x: u32,
        src_y: u32,
        src_width: u32,
        src_height: u32,
        dst_x: u32,
        dst_y: u32,
        dst_width: u32,
        dst_height: u32,
        full_src_width: u32,
        full_src_height: u32,
        full_dst_width: u32,
        _full_dst_height: u32,
        pixel_format: &PixelFormat,
        transform: Option<[f64; 9]>,
    ) {
        let bytes_per_pixel = match pixel_format {
            PixelFormat::Bgra | PixelFormat::Rgba => 4,
            PixelFormat::RgbaF16 => 8,
            PixelFormat::Nv12 | PixelFormat::Nv16 => 1,
            PixelFormat::P010Le => 2,
        };
        
        if let Some(plane) = plane_data.first() {
            for y in 0..dst_height {
                for x in 0..dst_width {
                    let (mapped_x, mapped_y) = if let Some(matrix) = transform {
                        self.apply_transform_to_point(matrix, x as f64, y as f64)
                    } else {
                        let scale_x = src_width as f64 / dst_width as f64;
                        let scale_y = src_height as f64 / dst_height as f64;
                        (x as f64 * scale_x, y as f64 * scale_y)
                    };
                    
                    let src_pixel_x = (mapped_x + src_x as f64) as i32;
                    let src_pixel_y = (mapped_y + src_y as f64) as i32;
                    
                    if src_pixel_x >= 0 && src_pixel_x < full_src_width as i32 &&
                       src_pixel_y >= 0 && src_pixel_y < full_src_height as i32 {
                        let src_offset = ((src_pixel_y as u32 * full_src_width + src_pixel_x as u32) * bytes_per_pixel) as usize;
                        
                        let dst_pixel_x = dst_x + x;
                        let dst_pixel_y = dst_y + y;
                        let dst_offset = ((dst_pixel_y * full_dst_width + dst_pixel_x) * 4) as usize;
                        
                        if src_offset + bytes_per_pixel as usize <= plane.len() && dst_offset + 3 < dst.len() {
                            match pixel_format {
                                PixelFormat::Rgba => {
                                    dst[dst_offset] = plane[src_offset];
                                    dst[dst_offset + 1] = plane[src_offset + 1];
                                    dst[dst_offset + 2] = plane[src_offset + 2];
                                    dst[dst_offset + 3] = plane[src_offset + 3];
                                }
                                PixelFormat::Bgra => {
                                    dst[dst_offset] = plane[src_offset + 2];
                                    dst[dst_offset + 1] = plane[src_offset + 1];
                                    dst[dst_offset + 2] = plane[src_offset];
                                    dst[dst_offset + 3] = plane[src_offset + 3];
                                }
                                _ => {
                                    dst[dst_offset] = plane[src_offset];
                                    dst[dst_offset + 1] = plane[src_offset];
                                    dst[dst_offset + 2] = plane[src_offset];
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
        
        let (hdisplay, vdisplay) = mode.size();
        let (screen_width, screen_height) = (hdisplay as u32, vdisplay as u32);
        
        let mut buffer = self.gbm.create_buffer_object::<()>(
            screen_width,
            screen_height,
            gbm::Format::Xrgb8888,
            gbm::BufferObjectFlags::SCANOUT | gbm::BufferObjectFlags::WRITE,
        )?;
        
        buffer.map_mut(0, 0, screen_width, screen_height, |mapping| {
            let mapping_slice = mapping.buffer_mut();
            
            for pixel in mapping_slice.chunks_exact_mut(4) {
                pixel[0] = 0;
                pixel[1] = 0;
                pixel[2] = 0;
                pixel[3] = 0;
            }
            
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
        })?;
        
        let framebuffer = self.device.add_framebuffer(&buffer, 32, 32)?;
        
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
