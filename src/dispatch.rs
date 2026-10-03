//! The opcode map: which handler each of the 65,536 first words runs.
//!
//! Transcribed from `m68000_python/_dispatch.py`. [`RULES`] is the 68000
//! instruction set as PRM Section 8 prints it, one line per encoding: a
//! 16-character bit pattern, the handler, and the operand constraint. In a
//! pattern `0` and `1` are fixed bits and letters are fields: `ss` a size
//! (00 byte, 01 word, 10 long; 11 is never that instruction), `eeeeee` an
//! effective address, anything else a field the handler decodes. The
//! constraint names the addressing category of PRM Table 2-4 the EA field
//! must fall in; an EA outside it makes the word something else or illegal
//! (UM 6.3.6).
//!
//! `build.rs` walks all 65,536 words once per build, through the rules in
//! `opcode_map.rs`, and fails the build if a word matches two rules. A word that matches none runs the
//! illegal-instruction exception, or the line 1010 / line 1111 emulator
//! exception when its top four bits say so (UM 6.3.6). Dispatch is then one
//! table index and one `match` per instruction.
//!
//! The reference's table holds bound handler functions, built per class;
//! here the table holds the handler's name as an [`Op`] value (the
//! reference's `NAMES`), and [`M68000::execute`] is the `match` from name to
//! handler.

use crate::core::{Bus, Step, M68000};
pub use crate::opcode_map::{decode, Check, Op, RULES};

/// The handler name of every first word, built by `build.rs` from
/// `opcode_map.rs` (the reference's `NAMES`).
pub static NAMES: [Op; 0x10000] = include!(concat!(env!("OUT_DIR"), "/names.rs"));

/// The handler name of every first word.
pub fn names() -> &'static [Op; 0x10000] {
    &NAMES
}

impl<B: Bus> M68000<B> {
    /// Run the handler named `op` for `opcode`: the reference's `table[opcode](cpu, opcode)`.
    pub(crate) fn execute(&mut self, op: Op, opcode: u16) -> Step {
        match op {
            Op::ori_to_ccr => self.op_ori_to_ccr(opcode),
            Op::ori_to_sr => self.op_ori_to_sr(opcode),
            Op::ori => self.op_ori(opcode),
            Op::andi_to_ccr => self.op_andi_to_ccr(opcode),
            Op::andi_to_sr => self.op_andi_to_sr(opcode),
            Op::andi => self.op_andi(opcode),
            Op::subi => self.op_subi(opcode),
            Op::addi => self.op_addi(opcode),
            Op::eori_to_ccr => self.op_eori_to_ccr(opcode),
            Op::eori_to_sr => self.op_eori_to_sr(opcode),
            Op::eori => self.op_eori(opcode),
            Op::cmpi => self.op_cmpi(opcode),
            Op::btst => self.op_btst(opcode),
            Op::bchg => self.op_bchg(opcode),
            Op::bclr => self.op_bclr(opcode),
            Op::bset => self.op_bset(opcode),
            Op::movep => self.op_movep(opcode),
            Op::r#move => self.op_move(opcode),
            Op::movea => self.op_movea(opcode),
            Op::move_from_sr => self.op_move_from_sr(opcode),
            Op::negx => self.op_negx(opcode),
            Op::clr => self.op_clr(opcode),
            Op::move_to_ccr => self.op_move_to_ccr(opcode),
            Op::neg => self.op_neg(opcode),
            Op::move_to_sr => self.op_move_to_sr(opcode),
            Op::not => self.op_not(opcode),
            Op::nbcd => self.op_nbcd(opcode),
            Op::swap => self.op_swap(opcode),
            Op::pea => self.op_pea(opcode),
            Op::ext => self.op_ext(opcode),
            Op::movem => self.op_movem(opcode),
            Op::illegal => self.op_illegal(opcode),
            Op::tas => self.op_tas(opcode),
            Op::tst => self.op_tst(opcode),
            Op::trap => self.op_trap(opcode),
            Op::link => self.op_link(opcode),
            Op::unlk => self.op_unlk(opcode),
            Op::move_usp => self.op_move_usp(opcode),
            Op::reset => self.op_reset(opcode),
            Op::nop => self.op_nop(opcode),
            Op::stop => self.op_stop(opcode),
            Op::rte => self.op_rte(opcode),
            Op::rts => self.op_rts(opcode),
            Op::trapv => self.op_trapv(opcode),
            Op::rtr => self.op_rtr(opcode),
            Op::jsr => self.op_jsr(opcode),
            Op::jmp => self.op_jmp(opcode),
            Op::chk => self.op_chk(opcode),
            Op::lea => self.op_lea(opcode),
            Op::addq => self.op_addq(opcode),
            Op::subq => self.op_subq(opcode),
            Op::dbcc => self.op_dbcc(opcode),
            Op::scc => self.op_scc(opcode),
            Op::bra => self.op_bra(opcode),
            Op::bsr => self.op_bsr(opcode),
            Op::bcc => self.op_bcc(opcode),
            Op::moveq => self.op_moveq(opcode),
            Op::divu => self.op_divu(opcode),
            Op::divs => self.op_divs(opcode),
            Op::sbcd => self.op_sbcd(opcode),
            Op::or => self.op_or(opcode),
            Op::suba => self.op_suba(opcode),
            Op::subx => self.op_subx(opcode),
            Op::sub => self.op_sub(opcode),
            Op::cmpa => self.op_cmpa(opcode),
            Op::cmpm => self.op_cmpm(opcode),
            Op::cmp => self.op_cmp(opcode),
            Op::eor => self.op_eor(opcode),
            Op::mulu => self.op_mulu(opcode),
            Op::muls => self.op_muls(opcode),
            Op::abcd => self.op_abcd(opcode),
            Op::exg => self.op_exg(opcode),
            Op::and => self.op_and(opcode),
            Op::adda => self.op_adda(opcode),
            Op::addx => self.op_addx(opcode),
            Op::add => self.op_add(opcode),
            Op::asd_memory => self.op_asd_memory(opcode),
            Op::lsd_memory => self.op_lsd_memory(opcode),
            Op::roxd_memory => self.op_roxd_memory(opcode),
            Op::rod_memory => self.op_rod_memory(opcode),
            Op::asd => self.op_asd(opcode),
            Op::lsd => self.op_lsd(opcode),
            Op::roxd => self.op_roxd(opcode),
            Op::rod => self.op_rod(opcode),
            Op::line_a => self.op_line_a(opcode),
            Op::line_f => self.op_line_f(opcode),
        }
    }
}
