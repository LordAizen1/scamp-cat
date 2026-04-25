use image::{GenericImageView, Rgba};

#[derive(Clone, Copy)]
pub struct SpriteCell {
    pub ch: char,
    pub fg: Option<(u8, u8, u8)>,
    pub bg: Option<(u8, u8, u8)>,
    pub transparent: bool,
}

pub struct Frame {
    pub width_cells: u16,
    pub height_cells: u16,
    pub cells: Vec<SpriteCell>,
}

impl Frame {
    pub fn cell_at(&self, row: u16, col: u16) -> SpriteCell {
        let i = (row as usize) * (self.width_cells as usize) + col as usize;
        self.cells[i]
    }
}

const SHEET_PNG: &[u8] = include_bytes!("../assets/sprites/cat.png");
const FRAME_PIXELS: u32 = 32;
// Halve source pixels before rendering so the terminal sprite isn't huge.
// 1 = full size, 2 = half, 4 = quarter. Frame coords stay in source pixels.
const SOURCE_SCALE_DOWN: u32 = 2;

pub fn load_frames(frame_coords: &[(u32, u32)]) -> anyhow::Result<Vec<Frame>> {
    let groups = load_animation_groups(&[frame_coords])?;
    Ok(groups.into_iter().next().unwrap())
}

// Load N animations and crop ALL of them to a single shared bounding box,
// so every frame across every animation has the same width × height. The
// compositor relies on this — sprite_width/height are constants per pet.
pub fn load_animation_groups(coord_groups: &[&[(u32, u32)]]) -> anyhow::Result<Vec<Vec<Frame>>> {
    let img = image::load_from_memory(SHEET_PNG)?;
    let img = if SOURCE_SCALE_DOWN > 1 {
        img.resize_exact(
            img.width() / SOURCE_SCALE_DOWN,
            img.height() / SOURCE_SCALE_DOWN,
            image::imageops::FilterType::Lanczos3,
        )
    } else {
        img
    };
    let frame_pixels = FRAME_PIXELS / SOURCE_SCALE_DOWN;
    let mut groups: Vec<Vec<Frame>> = Vec::with_capacity(coord_groups.len());
    for coords in coord_groups {
        let mut group = Vec::with_capacity(coords.len());
        for &(x, y) in *coords {
            let sx = x / SOURCE_SCALE_DOWN;
            let sy = y / SOURCE_SCALE_DOWN;
            group.push(extract_frame(&img, sx, sy, frame_pixels, frame_pixels));
        }
        groups.push(group);
    }
    Ok(crop_groups_to_global_union(groups))
}

fn crop_groups_to_global_union(groups: Vec<Vec<Frame>>) -> Vec<Vec<Frame>> {
    if groups.iter().all(|g| g.is_empty()) {
        return groups;
    }
    let w = groups.iter().find(|g| !g.is_empty()).unwrap()[0].width_cells;
    let h = groups.iter().find(|g| !g.is_empty()).unwrap()[0].height_cells;
    let mut min_x = u16::MAX;
    let mut max_x = 0u16;
    let mut min_y = u16::MAX;
    let mut max_y = 0u16;
    for group in &groups {
        for frame in group {
            for cy in 0..h {
                for cx in 0..w {
                    if !frame.cell_at(cy, cx).transparent {
                        if cx < min_x { min_x = cx; }
                        if cx > max_x { max_x = cx; }
                        if cy < min_y { min_y = cy; }
                        if cy > max_y { max_y = cy; }
                    }
                }
            }
        }
    }
    if min_x > max_x { return groups; }
    let new_w = max_x - min_x + 1;
    let new_h = max_y - min_y + 1;
    groups.into_iter().map(|group| {
        group.into_iter().map(|f| {
            let mut cells = Vec::with_capacity((new_w as usize) * (new_h as usize));
            for cy in min_y..=max_y {
                for cx in min_x..=max_x {
                    cells.push(f.cell_at(cy, cx));
                }
            }
            Frame { width_cells: new_w, height_cells: new_h, cells }
        }).collect()
    }).collect()
}

pub fn frame_from_image(img: &image::DynamicImage) -> Frame {
    extract_frame(img, 0, 0, img.width(), img.height())
}

// Build a Frame from raw RGBA8 bytes (4 bytes per pixel, row-major). Lets us
// accept image data from libraries that pin a different `image` crate version.
pub fn frame_from_rgba_bytes(data: &[u8], width: u32, height: u32) -> Frame {
    let cell_w = width as u16;
    let cell_h = (height / 2) as u16;
    let mut cells = Vec::with_capacity((cell_w as usize) * (cell_h as usize));
    let sample = |x: u32, y: u32| -> Rgba<u8> {
        if x >= width || y >= height {
            return Rgba([0, 0, 0, 0]);
        }
        let i = ((y * width + x) * 4) as usize;
        Rgba([data[i], data[i + 1], data[i + 2], data[i + 3]])
    };
    for cy in 0..cell_h {
        for cx in 0..cell_w {
            let px = cx as u32;
            let py = (cy as u32) * 2;
            let top = sample(px, py);
            let bot = sample(px, py + 1);
            cells.push(half_block_for(top, bot));
        }
    }
    Frame {
        width_cells: cell_w,
        height_cells: cell_h,
        cells,
    }
}

// Trim transparent padding around frames. Computes the union bounding box of
// non-transparent cells across ALL frames so each cropped frame stays the same
// size and the pet doesn't grow/shrink as the animation cycles.
pub fn crop_frames_to_union(frames: Vec<Frame>) -> Vec<Frame> {
    if frames.is_empty() {
        return frames;
    }
    let w = frames[0].width_cells;
    let h = frames[0].height_cells;
    let mut min_x = u16::MAX;
    let mut max_x = 0u16;
    let mut min_y = u16::MAX;
    let mut max_y = 0u16;
    for frame in &frames {
        for cy in 0..h {
            for cx in 0..w {
                if !frame.cell_at(cy, cx).transparent {
                    if cx < min_x {
                        min_x = cx;
                    }
                    if cx > max_x {
                        max_x = cx;
                    }
                    if cy < min_y {
                        min_y = cy;
                    }
                    if cy > max_y {
                        max_y = cy;
                    }
                }
            }
        }
    }
    if min_x > max_x {
        return frames;
    }
    let new_w = max_x - min_x + 1;
    let new_h = max_y - min_y + 1;
    frames
        .into_iter()
        .map(|f| {
            let mut cells = Vec::with_capacity((new_w as usize) * (new_h as usize));
            for cy in min_y..=max_y {
                for cx in min_x..=max_x {
                    cells.push(f.cell_at(cy, cx));
                }
            }
            Frame {
                width_cells: new_w,
                height_cells: new_h,
                cells,
            }
        })
        .collect()
}

// Half-block rendering: each terminal cell = 1 pixel wide × 2 pixels tall.
// Use '▀' (upper half block) with fg=top pixel, bg=bottom pixel.
// Compensates for terminal cell's ~2:1 aspect ratio so square sprites appear square.
fn extract_frame(img: &image::DynamicImage, x: u32, y: u32, w: u32, h: u32) -> Frame {
    let cell_w = w as u16;
    let cell_h = (h / 2) as u16;
    let mut cells = Vec::with_capacity((cell_w as usize) * (cell_h as usize));
    for cy in 0..cell_h {
        for cx in 0..cell_w {
            let px = x + cx as u32;
            let py = y + (cy as u32) * 2;
            let top = sample(img, px, py);
            let bot = sample(img, px, py + 1);
            cells.push(half_block_for(top, bot));
        }
    }
    Frame {
        width_cells: cell_w,
        height_cells: cell_h,
        cells,
    }
}

fn sample(img: &image::DynamicImage, x: u32, y: u32) -> Rgba<u8> {
    if x >= img.width() || y >= img.height() {
        return Rgba([0, 0, 0, 0]);
    }
    img.get_pixel(x, y)
}

fn half_block_for(top: Rgba<u8>, bot: Rgba<u8>) -> SpriteCell {
    let on_top = top[3] >= 128;
    let on_bot = bot[3] >= 128;

    if !on_top && !on_bot {
        return SpriteCell {
            ch: ' ',
            fg: None,
            bg: None,
            transparent: true,
        };
    }

    if on_top && !on_bot {
        return SpriteCell {
            ch: '▀',
            fg: Some((top[0], top[1], top[2])),
            bg: None,
            transparent: false,
        };
    }

    if !on_top && on_bot {
        return SpriteCell {
            ch: '▄',
            fg: Some((bot[0], bot[1], bot[2])),
            bg: None,
            transparent: false,
        };
    }

    SpriteCell {
        ch: '▀',
        fg: Some((top[0], top[1], top[2])),
        bg: Some((bot[0], bot[1], bot[2])),
        transparent: false,
    }
}
