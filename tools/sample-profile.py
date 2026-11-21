#!/usr/bin/env python3
"""Sampling profiler for one process, using ptrace instead of perf.

perf_event_open is disabled for unprivileged users on the build machine
(kernel.perf_event_paranoid = 4), so this samples the instruction pointer of
a child process with PTRACE_ATTACH / PTRACE_GETREGS / PTRACE_DETACH at a
fixed interval, then attributes each sample to the outlined function that
contains it (`nm`) and to the inlined frame chain at that address
(`addr2line -i`, which needs the `debug = 1` the release profile sets).

Usage: tools/sample-profile.py [--interval-ms 1] [--seconds 60] [--top 30]
                               -- <command> [args...]

Prints two tables: self time by innermost inlined frame, and self time by
outlined function. Percentages are of the samples taken while the child
was running in the binary's own text (samples that landed in libc or the
kernel-return path are counted separately). Linux x86_64 only.
"""
import argparse
import bisect
import collections
import ctypes
import ctypes.util
import os
import subprocess
import sys
import time

PTRACE_GETREGS = 12
PTRACE_ATTACH = 16
PTRACE_DETACH = 17
RIP_INDEX = 16  # user_regs_struct field order on x86_64


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--interval-ms", type=float, default=1.0)
    parser.add_argument("--seconds", type=float, default=60.0)
    parser.add_argument("--top", type=int, default=30)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command
    if command and command[0] == "--":
        command = command[1:]
    if not command:
        parser.error("no command given")

    libc = ctypes.CDLL(ctypes.util.find_library("c"), use_errno=True)
    libc.ptrace.restype = ctypes.c_long
    libc.ptrace.argtypes = [ctypes.c_long, ctypes.c_long, ctypes.c_void_p, ctypes.c_void_p]
    regs = (ctypes.c_ulonglong * 27)()

    child = subprocess.Popen(command)
    pid = child.pid
    time.sleep(0.2)  # let it get past loading

    exe = os.readlink(f"/proc/{pid}/exe")
    base = None
    with open(f"/proc/{pid}/maps") as maps:
        for line in maps:
            fields = line.split()
            if len(fields) >= 6 and fields[5] == exe and fields[2] == "00000000":
                base = int(fields[0].split("-")[0], 16)
                break
    if base is None:
        sys.exit("could not find the executable's load base")

    samples = collections.Counter()
    outside = 0
    deadline = time.monotonic() + args.seconds
    interval = args.interval_ms / 1000.0
    while time.monotonic() < deadline and child.poll() is None:
        if libc.ptrace(PTRACE_ATTACH, pid, None, None) != 0:
            break
        try:
            os.waitpid(pid, 0)
            if libc.ptrace(PTRACE_GETREGS, pid, None, ctypes.byref(regs)) == 0:
                samples[regs[RIP_INDEX]] += 1
        finally:
            libc.ptrace(PTRACE_DETACH, pid, None, None)
        time.sleep(interval)
    if child.poll() is None:
        child.terminate()
        child.wait()

    # Outlined symbols from nm.
    symbols = []
    for line in subprocess.run(
        ["nm", "-C", "--defined-only", "-n", exe], capture_output=True, text=True, check=True
    ).stdout.splitlines():
        parts = line.split(" ", 2)
        if len(parts) == 3 and parts[1] in "tTwW":
            symbols.append((int(parts[0], 16), parts[2]))
    addresses = [address for address, _ in symbols]

    def outlined(vaddr):
        index = bisect.bisect_right(addresses, vaddr) - 1
        return symbols[index][1] if index >= 0 else "?"

    in_text = {}
    for rip, count in samples.items():
        vaddr = rip - base
        if 0 <= vaddr < addresses[-1] + 0x100000:
            in_text[vaddr] = count
        else:
            outside += count
    total = sum(in_text.values())

    # Inlined frame chains from addr2line, innermost first per address.
    query = "\n".join(f"0x{vaddr:x}" for vaddr in in_text) + "\n"
    out = subprocess.run(
        ["addr2line", "-e", exe, "-f", "-i", "-C", "-a"], input=query, capture_output=True, text=True, check=True
    ).stdout.splitlines()
    chains = {}
    current = None
    lines = iter(out)
    for line in lines:
        if line.startswith("0x"):
            current = int(line, 16)
            chains[current] = []
            continue
        location = next(lines, "")
        chains[current].append(line)

    by_inline = collections.Counter()
    by_outlined = collections.Counter()
    for vaddr, count in in_text.items():
        chain = chains.get(vaddr) or ["??"]
        by_inline[short(chain[0])] += count
        by_outlined[short(outlined(vaddr))] += count

    print(f"{total} samples in {os.path.basename(exe)} text, {outside} elsewhere, "
          f"{len(in_text)} distinct addresses\n")
    for title, counter in (("self by innermost inlined frame", by_inline),
                           ("self by outlined function", by_outlined)):
        print(f"--- {title}")
        for name, count in counter.most_common(args.top):
            print(f"{100.0 * count / total:6.2f}%  {count:7d}  {name}")
        print()


def short(name):
    name = name.split(" (")[0]
    for prefix in ("z80_rust::", "<z80_rust::"):
        if name.startswith(prefix):
            name = name[len(prefix):]
    return name.replace("core::Z80<B>", "Z80").replace("core::Z80<z80_rust::conformance::ConformanceHost>", "Z80")


if __name__ == "__main__":
    main()
