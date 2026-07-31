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
import fcntl, os, pty, select, struct, sys, termios, time, signal
argv = sys.argv[1:]
sentinel = os.environ.get("SENTINEL", "")
timeout = float(os.environ.get("PTY_TIMEOUT", "120"))
rows = int(os.environ.get("PTY_ROWS", "40"))
cols = int(os.environ.get("PTY_COLS", "120"))
if sentinel and os.path.exists(sentinel):
    os.unlink(sentinel)
pid, fd = pty.fork()
if pid == 0:
    os.environ.setdefault("TERM", "xterm-256color")
    os.execvp(argv[0], argv)
# pty.fork() leaves the terminal at 0x0; a TTY app that sizes its frame from
# the winsize then initializes a zero-area display and never gets going --
# every run timed out, and before the sentinel rework this was the "script
# is ~50% flaky" mystery. Give the child a real terminal size, like
# script(1) does.
fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
deadline = time.time() + timeout
out = b""
while time.time() < deadline:
    if sentinel and os.path.exists(sentinel):
        break
    # select() with the REMAINING time, so the deadline is actually
    # enforced. A bare blocking os.read() ignores it -- the runner then waits
    # for the child no matter what PTY_TIMEOUT says, which made the timeout
    # decorative and the completion check below unreachable.
    remaining = deadline - time.time()
    if remaining <= 0:
        break
    ready, _, _ = select.select([fd], [], [], min(remaining, 0.25))
    if ready:
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
# A benchmark that dies partway is the dangerous case: it still produces a
# plausible instruction count, just for less work than asked. Treat "did not
# finish" as an ERROR (non-zero exit), never as a quiet result -- a silently
# truncated baseline once looked like a convincing +16% regression that did
# not exist.
ok = bool(sentinel) and os.path.exists(sentinel)
if not ok:
    print("PTY-RUN-INCOMPLETE: sentinel %r never appeared within %gs"
          % (sentinel, timeout), file=sys.stderr)
    sys.exit(2)
print("SENTINEL_WRITTEN")
