import sys
import shutil
import subprocess
import os
import urllib.request
from zipfile import ZipFile

# Requires installed: node, rustup, npm, 7z, nsis
# choco install nsis 7zip -y
def main():
    # Compile UI
    print("Compiling UI...")
    if not os.path.isfile("client\\dist\\dist.html"):
        subprocess.Popen(["npm", "i"], shell=True, cwd='client').wait()
        subprocess.Popen(["npm", "run", "build"], shell=True, cwd='client').wait()

    # Generate output folders
    if os.path.isdir("dist"):
        shutil.rmtree("dist")
    os.mkdir("dist")

    # Compile Rust
    print("Compiling...")
    # NOTE: Latest nightly breaks
    subprocess.Popen(["rustup", "install", "nightly"], shell=True).wait()
    subprocess.Popen(["rustup", "override", "set", "nightly"], shell=True).wait()
    subprocess.Popen(["cargo", "build", "--release"], shell=True).wait()

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
    subprocess.Popen(["7z", "a", "dist\\OneTagger-windows.7z", "-mmt8", "-mx9", "dist\\unpacked"], shell=True).wait()

    # Setup installer
    print("Generating installer...")
    subprocess.Popen(["C:\\Program Files (x86)\\NSIS\\makensis.exe", "assets\\installer.nsi"], shell=True).wait()
    

if __name__ == '__main__':
    if sys.platform != "win32":
        print("Not Windows, exitting...")
        exit(-1)

    main()