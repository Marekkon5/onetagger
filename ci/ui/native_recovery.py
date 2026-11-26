"""Check renderer loss against the packaged Linux entry point on a private display."""
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time


def renderer(parent):
    children = {}
    for line in subprocess.check_output(["ps", "-eo", "pid=,ppid="], text=True).splitlines():
        child, ppid = map(int, line.split())
        children.setdefault(ppid, []).append(child)
    descendants = list(children.get(parent, []))
    while descendants:
        child = descendants.pop()
        try:
            executable = Path(f"/proc/{child}/exe").readlink()
            if executable.name == "WebKitWebProcess":
                return child
        except (PermissionError, FileNotFoundError):
            pass
        descendants.extend(children.get(child, []))
    return None


def check(binary):
    with tempfile.TemporaryDirectory(prefix="onetagger-recovery-") as tmp:
        root = Path(tmp)
        env = dict(os.environ, XDG_DATA_HOME=str(root/"data"),
                   XDG_CONFIG_HOME=str(root/"config"), XDG_CACHE_HOME=str(root/"cache"))
        log = root/"app.log"
        with log.open("w") as output, subprocess.Popen([binary], env=env, stdout=output, stderr=subprocess.STDOUT) as app:
            def wait(condition, description):
                deadline = time.monotonic() + 45
                while time.monotonic() < deadline and app.poll() is None:
                    value = condition()
                    if value:
                        return value
                    time.sleep(.1)
                raise AssertionError(f"Did not observe {description} (desktop exit {app.poll()})")

            def initializations():
                return log.read_text(errors="replace").count("UI session initialized")

            try:
                wait(initializations, "an authenticated desktop client")
                first = wait(lambda: renderer(app.pid), "the app's WebKit page process")
                os.kill(first, signal.SIGKILL)
                wait(lambda: initializations() >= 2, "the reloaded UI using the same local server")
                second = wait(lambda: renderer(app.pid), "a replacement WebKit page process")
                assert second != first
                os.kill(second, signal.SIGKILL)
                wait(lambda: "Native display recovery controls are available" in log.read_text(errors="replace"),
                     "the GTK recovery panel on a repeated failure")
                assert initializations() == 2
                print("Native WebKit recovery passed: same server, reauthenticated page and bounded recovery.")
            finally:
                if app.poll() is None:
                    app.terminate()
                    try:
                        app.wait(timeout=6)
                    except subprocess.TimeoutExpired:
                        app.kill()


if __name__ == "__main__":
    check(Path(sys.argv[1]).resolve())
