//! The twelve addressing kinds and the categories of PRM Table 2-4.
//!
//! Split out of `ea.rs` (the reference's `_ea.py`) so that `build.rs` can
//! compile the opcode map with them and emit the dispatch table at build
//! time. Nothing here depends on the rest of the crate.

// The twelve kinds, in PRM 2.2 order.
pub const DN: u8 = 0; // Dn                data register direct
pub const AN: u8 = 1; // An                address register direct
pub const IND: u8 = 2; // (An)              address register indirect
pub const POSTINC: u8 = 3; // (An)+        indirect with postincrement
pub const PREDEC: u8 = 4; // -(An)         indirect with predecrement
pub const DISP: u8 = 5; // (d16,An)        indirect with displacement
pub const INDEX: u8 = 6; // (d8,An,Xn)     indirect with index
pub const ABSW: u8 = 7; // (xxx).W        absolute short
pub const ABSL: u8 = 8; // (xxx).L        absolute long
pub const PCDISP: u8 = 9; // (d16,PC)      program counter with displacement
pub const PCINDEX: u8 = 10; // (d8,PC,Xn)  program counter with index
pub const IMM: u8 = 11; // #<data>         immediate
/// Mode 7 registers 5-7: no addressing mode (the reference's `None`).
pub const NONE: u8 = 0xFF;

const fn kind(field: usize) -> u8 {
    let mode = field >> 3;
    let register = field & 7;
    if mode < 7 {
        return mode as u8;
    }
    [ABSW, ABSL, PCDISP, PCINDEX, IMM, NONE, NONE, NONE][register]
}

const fn kinds() -> [u8; 64] {
    let mut table = [0u8; 64];
    let mut field = 0;
    while field < 64 {
        table[field] = kind(field);
        field += 1;
    }
    table
}

/// EA field (bits 5-0) -> kind, or [`NONE`] for mode 7 registers 5-7 (illegal).
pub const EA_KIND: [u8; 64] = kinds();

/// The kind of the EA field in the low six bits of `opcode`.
#[inline]
pub fn ea_kind(opcode: u16) -> u8 {
    EA_KIND[usize::from(opcode & 0x3F)]
}

/// A set of kinds: bit n set means kind n is in the set.
pub type Category = u16;

const fn set(kinds: &[u8]) -> Category {
    let mut bits = 0;
    let mut index = 0;
    while index < kinds.len() {
        bits |= 1 << kinds[index];
        index += 1;
    }
    bits
}

// The addressing categories of PRM Table 2-4, as sets of kinds.
pub const ALL: Category = 0x0FFF;
pub const DATA: Category = ALL & !set(&[AN]);
pub const MEMORY: Category = ALL & !set(&[DN, AN]);
pub const CONTROL: Category = set(&[IND, DISP, INDEX, ABSW, ABSL, PCDISP, PCINDEX]);
pub const ALTERABLE: Category = set(&[DN, AN, IND, POSTINC, PREDEC, DISP, INDEX, ABSW, ABSL]);
pub const DATA_ALTERABLE: Category = DATA & ALTERABLE;
pub const MEMORY_ALTERABLE: Category = MEMORY & ALTERABLE;

/// Whether `kind` (possibly [`NONE`]) is in `category`.
#[inline]
pub fn within(kind: u8, category: Category) -> bool {
    kind != NONE && category & (1 << kind) != 0
}
