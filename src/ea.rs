//! Effective addresses: the twelve modes of the 68000 (PRM 2.2).
//!
//! Transcribed from `m68000_python/_ea.py`. The 6-bit EA field is
//! `mode:register`; mode 7 uses the register field to select five more
//! forms. [`EA_KIND`] maps the field to one of the twelve kinds below, and
//! handlers carry the kind, not the field.
//!
//! Timing is Table 8-1 of the UM, spent in the order the microcode spends
//! it: `-(An)` and `(d8,An,Xn)` start with two internal clocks, extension
//! words are taken from the prefetch queue (each one refills it with a
//! program read), and the operand access comes last. The steps that bring
//! the microcode's PC register up to the fetch address (what an address
//! error stacks) are the corpus's, T3.
//!
//! The reference's addressing categories are frozensets of kinds; here they
//! are bit sets over the kind numbers.

use crate::core::{Bus, GroupZero, M68000, MASK};

pub use crate::ea_kinds::*;

/// Call `$call::<KIND>` with the run-time `$kind` as a const generic, so a
/// handler specialized on its addressing kind is monomorphized once per kind
/// and its mode decode folds away. `$kind` is never NONE: the opcode map
/// sends no word with an invalid EA to a handler that specializes.
macro_rules! by_kind {
    ($kind:expr, $call:ident, $self:ident, $($argument:expr),*) => {
        match $kind {
            0 => $self.$call::<0>($($argument),*),
            1 => $self.$call::<1>($($argument),*),
            2 => $self.$call::<2>($($argument),*),
            3 => $self.$call::<3>($($argument),*),
            4 => $self.$call::<4>($($argument),*),
            5 => $self.$call::<5>($($argument),*),
            6 => $self.$call::<6>($($argument),*),
            7 => $self.$call::<7>($($argument),*),
            8 => $self.$call::<8>($($argument),*),
            9 => $self.$call::<9>($($argument),*),
            10 => $self.$call::<10>($($argument),*),
            11 => $self.$call::<11>($($argument),*),
            _ => unreachable!("no addressing kind"),
        }
    };
}
pub(crate) use by_kind;

pub fn sign_extend_8(value: u32) -> i32 {
    i32::from(value as u8 as i8)
}

pub fn sign_extend_16(value: u32) -> i32 {
    i32::from(value as u16 as i16)
}

/// A word sign-extended to 32 bits, as An, MOVEM.W and EXT.L take it (PRM 2.3).
pub fn word_to_long(value: u32) -> u32 {
    sign_extend_16(value) as u32
}

impl<B: Bus> M68000<B> {
    pub(crate) fn step_size(&self, register: usize, size: usize) -> u32 {
        // (A7)+ and -(A7) move by 2 for a byte, keeping SP even (PRM 2.2.4-2.2.5).
        if size == 1 && register == 7 {
            2
        } else {
            size as u32
        }
    }

    /// Take the brief extension word from the queue and apply it to `base`.
    pub(crate) fn index(&mut self, base: u32) -> Result<u32, GroupZero> {
        let extension = self.extension()?;
        Ok(self.indexed(base, extension))
    }

    /// Apply a brief extension word (PRM 2.4) to `base`.
    ///
    /// Bits 15-12 name Xn (D0-D7, A0-A7), bit 11 chooses the sign-extended
    /// low word (0) or the whole register (1), bits 7-0 are the signed
    /// displacement. Bits 10-8, the 68020's scale, are ignored (PRM 2.4).
    /// JMP, JSR, LEA and PEA apply the word still waiting in IRC.
    pub(crate) fn indexed(&self, base: u32, extension: u32) -> u32 {
        let mut index = self.r[(extension >> 12) as usize & 15];
        if extension & 0x0800 == 0 {
            index = sign_extend_16(index) as u32;
        }
        base.wrapping_add(index)
            .wrapping_add(sign_extend_8(extension) as u32)
    }

    /// Compute a memory operand's address, with its extension words and clocks.
    ///
    /// The standard source-operand sequence: the one TST, CMP, ADD <ea>,Dn
    /// and the read half of a read-modify-write use.
    #[inline(always)]
    pub(crate) fn ea_address(
        &mut self,
        kind: u8,
        register: usize,
        size: usize,
    ) -> Result<u32, GroupZero> {
        match kind {
            IND => Ok(self.r[8 + register]),
            POSTINC => {
                let address = self.r[8 + register];
                self.r[8 + register] = address.wrapping_add(self.step_size(register, size));
                Ok(address)
            }
            PREDEC => {
                self.cycles += 2;
                self.commit_pc();
                let address = self.r[8 + register].wrapping_sub(self.step_size(register, size));
                self.r[8 + register] = address;
                Ok(address)
            }
            DISP => {
                let displacement = sign_extend_16(self.extension()?) as u32;
                Ok(self.r[8 + register].wrapping_add(displacement))
            }
            INDEX => {
                self.cycles += 2;
                self.index(self.r[8 + register])
            }
            ABSW => {
                self.commit_pc();
                Ok(word_to_long(self.extension()?))
            }
            ABSL => {
                let high = self.extension()?;
                self.commit_pc();
                Ok((high << 16) | self.extension()?)
            }
            PCDISP => {
                let base = self.pc.wrapping_sub(2); // the address of the extension word itself
                Ok(base.wrapping_add(sign_extend_16(self.extension()?) as u32))
            }
            PCINDEX => {
                self.cycles += 2;
                self.index(self.pc.wrapping_sub(2))
            }
            _ => unreachable!("not a memory kind: {kind}"),
        }
    }

    /// Fetch a source operand of any kind (UM Table 8-1).
    #[inline(always)]
    pub(crate) fn ea_read(
        &mut self,
        kind: u8,
        register: usize,
        size: usize,
    ) -> Result<u32, GroupZero> {
        match kind {
            DN => Ok(self.r[register] & MASK[size]),
            AN => Ok(self.r[8 + register] & MASK[size]),
            IMM => {
                if size == 4 {
                    self.extension_long()
                } else {
                    Ok(self.extension()? & MASK[size])
                }
            }
            _ => Ok(self.ea_fetch(kind, register, size)?.1),
        }
    }

    /// Address and value of a memory operand: the read half of every access.
    ///
    /// PC-relative operands are read in program space (FC 2/6, UM Table 3-2).
    #[inline(always)]
    pub(crate) fn ea_fetch(
        &mut self,
        kind: u8,
        register: usize,
        size: usize,
    ) -> Result<(u32, u32), GroupZero> {
        if size == 4 && kind == POSTINC {
            // A long (An)+ steps An between its two reads (SST ADD.l, MOVE.l
            // and every long (An)+ read; T3): a fault on the first leaves An
            // alone.
            let address = self.r[8 + register];
            let high = self.read_word(address)?;
            self.r[8 + register] = address.wrapping_add(4);
            return Ok((
                address,
                (high << 16) | self.read_word(address.wrapping_add(2))?,
            ));
        }
        if size == 4 && kind == PREDEC {
            // A long -(An) does not bring PC up to date, unlike a word (SST
            // ADD.l, MOVE.l and every long -(An) read; T3).
            self.cycles += 2;
            let address = self.r[8 + register].wrapping_sub(4);
            self.r[8 + register] = address;
            return Ok((address, self.read_long(address)?));
        }
        let address = self.ea_address(kind, register, size)?;
        if kind >= PCDISP {
            return Ok((address, self.read_program_operand(size, address)?));
        }
        Ok((address, self.read(size, address)?))
    }

    /// Write the low byte, word or all of a data register (PRM 1.1).
    pub(crate) fn write_register(&mut self, register: usize, size: usize, value: u32) {
        if size == 4 {
            self.r[register] = value;
        } else {
            let mask = MASK[size];
            self.r[register] = (self.r[register] & !mask) | (value & mask);
        }
    }
}
