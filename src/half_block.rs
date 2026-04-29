// Fallback renderer using Unicode upper-half-block (▀) and lower-half-block
// (▄) characters with truecolor fg/bg. Each terminal cell shows two vertical
// pixels of the source. Works in any terminal that supports 24-bit color
// (so cmd.exe / ConHost on Windows 10+, gnome-terminal, alacritty, etc.).

use std::fmt::Write;

#[derive(Clone, Copy)]
pub struct HbCell {
    pub ch: char,
    pub fg: Option<(u8, u8, u8)>,
    pub bg: Option<(u8, u8, u8)>,
    pub transparent: bool,
}

pub struct HbFrame {
    pub width_cells: u16,
    pub height_cells: u16,
    pub cells: Vec<HbCell>,
}

impl HbFrame {
    pub fn cell_at(&self, row: u16, col: u16) -> HbCell {
        let i = (row as usize) * (self.width_cells as usize) + col as usize;
        self.cells[i]
    }
}

pub fn frame_from_rgba_bytes(data: &[u8], width: u32, height: u32) -> HbFrame {
    let cell_w = width as u16;
    let cell_h = (height / 2) as u16;
    let mut cells = Vec::with_capacity((cell_w as usize) * (cell_h as usize));
    let sample = |x: u32, y: u32| -> [u8; 4] {
        if x >= width || y >= height {
            return [0, 0, 0, 0];
        }
        let i = ((y * width + x) * 4) as usize;
        [data[i], data[i + 1], data[i + 2], data[i + 3]]
    };
    for cy in 0..cell_h {
        for cx in 0..cell_w {
            let px = cx as u32;
            let py = (cy as u32) * 2;
            cells.push(half_block_for(sample(px, py), sample(px, py + 1)));
        }
    }
    HbFrame {
        width_cells: cell_w,
        height_cells: cell_h,
        cells,
    }
}

fn half_block_for(top: [u8; 4], bot: [u8; 4]) -> HbCell {
    let on_top = top[3] >= 128;
    let on_bot = bot[3] >= 128;
    if !on_top && !on_bot {
        return HbCell { ch: ' ', fg: None, bg: None, transparent: true };
    }
    if on_top && !on_bot {
        return HbCell { ch: '▀', fg: Some((top[0], top[1], top[2])), bg: None, transparent: false };
    }
    if !on_top && on_bot {
        return HbCell { ch: '▄', fg: Some((bot[0], bot[1], bot[2])), bg: None, transparent: false };
    }
    HbCell {
        ch: '▀',
        fg: Some((top[0], top[1], top[2])),
        bg: Some((bot[0], bot[1], bot[2])),
        transparent: false,
    }
}

// Trim transparent padding around all frames to a single shared bounding box,
// so every frame is the same size (compositor relies on this).
pub fn crop_frames_to_union(groups: Vec<Vec<HbFrame>>) -> Vec<Vec<HbFrame>> {
    if groups.iter().all(|g| g.is_empty()) {
        return groups;
    }
    let first = groups.iter().find(|g| !g.is_empty()).unwrap();
    let w = first[0].width_cells;
    let h = first[0].height_cells;
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
            HbFrame { width_cells: new_w, height_cells: new_h, cells }
        }).collect()
    }).collect()
}

// Append the half-block render of `frame` at terminal cell (row, col) into `buf`.
//
// Optimizations vs naive per-cell emit:
//   - Reset SGR once at entry, then track prev fg/bg and only emit SGR when it
//     changes (most adjacent cells in a sprite share colors).
//   - Only emit a cursor-position escape on the first non-transparent cell of
//     each row, or after a transparent gap / clipped cell — otherwise rely on
//     the cursor advancing naturally as we write each half-block char.
pub fn write_frame(buf: &mut String, frame: &HbFrame, row: u16, col: u16, max_rows: u16, max_cols: u16) {
    buf.push_str("\x1b[0m");
    let mut prev_fg: Option<(u8, u8, u8)> = None;
    let mut prev_bg: Option<(u8, u8, u8)> = None;
    for cy in 0..frame.height_cells {
        // None = cursor position unknown / next cell must reposition.
        let mut next_expected_col: Option<u16> = None;
        for cx in 0..frame.width_cells {
            let cell = frame.cell_at(cy, cx);
            let r = row + cy;
            let c = col + cx;
            if r >= max_rows || c >= max_cols {
                next_expected_col = None;
                continue;
            }
            if cell.transparent {
                next_expected_col = None;
                continue;
            }
            if next_expected_col != Some(c) {
                let _ = write!(buf, "\x1b[{};{}H", r + 1, c + 1);
            }
            if cell.fg != prev_fg {
                match cell.fg {
                    Some((rc, gc, bc)) => { let _ = write!(buf, "\x1b[38;2;{};{};{}m", rc, gc, bc); }
                    None => buf.push_str("\x1b[39m"),
                }
                prev_fg = cell.fg;
            }
            if cell.bg != prev_bg {
                match cell.bg {
                    Some((br, bg, bb)) => { let _ = write!(buf, "\x1b[48;2;{};{};{}m", br, bg, bb); }
                    None => buf.push_str("\x1b[49m"),
                }
                prev_bg = cell.bg;
            }
            buf.push(cell.ch);
            next_expected_col = Some(c + 1);
        }
    }
}
