// Run wasm-check's exports under Node: the BCD rung, rungs 1 and 5, and a speed figure.
import { readFileSync } from "fs";
const { instance } = await WebAssembly.instantiate(readFileSync(process.argv[2]));
const e = instance.exports;
let ok = true;
const bcd = e.bcd_ok() === 1;
console.log(`BCD tables in wasm: ${bcd ? "all 525,312 inputs agree" : "DIFFER"}`);
ok &&= bcd;
const failures = e.manifest_failures();
console.log(`manifests in wasm: ${e.manifest_count() - failures} of ${e.manifest_count()} match the reference traces`);
if (failures) {
  const packed = e.report();
  const bytes = new Uint8Array(e.memory.buffer, Number(packed >> 32n), Number(packed & 0xffffffffn));
  console.log(new TextDecoder().decode(bytes));
  ok = false;
}
const n = 20_000_000;
e.base_loop(1_000_000);
let best = Infinity;
for (let i = 0; i < 5; i++) {
  const t = performance.now();
  e.base_loop(n);
  best = Math.min(best, performance.now() - t);
}
console.log(`base loop in wasm: ${(n / best / 1000).toFixed(1)} M instr/s (best of 5)`);
process.exit(ok ? 0 : 1);
