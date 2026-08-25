#!/usr/bin/env python3
"""Run an editor in a pty and wait for it to exit, capturing its output.

Ledger 195.  Companion to scripts/motion-parity-audit.el: gives the editor a
real terminal so `noninteractive' is nil and GNU's DISPLAY-ITERATOR arm of
`Fvertical_motion' (src/indent.c:2287) is the one under test.

  L195_COLS / L195_ROWS   pty size, default 160x50 (the neomacs-tui-tests
                          geometry, neomacs-tui-tests/src/lib.rs:38-39)
  L195_TIMEOUT            seconds before SIGKILL, default 180
"""
import os, pty, sys, select, signal, time, struct, fcntl, termios

def main():
    prog = sys.argv[1]
    args = sys.argv[1:]
    timeout = float(os.environ.get("L195_TIMEOUT", "180"))
    pid, fd = pty.fork()
    if pid == 0:
        os.environ["TERM"] = "screen-256color"
        os.environ["COLUMNS"] = os.environ.get("L195_COLS", "160")
        os.environ["LINES"] = os.environ.get("L195_ROWS", "50")
        os.environ.pop("RUST_LOG", None)
        os.execvp(prog, args)
        os._exit(127)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", int(os.environ.get("L195_ROWS","50")), int(os.environ.get("L195_COLS","160")), 0, 0))
    start = time.time()
    out = bytearray()
    while True:
        if time.time() - start > timeout:
            os.kill(pid, signal.SIGKILL)
            sys.stderr.write("L195 TIMEOUT\n")
            os.waitpid(pid, 0)
            sys.exit(124)
        r, _, _ = select.select([fd], [], [], 1.0)
        if r:
            try:
                data = os.read(fd, 65536)
            except OSError:
                break
            if not data:
                break
            out.extend(data)
        wpid, status = os.waitpid(pid, os.WNOHANG)
        if wpid == pid:
            # drain
            while True:
                r, _, _ = select.select([fd], [], [], 0.2)
                if not r:
                    break
                try:
                    data = os.read(fd, 65536)
                except OSError:
                    break
                if not data:
                    break
                out.extend(data)
            break
    sys.stdout.buffer.write(bytes(out))
    sys.exit(0)

main()
