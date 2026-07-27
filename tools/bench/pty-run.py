"""Deterministic pty driver for TTY benchmarks and reproducers.

WHY THIS EXISTS: driving a TTY neomacs with `script -qec` is ~50% flaky --
the identical fixture completed on one run and timed out on the next, which
made every TTY timing meaningless. Waiting on a sentinel FILE rather than on
terminal output makes it deterministic (5/5 runs).

  SENTINEL=path PTY_TIMEOUT=secs TERM=xterm-256color \
    python3 tools/bench/pty-run.py ./target/release/neomacs -nw -Q -l fixture.el

Known limit: it drains terminal output in a read loop, so a fixture that
redraws thousands of times is bottlenecked by pty I/O rather than by the
application. Keep iteration counts modest, or the app blocks on write.

Original docstring: run a TTY neomacs, wait for a sentinel FILE
(not terminal output), report status. `script` proved flaky as a driver --
the same fixture completed on one run and timed out on the next -- so this
removes `script` from the equation to see whether it was the cause."""
import os, pty, sys, time, subprocess, signal
argv = sys.argv[1:]
sentinel = os.environ.get("SENTINEL", "")
timeout = float(os.environ.get("PTY_TIMEOUT", "120"))
if sentinel and os.path.exists(sentinel):
    os.unlink(sentinel)
pid, fd = pty.fork()
if pid == 0:
    os.execvp(argv[0], argv)
deadline = time.time() + timeout
out = b""
while time.time() < deadline:
    if sentinel and os.path.exists(sentinel):
        break
    try:
        r = os.read(fd, 65536)
        if not r:
            break
        out += r
    except OSError:
        break
    done, _ = os.waitpid(pid, os.WNOHANG)
    if done:
        break
try:
    os.kill(pid, signal.SIGKILL)
except ProcessLookupError:
    pass
ok = bool(sentinel) and os.path.exists(sentinel)
print("SENTINEL_WRITTEN" if ok else "NO_SENTINEL")
