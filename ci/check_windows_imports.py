"""Check that shipped Windows executables can use the static MSVC runtime."""

import argparse
import os
from pathlib import Path
import re
import subprocess


REDISTRIBUTABLE = re.compile(r"^(?:vcruntime|msvcp|msvcr|concrt|vccorlib|vcomp|vcamp|mfc[m]?)\d", re.IGNORECASE)
DLL_NAME = re.compile(r"^[ \t]+([A-Za-z0-9_.-]+\.dll)[ \t]*$", re.IGNORECASE | re.MULTILINE)


def check_imports(dumpbin: str, binary: Path) -> None:
    output = subprocess.check_output([dumpbin, "/IMPORTS", str(binary)], text=True, errors="replace")
    imports = sorted({name.lower() for name in DLL_NAME.findall(output)})
    if not imports:
        raise SystemExit(f"Could not read PE imports for {binary}")
    print(f"{binary.name} imports: {', '.join(imports)}", flush=True)
    dynamic = [name for name in imports if REDISTRIBUTABLE.match(name)]
    if dynamic:
        raise SystemExit(f"{binary} needs MSVC Redistributable DLLs: {', '.join(dynamic)}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binaries", type=Path, nargs="+")
    args = parser.parse_args()
    vswhere = Path(os.environ["ProgramFiles(x86)"]) / "Microsoft Visual Studio/Installer/vswhere.exe"
    tools = subprocess.check_output([
        str(vswhere), "-latest", "-products", "*", "-requires",
        "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
        "-find", r"VC\Tools\MSVC\**\bin\Hostx64\x64\dumpbin.exe",
    ], text=True).splitlines()
    if not tools:
        raise SystemExit("Visual Studio DUMPBIN is required to verify Windows executables")
    for binary in args.binaries:
        check_imports(tools[0], binary)


if __name__ == "__main__":
    main()
