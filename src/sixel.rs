use anyhow::{anyhow, Result};
use icy_sixel::SixelImage;

pub fn encode_rgba(bytes: &[u8], width: u32, height: u32) -> Result<String> {
    let img = SixelImage::from_rgba(bytes.to_vec(), width as usize, height as usize);
    img.encode()
        .map_err(|e| anyhow!("sixel encode failed: {}", e))
}
