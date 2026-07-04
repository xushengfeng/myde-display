use anyhow::{Result, anyhow};
use std::collections::HashMap;
use uuid::Uuid;

pub struct Texture {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

pub struct TextureHandle {
    pub id: String,
    pub width: u32,
    pub height: u32,
}

pub struct TextureManager {
    textures: HashMap<String, Texture>,
}

impl TextureManager {
    pub fn new() -> Self {
        TextureManager {
            textures: HashMap::new(),
        }
    }

    pub fn create_texture(&mut self, width: u32, height: u32, data: &[u8]) -> Result<TextureHandle> {
        let expected_size = (width * height * 4) as usize;
        if data.len() != expected_size {
            return Err(anyhow!(
                "Invalid data size: expected {}, got {}",
                expected_size,
                data.len()
            ));
        }

        let id = Uuid::new_v4().to_string();
        let texture = Texture {
            id: id.clone(),
            width,
            height,
            data: data.to_vec(),
        };

        self.textures.insert(id.clone(), texture);

        Ok(TextureHandle {
            id,
            width,
            height,
        })
    }

    pub fn destroy_texture(&mut self, id: &str) {
        self.textures.remove(id);
    }

    pub fn update_texture(&mut self, id: &str, data: &[u8]) -> Result<()> {
        let texture = self.textures.get_mut(id)
            .ok_or_else(|| anyhow!("Texture not found: {}", id))?;

        let expected_size = (texture.width * texture.height * 4) as usize;
        if data.len() != expected_size {
            return Err(anyhow!(
                "Invalid data size: expected {}, got {}",
                expected_size,
                data.len()
            ));
        }

        texture.data = data.to_vec();
        Ok(())
    }

    pub fn get_texture(&self, id: &str) -> Option<&Texture> {
        self.textures.get(id)
    }

    pub fn get_texture_mut(&mut self, id: &str) -> Option<&mut Texture> {
        self.textures.get_mut(id)
    }

    pub fn list_textures(&self) -> Vec<TextureHandle> {
        self.textures.values().map(|t| TextureHandle {
            id: t.id.clone(),
            width: t.width,
            height: t.height,
        }).collect()
    }
}