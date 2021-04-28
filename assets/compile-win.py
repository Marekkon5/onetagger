import sys
import shutil
import subprocess
import os
import urllib.request
from zipfile import ZipFile

# Requires installed: node, rustup, npm, 7z, nsis, vcpkg
# VCPKG_ROOT has to be set
# choco install nsis 7zip -y
def main():
    vcpkg = os.environ["VCPKG_ROOT"]
    if not os.path.isdir(vcpkg):
        print("VCPKG_ROOT not set or invalid!")
        return

    # Install libsndfile
    print("Installing libsndfile...")
    subprocess.check_output(["vcpkg", "install", "libsndfile:x64-windows-static"])

    # Compile UI
    print("Compiling UI...")
    if not os.path.isfile("client\\dist\\dist.html"):
        subprocess.check_output(["npm", "i"], shell=True, cwd='client')
        subprocess.check_output(["npm", "run", "build"], shell=True, cwd='client')

    # Generate output folders
    if os.path.isdir("dist"):
        shutil.rmtree("dist")
    os.mkdir("dist")

    # Compile Rust
    print("Compiling...")
    subprocess.check_output(["rustup", "install", "nightly"], shell=True)
    subprocess.check_output(["rustup", "override", "set", "nightly"], shell=True)
    env = os.environ.copy()
    # For libsndfile
    env["PATH"] = os.path.join(vcpkg, "installed", "x64-windows-static", "lib") + ';' + env["PATH"]
    env["RUSTFLAGS"] = "-Ctarget-feature=+crt-static"
    subprocess.check_output(["cargo", "build", "--release"], shell=True, env=env)

    # Copy CEF
    print("Copying output files...")
    for i in os.listdir("target\\release\\build"):
        if i.startswith("cef-sys-"):
            if 'out' in os.listdir(os.path.join("target\\release\\build", i)):
                d = os.path.join("target\\release\\build", i, "out")
                for file in os.listdir(d):
                    if file.endswith('.tar.bz2'):
                        os.remove(os.path.join(d, file))
                        continue
                shutil.copytree(d, "dist\\unpacked")
                break
    # Copy bin
    shutil.copy("target\\release\\onetagger.exe", "dist\\unpacked")
    shutil.copy("assets\\icon.ico", "dist\\unpacked")

    # Generate output archive
    print("Generating archive...")
    subprocess.check_output(["7z", "a", "dist\\OneTagger-windows.7z", "-mmt8", "-mx9", "dist\\unpacked"], shell=True)

    # Setup installer
    print("Generating installer...")
    subprocess.check_output(["C:\\Program Files (x86)\\NSIS\\makensis.exe", "assets\\installer.nsi"], shell=True)
    

if __name__ == '__main__':
    if sys.platform != "win32":
        print("Not Windows, exitting...")
        exit(-1)

    main()