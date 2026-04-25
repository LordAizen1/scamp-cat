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
}

impl Screen {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            cols,
            rows,
            cells: vec![Cell::default(); (cols as usize) * (rows as usize)],
            cursor_row: 0,
            cursor_col: 0,
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
        let cols = self.cols as usize;
        let rows = self.rows as usize;
        let n = (n as usize).min(rows);
        if n == rows {
            self.cells.iter_mut().for_each(|c| *c = Cell::default());
            return;
        }
        for r in 0..(rows - n) {
            for c in 0..cols {
                self.cells[r * cols + c] = self.cells[(r + n) * cols + c];
            }
        }
        for r in (rows - n)..rows {
            for c in 0..cols {
                self.cells[r * cols + c] = Cell::default();
            }
        }
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

    fn csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], _ignore: bool, c: char) {
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
