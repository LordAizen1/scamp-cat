use vte::{Params, Perform};

#[derive(Clone, Copy)]
pub struct Cell {
    pub ch: char,
}

impl Default for Cell {
    fn default() -> Self {
        Self { ch: ' ' }
    }
}

pub struct Screen {
    pub cols: u16,
    pub rows: u16,
    pub cells: Vec<Cell>,
    pub cursor_row: u16,
    pub cursor_col: u16,
    pub scroll_count: u64,
    // True while a full-screen TUI (vim, less, htop) has switched the
    // terminal to its alt-screen buffer via DECSET 1049 / 1047 / 47.
    // Sprite rendering pauses while this is set so we don't draw the cat
    // over the TUI's UI.
    pub alt_screen: bool,
}

impl Screen {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            cols,
            rows,
            cells: vec![Cell::default(); (cols as usize) * (rows as usize)],
            cursor_row: 0,
            cursor_col: 0,
            scroll_count: 0,
            alt_screen: false,
        }
    }

    pub fn cell_at(&self, row: u16, col: u16) -> Cell {
        if row >= self.rows || col >= self.cols {
            return Cell::default();
        }
        self.cells[(row as usize) * (self.cols as usize) + col as usize]
    }

    fn idx(&self, row: u16, col: u16) -> usize {
        (row as usize) * (self.cols as usize) + col as usize
    }

    fn advance_row(&mut self) {
        if self.cursor_row + 1 >= self.rows {
            self.scroll_up(1);
        } else {
            self.cursor_row += 1;
        }
    }

    fn scroll_up(&mut self, n: u16) {
        self.scroll_count = self.scroll_count.wrapping_add(n as u64);
        let cols = self.cols as usize;
        let rows = self.rows as usize;
        let n = (n as usize).min(rows);
        if n == rows {
            self.cells.fill(Cell::default());
            return;
        }
        let shift = n * cols;
        self.cells.copy_within(shift.., 0);
        let tail_start = (rows - n) * cols;
        self.cells[tail_start..].fill(Cell::default());
    }

    fn put_char(&mut self, ch: char) {
        if self.cursor_col >= self.cols {
            self.cursor_col = 0;
            self.advance_row();
        }
        let i = self.idx(self.cursor_row, self.cursor_col);
        if i < self.cells.len() {
            self.cells[i] = Cell { ch };
        }
        self.cursor_col += 1;
    }
}

fn first_param(params: &Params, default: u16) -> u16 {
    params
        .iter()
        .next()
        .and_then(|p| p.first().copied())
        .unwrap_or(default)
}

impl Perform for Screen {
    fn print(&mut self, ch: char) {
        self.put_char(ch);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => self.advance_row(),
            b'\r' => self.cursor_col = 0,
            0x08 => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                }
            }
            b'\t' => {
                let next = ((self.cursor_col / 8) + 1) * 8;
                self.cursor_col = next.min(self.cols.saturating_sub(1));
            }
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], _ignore: bool, c: char) {
        // Private mode set/reset (CSI ? <Pn> h/l) — most relevant for us is
        // the alt-screen toggle so we can pause rendering during vim/htop/less.
        if intermediates == b"?" && (c == 'h' || c == 'l') {
            for param in params.iter() {
                if let Some(&n) = param.first() {
                    if matches!(n, 1049 | 1047 | 47) {
                        self.alt_screen = c == 'h';
                    }
                }
            }
            return;
        }
        match c {
            'H' | 'f' => {
                let mut it = params.iter();
                let row = it.next().and_then(|p| p.first().copied()).unwrap_or(1);
                let col = it.next().and_then(|p| p.first().copied()).unwrap_or(1);
                self.cursor_row = row.saturating_sub(1).min(self.rows.saturating_sub(1));
                self.cursor_col = col.saturating_sub(1).min(self.cols.saturating_sub(1));
            }
            'A' => {
                let n = first_param(params, 1).max(1);
                self.cursor_row = self.cursor_row.saturating_sub(n);
            }
            'B' => {
                let n = first_param(params, 1).max(1);
                self.cursor_row = (self.cursor_row + n).min(self.rows.saturating_sub(1));
            }
            'C' => {
                let n = first_param(params, 1).max(1);
                self.cursor_col = (self.cursor_col + n).min(self.cols.saturating_sub(1));
            }
            'D' => {
                let n = first_param(params, 1).max(1);
                self.cursor_col = self.cursor_col.saturating_sub(n);
            }
            'J' => {
                let mode = first_param(params, 0);
                let cur = self.idx(self.cursor_row, self.cursor_col);
                let total = self.cells.len();
                match mode {
                    0 => {
                        let from = cur.min(total);
                        for c in &mut self.cells[from..] {
                            *c = Cell::default();
                        }
                    }
                    1 => {
                        let end = (cur + 1).min(total);
                        for c in &mut self.cells[..end] {
                            *c = Cell::default();
                        }
                    }
                    2 | 3 => {
                        for c in &mut self.cells {
                            *c = Cell::default();
                        }
                    }
                    _ => {}
                }
            }
            'K' => {
                let mode = first_param(params, 0);
                let row_start = self.idx(self.cursor_row, 0);
                let line_end = row_start + self.cols as usize;
                let cur = self.idx(self.cursor_row, self.cursor_col);
                match mode {
                    0 => {
                        for c in &mut self.cells[cur..line_end] {
                            *c = Cell::default();
                        }
                    }
                    1 => {
                        let to = (cur + 1).min(line_end);
                        for c in &mut self.cells[row_start..to] {
                            *c = Cell::default();
                        }
                    }
                    2 => {
                        for c in &mut self.cells[row_start..line_end] {
                            *c = Cell::default();
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn hook(&mut self, _: &Params, _: &[u8], _: bool, _: char) {}
    fn put(&mut self, _: u8) {}
    fn unhook(&mut self) {}
    fn osc_dispatch(&mut self, _: &[&[u8]], _: bool) {}
    fn esc_dispatch(&mut self, _: &[u8], _: bool, _: u8) {}
}

pub fn dump_to_string(screen: &Screen) -> String {
    let mut s = String::with_capacity((screen.cols as usize + 1) * screen.rows as usize);
    for r in 0..screen.rows {
        for c in 0..screen.cols {
            s.push(screen.cell_at(r, c).ch);
        }
        s.push('\n');
    }
    s
}
