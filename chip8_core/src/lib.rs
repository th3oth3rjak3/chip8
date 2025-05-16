pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;
const RAM_SIZE: usize = 4096;
const NUM_REGS: usize = 16;
const STACK_SIZE: usize = 16;
const NUM_KEYS: usize = 16;
const START_ADDR: u16 = 0x200;
const FONTSET_SIZE: usize = 80;
const FONTSET: [u8; FONTSET_SIZE] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

pub struct Emulator {
    pc: u16,
    ram: [u8; RAM_SIZE],
    screen: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],
    v_reg: [u8; NUM_REGS],
    i_reg: u16,
    stack: [u16; STACK_SIZE],
    sp: u16,
    keys: [bool; NUM_KEYS],
    dt: u8,
    pub st: u8,
    pub draw_completed: bool,
    waiting_for_key_release: Option<usize>,
}

impl Emulator {
    pub fn new() -> Self {
        let mut new_emulator = Emulator {
            pc: START_ADDR,
            ram: [0; RAM_SIZE],
            screen: [false; SCREEN_WIDTH * SCREEN_HEIGHT],
            v_reg: [0; NUM_REGS],
            i_reg: 0,
            sp: 0,
            stack: [0; STACK_SIZE],
            keys: [false; NUM_KEYS],
            dt: 0,
            st: 0,
            draw_completed: true,
            waiting_for_key_release: None,
        };

        new_emulator.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);
        new_emulator
    }

    pub fn is_key_pressed(&self) -> bool {
        self.keys.iter().any(|k| *k)
    }

    pub fn push(&mut self, val: u16) {
        self.stack[self.sp as usize] = val;
        self.sp += 1;
    }

    pub fn pop(&mut self) -> u16 {
        self.sp -= 1;
        self.stack[self.sp as usize]
    }

    pub fn reset(&mut self) {
        self.pc = START_ADDR;
        self.ram = [0; RAM_SIZE];
        self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
        self.v_reg = [0; NUM_REGS];
        self.i_reg = 0;
        self.sp = 0;
        self.stack = [0; STACK_SIZE];
        self.keys = [false; NUM_KEYS];
        self.dt = 0;
        self.st = 0;
        self.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);
    }

    pub fn tick(&mut self) {
        if self.waiting_for_key_release.is_some() {
            return;
        }

        // FETCH
        let op = self.fetch();

        // DECODE & EXECUTE
        self.execute(op);
    }

    pub fn get_display(&self) -> &[bool] {
        &self.screen
    }

    pub fn keypress(&mut self, idx: usize, pressed: bool) {
        self.keys[idx] = pressed;

        if !pressed && Some(idx) == self.waiting_for_key_release {
            self.waiting_for_key_release = None;
        }
    }

    pub fn load_rom(&mut self, data: &[u8]) {
        let start = START_ADDR as usize;
        let end = start + data.len();
        self.ram[start..end].copy_from_slice(data);
    }

    fn fetch(&mut self) -> u16 {
        let higher_byte = self.ram[self.pc as usize] as u16;
        let lower_byte = self.ram[self.pc as usize + 1] as u16;
        let op = (higher_byte << 8) | lower_byte;
        self.pc += 2;
        op
    }

    fn execute(&mut self, op: u16) {
        let digit1 = (op & 0xF000) >> 12;
        let digit2 = (op & 0x0F00) >> 8;
        let digit3 = (op & 0x00F0) >> 4;
        let digit4 = op & 0x000F;

        match (digit1, digit2, digit3, digit4) {
            // NOP - No Operation
            (0, 0, 0, 0) => return,
            // CLS - clear screen
            (0, 0, 0xE, 0) => {
                self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
            }
            // RET - return from subroutine
            (0, 0, 0xE, 0xE) => {
                let ret_addr = self.pop();
                self.pc = ret_addr;
            }
            // JMP NNN
            (1, _, _, _) => {
                let nnn = op & 0x0FFF;
                self.pc = nnn;
            }
            // CALL NNN
            (2, _, _, _) => {
                let nnn = op & 0x0FFF;
                self.push(self.pc);
                self.pc = nnn;
            }
            // SKIP VX == NN
            (3, _, _, _) => {
                let x = digit2 as usize;
                let nn = (op & 0xFF) as u8;
                if self.v_reg[x] == nn {
                    self.pc += 2;
                }
            }
            // SKIP VX != NN
            (4, _, _, _) => {
                let x = digit2 as usize;
                let nn = (op & 0xFF) as u8;
                if self.v_reg[x] != nn {
                    self.pc += 2;
                }
            }
            // SKIP VX == VY
            (5, _, _, 0) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                if self.v_reg[x] == self.v_reg[y] {
                    self.pc += 2;
                }
            }
            // VX = NN
            (6, _, _, _) => {
                let x = digit2 as usize;
                let nn = (op & 0xFF) as u8;
                self.v_reg[x] = nn;
            }
            // VX += NN
            (7, _, _, _) => {
                let x = digit2 as usize;
                let nn = (op & 0xFF) as u8;
                self.v_reg[x] = self.v_reg[x].wrapping_add(nn);
            }
            // VX = VY
            (8, _, _, 0) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v_reg[x] = self.v_reg[y];
            }
            // VX |= VY
            (8, _, _, 1) => {
                self.v_reg[0xF] = 0;
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v_reg[x] |= self.v_reg[y];
            }
            // VX &= VY
            (8, _, _, 2) => {
                self.v_reg[0xF] = 0;
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v_reg[x] &= self.v_reg[y];
            }
            // VX ^= VY
            (8, _, _, 3) => {
                self.v_reg[0xF] = 0;
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v_reg[x] ^= self.v_reg[y];
            }
            // VX += VY (overflowing)
            (8, _, _, 4) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                let (new_vx, carry) = self.v_reg[x].overflowing_add(self.v_reg[y]);
                let new_vf = if carry { 1 } else { 0 };

                self.v_reg[x] = new_vx;
                self.v_reg[0xF] = new_vf;
            }
            // VX -= VY (overflowing)
            (8, _, _, 5) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                let (new_vx, borrow) = self.v_reg[x].overflowing_sub(self.v_reg[y]);
                let new_vf = if borrow { 0 } else { 1 };

                self.v_reg[x] = new_vx;
                self.v_reg[0xF] = new_vf;
            }
            // VX = VY >> 1
            (8, _, _, 6) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                let lsb = self.v_reg[x] & 0x1;
                self.v_reg[x] = self.v_reg[y] >> 1;
                self.v_reg[0xF] = lsb;
            }
            // VY -= VX
            (8, _, _, 7) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                let (new_vx, borrow) = self.v_reg[y].overflowing_sub(self.v_reg[x]);
                let new_vf = if borrow { 0 } else { 1 };

                self.v_reg[x] = new_vx;
                self.v_reg[0xF] = new_vf;
            }
            // VX = VY << 1
            (8, _, _, 0xE) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                let msb = (self.v_reg[y] >> 7) & 0x1;
                self.v_reg[x] = self.v_reg[y] << 1;
                self.v_reg[0xF] = msb;
            }
            // SKIP VX != VY
            (9, _, _, 0) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                if self.v_reg[x] != self.v_reg[y] {
                    self.pc += 2;
                }
            }
            // I = NNN
            (0xA, _, _, _) => {
                let nnn = op & 0x0FFF;
                self.i_reg = nnn;
            }
            // JMP V0 + NNN
            (0xB, _, _, _) => {
                let nnn = op & 0x0FFF;
                self.pc = (self.v_reg[0] as u16 + nnn).into();
            }
            // CXNN - VX = rand() & NN
            (0xC, _, _, _) => {
                let x = digit2 as usize;
                let nn = (op & 0xFF) as u8;
                let rng: u8 = rand::random();
                self.v_reg[x] = rng & nn;
            }
            // DRAW!
            (0xD, _, _, _) => {
                let x_coord = self.v_reg[digit2 as usize] as usize % SCREEN_WIDTH;
                let y_coord = self.v_reg[digit3 as usize] as usize % SCREEN_HEIGHT;
                let num_rows = digit4;

                // keep track of whether any pixels were flipped.
                let mut flipped = false;
                // Iterate over each row in the sprite.
                for y_line in 0..num_rows as usize {
                    // get the memory address where our row's data is stored.
                    let addr = self.i_reg + y_line as u16;
                    let pixels = self.ram[addr as usize];

                    let y = y_coord + y_line;
                    if y >= SCREEN_HEIGHT {
                        continue;
                    }

                    // iterate over each column in the current row
                    for x_line in 0..8 {


                        // this fetches the value of the current bit with a mask.
                        if (pixels & (0b1000_0000 >> x_line)) != 0 {
                            let x = x_coord + x_line;
                            if x >= SCREEN_WIDTH {
                                continue;
                            }
                            let idx = x + (SCREEN_WIDTH * y);
                            flipped |= self.screen[idx];
                            self.screen[idx] ^= true;
                        }
                    }
                }
                self.v_reg[0xF] = if flipped { 1 } else { 0 };
                self.draw_completed = false;
            }
            // SKIP KEY PRESS
            (0xE, _, 9, 0xE) => {
                let x = digit2 as usize;
                let vx = self.v_reg[x];
                let key = self.keys[vx as usize];
                if key {
                    self.pc += 2;
                }
            }
            // SKIP KEY NOT PRESSED
            (0xE, _, 0xA, 1) => {
                let x = digit2 as usize;
                let vx = self.v_reg[x];
                let key = self.keys[vx as usize];
                if !key {
                    self.pc += 2;
                }
            }
            // VX = DT
            (0xF, _, 0, 7) => {
                let x = digit2 as usize;
                self.v_reg[x] = self.dt;
            }
            // WAIT KEY
            (0xF, _, 0, 0xA) => {
                let x = digit2 as usize;
                let mut pressed_key = None;

                for i in 0..self.keys.len() {
                    if self.keys[i] {
                        pressed_key = Some(i);
                        break;
                    }
                }

                if let Some(key_idx) = pressed_key {
                    // Key is pressed, store its value and remember we're waiting for it to be released
                    self.v_reg[x] = key_idx as u8;
                    self.waiting_for_key_release = Some(key_idx);
                } else {
                    // No key pressed, repeat this instruction
                    self.pc -= 2;
                }
            }
            // DT = VX
            (0xF, _, 1, 5) => {
                let x = digit2 as usize;
                self.dt = self.v_reg[x];
            }
            // ST = VX
            (0xF, _, 1, 8) => {
                let x = digit2 as usize;
                self.st = self.v_reg[x];
            }
            // I += VX
            (0xF, _, 1, 0xE) => {
                let x = digit2 as usize;
                let vx = self.v_reg[x] as u16;
                self.i_reg = self.i_reg.wrapping_add(vx);
            }
            // I = FONT
            (0xF, _, 2, 9) => {
                let x = digit2 as usize;
                let c = self.v_reg[x] as u16;
                self.i_reg = c * 5; // 5 bytes per font char. '0' is 0*5 in ram, '2' is at 2*5 (10).
            }
            // BCD
            (0xF, _, 3, 3) => {
                let x = digit2 as usize;
                let vx = self.v_reg[x];
                // fetch the hundreds digit by dividing by 100 and tossing the decimal
                let hundreds = vx / 100;
                // Fetch the tens digit by dividing by 10, tossing the ones digit and the decimal
                let tens = (vx % 100) / 10;
                // Fetch the ones digit by tossing the hundreds and the tens
                let ones = vx % 10;

                self.ram[self.i_reg as usize] = hundreds;
                self.ram[self.i_reg as usize + 1] = tens;
                self.ram[self.i_reg as usize + 2] = ones;
            }
            // FX55 store V0 - VX into I
            (0xF, _, 5, 5) => {
                let x = digit2 as usize;
                let i = self.i_reg as usize;
                for idx in 0..=x {
                    self.ram[i + idx] = self.v_reg[idx];
                }
                self.i_reg += (x + 1) as u16;
            }
            // FX65 load I into V0 - VX
            (0xF, _, 6, 5) => {
                let x = digit2 as usize;
                let i = self.i_reg as usize;
                for idx in 0..=x {
                    self.v_reg[idx] = self.ram[i + idx];
                }
                self.i_reg += (x + 1) as u16;
            }
            (_, _, _, _) => unimplemented!("Unimplemented OpCode: {}", op),
        }
    }

    pub fn tick_timers(&mut self)    {
        if self.dt > 0 {
            self.dt -= 1;
        }
        if self.st > 0 {
            self.st -= 1;
        } else {
        }
    }
}
