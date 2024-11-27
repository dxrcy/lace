use std::io::{self, IsTerminal, Read, Write as _};

use colored::Colorize as _;
use console::Term;

use crate::RunState;

type TrapFn = fn(&mut RunState) -> ();

pub struct Traps {
    array: [Option<TrapFn>; 0x100],
}

impl Default for Traps {
    fn default() -> Self {
        let mut traps = Self {
            array: [None; 0x100],
        };
        traps.register(0x20, trap_getc);
        traps.register(0x21, trap_out);
        traps.register(0x22, trap_puts);
        traps.register(0x23, trap_in);
        traps.register(0x24, trap_putsp);
        traps.register(0x25, trap_halt);
        traps.register(0x26, trap_putn);
        traps.register(0x27, trap_reg);
        traps
    }
}

impl Traps {
    pub fn register(&mut self, index: u16, func: TrapFn) {
        let entry = &mut self.array[index as usize];
        if entry.is_some() {
            panic!("trap vector 0x{:04x} already registered", index);
        }
        *entry = Some(func);
    }

    pub fn get(&self, index: u16) -> Option<TrapFn> {
        self.array[index as usize]
    }
}

fn trap_getc(state: &mut RunState) {
    *state.reg(0) = read_input() as u16;
}

fn trap_out(state: &mut RunState) {
    let chr = (*state.reg(0) & 0xFF) as u8 as char;
    print!("{chr}");
    io::stdout().flush().unwrap();
}

fn trap_puts(state: &mut RunState) {
    // could probably rewrite with iterators but idk if worth
    for addr in *state.reg(0).. {
        let chr_raw = *state.mem(addr);
        let chr_ascii = (chr_raw & 0xFF) as u8 as char;
        if chr_ascii == '\0' {
            break;
        }
        print!("{}", chr_ascii);
    }
    io::stdout().flush().unwrap();
}

fn trap_in(state: &mut RunState) {
    let ch = read_input();
    *state.reg(0) = ch as u16;
    print!("{}", ch);
    io::stdout().flush().unwrap();
}

fn trap_putsp(state: &mut RunState) {
    'string: for addr in *state.reg(0).. {
        let chr_raw = *state.mem(addr);
        for chr in [chr_raw >> 8, chr_raw & 0xFF] {
            let chr_ascii = chr as u8 as char;
            if chr_ascii == '\0' {
                break 'string;
            }
            print!("{}", chr_ascii);
        }
    }
    io::stdout().flush().unwrap();
}

fn trap_halt(state: &mut RunState) {
    state.pc = u16::MAX;
    println!("\n{:>12}", "Halted".cyan());
}

fn trap_putn(state: &mut RunState) {
    let val = *state.reg(0);
    println!("{val}");
}

fn trap_reg(state: &mut RunState) {
    println!("\n------ Registers ------");
    for (i, reg) in state.reg.iter().enumerate() {
        println!("r{i}: {reg:.>#19}");
        // println!("r{i}: {reg:.>#19b}");
    }
    println!("-----------------------");
}

// Read one byte from stdin or unbuffered terminal
fn read_input() -> u8 {
    let mut stdin = io::stdin();
    if stdin.is_terminal() {
        let cons = Term::stdout();
        let ch = cons.read_char().unwrap();
        ch as u8
    } else {
        let mut buf = [0; 1];
        stdin.read_exact(&mut buf).unwrap();
        buf[0]
    }
}
