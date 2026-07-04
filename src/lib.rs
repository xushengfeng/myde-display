use neon::prelude::*;
use std::sync::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

mod drm_renderer;
mod texture_manager;
mod transform;

use drm_renderer::DrmRenderer;
use texture_manager::TextureManager;
use transform::TransformManager;

lazy_static::lazy_static! {
    static ref DRM_RENDERERS: Mutex<HashMap<String, Arc<Mutex<DrmRenderer>>>> = 
        Mutex::new(HashMap::new());
    static ref TEXTURE_MANAGER: Mutex<TextureManager> = 
        Mutex::new(TextureManager::new());
    static ref TRANSFORM_MANAGER: Mutex<TransformManager> = 
        Mutex::new(TransformManager::new());
}

#[derive(Clone)]
struct SharedTexturePlane {
    stride: u32,
    offset: u64,
    size: u64,
    fd: i32,
}

#[derive(Clone)]
struct NativePixmap {
    planes: Vec<SharedTexturePlane>,
    modifier: String,
    supports_zero_copy_webgpu_import: bool,
}

#[derive(Clone)]
struct SharedTextureHandle {
    native_pixmap: Option<NativePixmap>,
}

#[derive(Clone)]
struct SharedTextureImportTextureInfo {
    pixel_format: PixelFormat,
    coded_width: u32,
    coded_height: u32,
    visible_rect: Option<Rectangle>,
    handle: SharedTextureHandle,
}

#[derive(Clone)]
struct Rectangle {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[derive(Clone)]
struct ScreenTarget {
    screen_id: u32,
    connector_id: Option<u32>,
    dest_x: Option<u32>,
    dest_y: Option<u32>,
    dest_width: Option<u32>,
    dest_height: Option<u32>,
}

#[derive(Clone)]
struct RegionMapping {
    source_rect: Rectangle,
    target: ScreenTarget,
    transform: Option<[f64; 9]>,
}

#[derive(Clone, Copy)]
enum PixelFormat {
    Bgra,
    Rgba,
    RgbaF16,
    Nv12,
    Nv16,
    P010Le,
}

fn open_drm_device(mut cx: FunctionContext) -> JsResult<JsObject> {
    let device_path = cx.argument_opt(0)
        .and_then(|v| v.downcast::<JsString, _>(&mut cx).ok())
        .map(|s| s.value(&mut cx))
        .unwrap_or_else(|| "/dev/dri/card0".to_string());

    let mut renderers = DRM_RENDERERS.lock().unwrap();
    
    if renderers.contains_key(&device_path) {
        let handle = cx.empty_object();
        let id = cx.string(&device_path);
        let path = cx.string(&device_path);
        handle.set(&mut cx, "id", id)?;
        handle.set(&mut cx, "devicePath", path)?;
        return Ok(handle);
    }

    match DrmRenderer::new(&device_path) {
        Ok(renderer) => {
            let id = device_path.clone();
            renderers.insert(device_path.clone(), Arc::new(Mutex::new(renderer)));
            
            let handle = cx.empty_object();
            let id_js = cx.string(&id);
            let path_js = cx.string(&device_path);
            handle.set(&mut cx, "id", id_js)?;
            handle.set(&mut cx, "devicePath", path_js)?;
            Ok(handle)
        }
        Err(e) => cx.throw_error(format!("Failed to open DRM device: {}", e)),
    }
}

fn close_drm_device(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let handle = cx.argument::<JsObject>(0)?;
    let id = handle.get::<JsString, _, _>(&mut cx, "id")?.value(&mut cx);
    
    let mut renderers = DRM_RENDERERS.lock().unwrap();
    renderers.remove(&id);
    
    Ok(cx.undefined())
}

fn get_screen_info(mut cx: FunctionContext) -> JsResult<JsArray> {
    let handle = cx.argument::<JsObject>(0)?;
    let id = handle.get::<JsString, _, _>(&mut cx, "id")?.value(&mut cx);
    
    let renderers = DRM_RENDERERS.lock().unwrap();
    let renderer = renderers.get(&id)
        .ok_or_else(|| cx.throw_error::<_, ()>("Device not found").unwrap_err())?;
    
    let renderer = renderer.lock().unwrap();
    let screens = renderer.get_screen_info();
    
    let js_array = cx.empty_array();
    for (i, screen) in screens.iter().enumerate() {
        let js_screen = cx.empty_object();
        
        let connector_id = cx.number(screen.connector_id as f64);
        let encoder_id = cx.number(screen.encoder_id as f64);
        let crtc_id = cx.number(screen.crtc_id as f64);
        let width = cx.number(screen.width as f64);
        let height = cx.number(screen.height as f64);
        let refresh_rate = cx.number(screen.refresh_rate as f64);
        let is_connected = cx.boolean(screen.is_connected);
        let physical_width = cx.number(screen.physical_width as f64);
        let physical_height = cx.number(screen.physical_height as f64);
        let subpixel = cx.string(&screen.subpixel);
        let connection = cx.string(&screen.connection);
        
        js_screen.set(&mut cx, "connectorId", connector_id)?;
        js_screen.set(&mut cx, "encoderId", encoder_id)?;
        js_screen.set(&mut cx, "crtcId", crtc_id)?;
        js_screen.set(&mut cx, "width", width)?;
        js_screen.set(&mut cx, "height", height)?;
        js_screen.set(&mut cx, "refreshRate", refresh_rate)?;
        js_screen.set(&mut cx, "isConnected", is_connected)?;
        js_screen.set(&mut cx, "physicalWidth", physical_width)?;
        js_screen.set(&mut cx, "physicalHeight", physical_height)?;
        js_screen.set(&mut cx, "subpixel", subpixel)?;
        js_screen.set(&mut cx, "connection", connection)?;
        
        let modes = cx.empty_array();
        for (j, mode) in screen.modes.iter().enumerate() {
            let js_mode = cx.empty_object();
            
            let clock = cx.number(mode.clock as f64);
            let hdisplay = cx.number(mode.hdisplay as f64);
            let hsync_start = cx.number(mode.hsync_start as f64);
            let hsync_end = cx.number(mode.hsync_end as f64);
            let htotal = cx.number(mode.htotal as f64);
            let hskew = cx.number(mode.hskew as f64);
            let vdisplay = cx.number(mode.vdisplay as f64);
            let vsync_start = cx.number(mode.vsync_start as f64);
            let vsync_end = cx.number(mode.vsync_end as f64);
            let vtotal = cx.number(mode.vtotal as f64);
            let vscan = cx.number(mode.vscan as f64);
            let vrefresh = cx.number(mode.vrefresh as f64);
            let flags = cx.number(mode.flags as f64);
            let type_ = cx.number(mode.type_ as f64);
            let name = cx.string(&mode.name);
            
            js_mode.set(&mut cx, "clock", clock)?;
            js_mode.set(&mut cx, "hdisplay", hdisplay)?;
            js_mode.set(&mut cx, "hsyncStart", hsync_start)?;
            js_mode.set(&mut cx, "hsyncEnd", hsync_end)?;
            js_mode.set(&mut cx, "htotal", htotal)?;
            js_mode.set(&mut cx, "hskew", hskew)?;
            js_mode.set(&mut cx, "vdisplay", vdisplay)?;
            js_mode.set(&mut cx, "vsyncStart", vsync_start)?;
            js_mode.set(&mut cx, "vsyncEnd", vsync_end)?;
            js_mode.set(&mut cx, "vtotal", vtotal)?;
            js_mode.set(&mut cx, "vscan", vscan)?;
            js_mode.set(&mut cx, "vrefresh", vrefresh)?;
            js_mode.set(&mut cx, "flags", flags)?;
            js_mode.set(&mut cx, "type", type_)?;
            js_mode.set(&mut cx, "name", name)?;
            
            modes.set(&mut cx, j as u32, js_mode)?;
        }
        
        js_screen.set(&mut cx, "modes", modes)?;
        js_array.set(&mut cx, i as u32, js_screen)?;
    }
    
    Ok(js_array)
}

fn parse_rectangle(cx: &mut FunctionContext, obj: JsObject) -> NeonResult<Rectangle> {
    let x = obj.get::<JsNumber, _, _>(cx, "x")?.value(cx) as u32;
    let y = obj.get::<JsNumber, _, _>(cx, "y")?.value(cx) as u32;
    let width = obj.get::<JsNumber, _, _>(cx, "width")?.value(cx) as u32;
    let height = obj.get::<JsNumber, _, _>(cx, "height")?.value(cx) as u32;
    
    Ok(Rectangle { x, y, width, height })
}

fn parse_screen_target(cx: &mut FunctionContext, obj: JsObject) -> NeonResult<ScreenTarget> {
    let screen_id = obj.get::<JsNumber, _, _>(cx, "screenId")?.value(cx) as u32;
    let connector_id = obj.get_opt::<JsNumber, _, _>(cx, "connectorId")?
        .map(|v| v.value(cx) as u32);
    let dest_x = obj.get_opt::<JsNumber, _, _>(cx, "destX")?
        .map(|v| v.value(cx) as u32);
    let dest_y = obj.get_opt::<JsNumber, _, _>(cx, "destY")?
        .map(|v| v.value(cx) as u32);
    let dest_width = obj.get_opt::<JsNumber, _, _>(cx, "destWidth")?
        .map(|v| v.value(cx) as u32);
    let dest_height = obj.get_opt::<JsNumber, _, _>(cx, "destHeight")?
        .map(|v| v.value(cx) as u32);
    
    Ok(ScreenTarget {
        screen_id,
        connector_id,
        dest_x,
        dest_y,
        dest_width,
        dest_height,
    })
}

fn parse_transform_matrix(cx: &mut FunctionContext, transform_obj: JsObject) -> NeonResult<Option<[f64; 9]>> {
    let matrix = transform_obj.get::<JsArray, _, _>(cx, "matrix")?;
    let mut transform_matrix = [0.0f64; 9];
    for i in 0..9 {
        let val = matrix.get::<JsNumber, _, _>(cx, i as u32)?;
        transform_matrix[i] = val.value(cx);
    }
    Ok(Some(transform_matrix))
}

fn parse_shared_texture_handle(cx: &mut FunctionContext, handle_obj: JsObject) -> NeonResult<SharedTextureHandle> {
    let mut handle = SharedTextureHandle { native_pixmap: None };
    
    if let Ok(native_pixmap) = handle_obj.get::<JsObject, _, _>(cx, "nativePixmap") {
        let planes_array = native_pixmap.get::<JsArray, _, _>(cx, "planes")?;
        let planes_len = planes_array.len(cx);
        
        let mut planes = Vec::new();
        for i in 0..planes_len {
            let plane_obj = planes_array.get::<JsObject, _, _>(cx, i)?;
            let stride = plane_obj.get::<JsNumber, _, _>(cx, "stride")?.value(cx) as u32;
            let offset = plane_obj.get::<JsNumber, _, _>(cx, "offset")?.value(cx) as u64;
            let size = plane_obj.get::<JsNumber, _, _>(cx, "size")?.value(cx) as u64;
            let fd = plane_obj.get::<JsNumber, _, _>(cx, "fd")?.value(cx) as i32;
            
            planes.push(SharedTexturePlane {
                stride,
                offset,
                size,
                fd,
            });
        }
        
        let modifier = native_pixmap.get::<JsString, _, _>(cx, "modifier")?.value(cx);
        let supports_zero_copy = native_pixmap.get::<JsBoolean, _, _>(cx, "supportsZeroCopyWebGpuImport")?.value(cx);
        
        handle.native_pixmap = Some(NativePixmap {
            planes,
            modifier,
            supports_zero_copy_webgpu_import: supports_zero_copy,
        });
    }
    
    Ok(handle)
}

fn parse_texture_info(cx: &mut FunctionContext, texture_info: JsObject) -> NeonResult<SharedTextureImportTextureInfo> {
    let pixel_format_str = texture_info.get::<JsString, _, _>(cx, "pixelFormat")?.value(cx);
    let coded_size = texture_info.get::<JsObject, _, _>(cx, "codedSize")?;
    let width = coded_size.get::<JsNumber, _, _>(cx, "width")?.value(cx) as u32;
    let height = coded_size.get::<JsNumber, _, _>(cx, "height")?.value(cx) as u32;
    
    let visible_rect = if let Ok(rect) = texture_info.get::<JsObject, _, _>(cx, "visibleRect") {
        Some(parse_rectangle(cx, rect)?)
    } else {
        None
    };
    
    let handle_obj = texture_info.get::<JsObject, _, _>(cx, "handle")?;
    let handle = parse_shared_texture_handle(cx, handle_obj)?;
    
    let pixel_format = match pixel_format_str.as_str() {
        "bgra" => PixelFormat::Bgra,
        "rgba" => PixelFormat::Rgba,
        "rgbaf16" => PixelFormat::RgbaF16,
        "nv12" => PixelFormat::Nv12,
        "nv16" => PixelFormat::Nv16,
        "p010le" => PixelFormat::P010Le,
        _ => return cx.throw_error(format!("Unsupported pixel format: {}", pixel_format_str)),
    };
    
    Ok(SharedTextureImportTextureInfo {
        pixel_format,
        coded_width: width,
        coded_height: height,
        visible_rect,
        handle,
    })
}

fn render_to_screen(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let device_handle = cx.argument::<JsObject>(0)?;
    let texture_info = cx.argument::<JsObject>(1)?;
    let transform = cx.argument_opt(2)
        .and_then(|v| v.downcast::<JsObject, _>(&mut cx).ok());
    
    let device_id = device_handle.get::<JsString, _, _>(&mut cx, "id")?.value(&mut cx);
    
    let renderers = DRM_RENDERERS.lock().unwrap();
    let renderer = renderers.get(&device_id)
        .ok_or_else(|| cx.throw_error::<_, ()>("Device not found").unwrap_err())?;
    
    let mut renderer = renderer.lock().unwrap();
    
    let tex_info = parse_texture_info(&mut cx, texture_info)?;
    
    let transform_data = if let Some(transform_obj) = transform {
        parse_transform_matrix(&mut cx, transform_obj)?
    } else {
        None
    };
    
    match renderer.render_shared_texture(&tex_info, transform_data) {
        Ok(_) => Ok(cx.undefined()),
        Err(e) => cx.throw_error(format!("Failed to render: {}", e)),
    }
}

fn render_buffer_to_screen(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let device_handle = cx.argument::<JsObject>(0)?;
    let buffer = cx.argument::<JsBuffer>(1)?;
    let width = cx.argument::<JsNumber>(2)?.value(&mut cx) as u32;
    let height = cx.argument::<JsNumber>(3)?.value(&mut cx) as u32;
    let pixel_format = cx.argument::<JsString>(4)?.value(&mut cx);
    let transform = cx.argument_opt(5)
        .and_then(|v| v.downcast::<JsObject, _>(&mut cx).ok());
    
    let device_id = device_handle.get::<JsString, _, _>(&mut cx, "id")?.value(&mut cx);
    
    let renderers = DRM_RENDERERS.lock().unwrap();
    let renderer = renderers.get(&device_id)
        .ok_or_else(|| cx.throw_error::<_, ()>("Device not found").unwrap_err())?;
    
    let mut renderer = renderer.lock().unwrap();
    let data_slice = buffer.as_slice(&cx);
    
    let transform_data = if let Some(transform_obj) = transform {
        parse_transform_matrix(&mut cx, transform_obj)?
    } else {
        None
    };
    
    let pixel_format_enum = match pixel_format.as_str() {
        "bgra" => PixelFormat::Bgra,
        "rgba" => PixelFormat::Rgba,
        "rgbaf16" => PixelFormat::RgbaF16,
        "nv12" => PixelFormat::Nv12,
        "nv16" => PixelFormat::Nv16,
        "p010le" => PixelFormat::P010Le,
        _ => return cx.throw_error(format!("Unsupported pixel format: {}", pixel_format)),
    };
    
    match renderer.render_buffer(data_slice, width, height, pixel_format_enum, transform_data) {
        Ok(_) => Ok(cx.undefined()),
        Err(e) => cx.throw_error(format!("Failed to render buffer: {}", e)),
    }
}

fn render_region_to_screen(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let device_handle = cx.argument::<JsObject>(0)?;
    let texture_info = cx.argument::<JsObject>(1)?;
    let source_rect = cx.argument::<JsObject>(2)?;
    let dest_rect = cx.argument::<JsObject>(3)?;
    let transform = cx.argument_opt(4)
        .and_then(|v| v.downcast::<JsObject, _>(&mut cx).ok());
    
    let device_id = device_handle.get::<JsString, _, _>(&mut cx, "id")?.value(&mut cx);
    
    let renderers = DRM_RENDERERS.lock().unwrap();
    let renderer = renderers.get(&device_id)
        .ok_or_else(|| cx.throw_error::<_, ()>("Device not found").unwrap_err())?;
    
    let mut renderer = renderer.lock().unwrap();
    
    let tex_info = parse_texture_info(&mut cx, texture_info)?;
    let src_rect = parse_rectangle(&mut cx, source_rect)?;
    let dst_rect = parse_rectangle(&mut cx, dest_rect)?;
    
    let transform_data = if let Some(transform_obj) = transform {
        parse_transform_matrix(&mut cx, transform_obj)?
    } else {
        None
    };
    
    match renderer.render_region(&tex_info, &src_rect, &dst_rect, transform_data) {
        Ok(_) => Ok(cx.undefined()),
        Err(e) => cx.throw_error(format!("Failed to render region: {}", e)),
    }
}

fn render_multi_screen(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    let device_handles = cx.argument::<JsArray>(0)?;
    let texture_info = cx.argument::<JsObject>(1)?;
    let mappings_array = cx.argument::<JsArray>(2)?;
    
    let handles_len = device_handles.len(&mut cx);
    let mappings_len = mappings_array.len(&mut cx);
    
    let mut handles = Vec::new();
    for i in 0..handles_len {
        let handle = device_handles.get::<JsObject, _, _>(&mut cx, i)?;
        let id = handle.get::<JsString, _, _>(&mut cx, "id")?.value(&mut cx);
        handles.push(id);
    }
    
    let tex_info = parse_texture_info(&mut cx, texture_info)?;
    
    let mut mappings = Vec::new();
    for i in 0..mappings_len {
        let mapping = mappings_array.get::<JsObject, _, _>(&mut cx, i)?;
        
        let source_rect = mapping.get::<JsObject, _, _>(&mut cx, "sourceRect")?;
        let src_rect = parse_rectangle(&mut cx, source_rect)?;
        
        let target = mapping.get::<JsObject, _, _>(&mut cx, "target")?;
        let screen_target = parse_screen_target(&mut cx, target)?;
        
        let transform = if let Ok(transform_obj) = mapping.get::<JsObject, _, _>(&mut cx, "transform") {
            parse_transform_matrix(&mut cx, transform_obj)?
        } else {
            None
        };
        
        mappings.push(RegionMapping {
            source_rect: src_rect,
            target: screen_target,
            transform,
        });
    }
    
    let renderers = DRM_RENDERERS.lock().unwrap();
    
    for mapping in &mappings {
        let screen_id = mapping.target.screen_id as usize;
        if screen_id >= handles.len() {
            continue;
        }
        
        let device_id = &handles[screen_id];
        if let Some(renderer) = renderers.get(device_id) {
            let mut renderer = renderer.lock().unwrap();
            
            let dest_rect = Rectangle {
                x: mapping.target.dest_x.unwrap_or(0),
                y: mapping.target.dest_y.unwrap_or(0),
                width: mapping.target.dest_width.unwrap_or(mapping.source_rect.width),
                height: mapping.target.dest_height.unwrap_or(mapping.source_rect.height),
            };
            
            let _ = renderer.render_region(&tex_info, &mapping.source_rect, &dest_rect, mapping.transform);
        }
    }
    
    Ok(cx.undefined())
}

fn create_transform(mut cx: FunctionContext) -> JsResult<JsObject> {
    let options = cx.argument::<JsObject>(0)?;
    
    let rotation = options.get_opt::<JsNumber, _, _>(&mut cx, "rotation")?
        .map(|v| v.value(&mut cx))
        .unwrap_or(0.0);
    
    let scale_x = options.get_opt::<JsNumber, _, _>(&mut cx, "scaleX")?
        .map(|v| v.value(&mut cx))
        .unwrap_or(1.0);
    
    let scale_y = options.get_opt::<JsNumber, _, _>(&mut cx, "scaleY")?
        .map(|v| v.value(&mut cx))
        .unwrap_or(1.0);
    
    let translate_x = options.get_opt::<JsNumber, _, _>(&mut cx, "translateX")?
        .map(|v| v.value(&mut cx))
        .unwrap_or(0.0);
    
    let translate_y = options.get_opt::<JsNumber, _, _>(&mut cx, "translateY")?
        .map(|v| v.value(&mut cx))
        .unwrap_or(0.0);
    
    let origin_x = options.get_opt::<JsNumber, _, _>(&mut cx, "originX")?
        .map(|v| v.value(&mut cx))
        .unwrap_or(0.0);
    
    let origin_y = options.get_opt::<JsNumber, _, _>(&mut cx, "originY")?
        .map(|v| v.value(&mut cx))
        .unwrap_or(0.0);
    
    let mut manager = TRANSFORM_MANAGER.lock().unwrap();
    let transform = manager.create_transform(rotation, scale_x, scale_y, translate_x, translate_y, origin_x, origin_y);
    
    let js_transform = cx.empty_object();
    let matrix = cx.empty_array();
    
    for (i, val) in transform.matrix.iter().enumerate() {
        let js_val = cx.number(*val);
        matrix.set(&mut cx, i as u32, js_val)?;
    }
    
    js_transform.set(&mut cx, "matrix", matrix)?;
    Ok(js_transform)
}

fn apply_transform(mut cx: FunctionContext) -> JsResult<JsObject> {
    let transform = cx.argument::<JsObject>(0)?;
    let point = cx.argument::<JsObject>(1)?;
    
    let x = point.get::<JsNumber, _, _>(&mut cx, "x")?.value(&mut cx);
    let y = point.get::<JsNumber, _, _>(&mut cx, "y")?.value(&mut cx);
    
    let matrix = transform.get::<JsArray, _, _>(&mut cx, "matrix")?;
    let mut transform_matrix = [0.0f64; 9];
    for i in 0..9 {
        let val = matrix.get::<JsNumber, _, _>(&mut cx, i as u32)?;
        transform_matrix[i] = val.value(&mut cx);
    }
    
    let manager = TRANSFORM_MANAGER.lock().unwrap();
    let (new_x, new_y) = manager.apply_transform(&transform_matrix, x, y);
    
    let js_point = cx.empty_object();
    let js_x = cx.number(new_x);
    let js_y = cx.number(new_y);
    
    js_point.set(&mut cx, "x", js_x)?;
    js_point.set(&mut cx, "y", js_y)?;
    Ok(js_point)
}

fn compose_transforms(mut cx: FunctionContext) -> JsResult<JsObject> {
    let transforms = cx.argument::<JsArray>(0)?;
    let length = transforms.len(&mut cx);
    
    let mut matrices = Vec::new();
    for i in 0..length {
        let transform = transforms.get::<JsObject, _, _>(&mut cx, i)?;
        let matrix = transform.get::<JsArray, _, _>(&mut cx, "matrix")?;
        let mut transform_matrix = [0.0f64; 9];
        for j in 0..9 {
            let val = matrix.get::<JsNumber, _, _>(&mut cx, j)?;
            transform_matrix[j] = val.value(&mut cx);
        }
        matrices.push(transform_matrix);
    }
    
    let manager = TRANSFORM_MANAGER.lock().unwrap();
    let result = manager.compose_transforms(&matrices);
    
    let js_transform = cx.empty_object();
    let js_matrix = cx.empty_array();
    
    for (i, val) in result.iter().enumerate() {
        let js_val = cx.number(*val);
        js_matrix.set(&mut cx, i as u32, js_val)?;
    }
    
    js_transform.set(&mut cx, "matrix", js_matrix)?;
    Ok(js_transform)
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    env_logger::init();
    
    cx.export_function("openDrmDevice", open_drm_device)?;
    cx.export_function("closeDrmDevice", close_drm_device)?;
    cx.export_function("getScreenInfo", get_screen_info)?;
    cx.export_function("renderToScreen", render_to_screen)?;
    cx.export_function("renderBufferToScreen", render_buffer_to_screen)?;
    cx.export_function("renderRegionToScreen", render_region_to_screen)?;
    cx.export_function("renderMultiScreen", render_multi_screen)?;
    cx.export_function("createTransform", create_transform)?;
    cx.export_function("applyTransform", apply_transform)?;
    cx.export_function("composeTransforms", compose_transforms)?;
    
    Ok(())
}