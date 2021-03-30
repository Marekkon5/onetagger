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

    # Download ffmpeg
    print("Setting up ffmpeg and pkg-config")
    url = 'https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-full-shared.7z'
    urllib.request.urlretrieve(url, "dist\\ffmpeg.7z")
    subprocess.Popen(["7z", "x", "dist\\ffmpeg.7z", "-odist\\ffmpeg"]).wait()
    ffmpeg_path = os.path.abspath(os.path.join("dist\\ffmpeg", os.listdir("dist\\ffmpeg")[0]))
    for f in os.listdir(os.path.join(ffmpeg_path, "bin")):
        if f.endswith(".dll"):
            shutil.copy(os.path.join(ffmpeg_path, "bin", f), os.path.join(ffmpeg_path, f))
        else:
            os.remove(os.path.join(ffmpeg_path, "bin", f))
    os.mkdir(os.path.join(ffmpeg_path, "lib", "pkgconfig"))
    with ZipFile("assets\\pkgconfig-win.zip", 'r') as z:
        z.extractall(os.path.join(ffmpeg_path, "lib", "pkgconfig"))

    # Download pkg-config
    url = "https://sourceforge.net/projects/pkgconfiglite/files/0.28-1/pkg-config-lite-0.28-1_bin-win32.zip"
    urllib.request.urlretrieve(url, "dist\\pkg-config.zip")
    with ZipFile("dist\\pkg-config.zip", 'r') as z:
        of = open(os.path.join(ffmpeg_path, "pkg-config.exe"), 'wb')
        of.write(z.read("pkg-config-lite-0.28-1/bin/pkg-config.exe"))
        of.close()

    # Compile Rust
    print("Compiling...")
    subprocess.Popen(["rustup", "override", "set", "nightly"], shell=True).wait()
    env = os.environ.copy()
    env["PATH"] = f"{env['PATH']};{ffmpeg_path}"
    subprocess.Popen(["cargo", "build", "--release"], shell=True, env=env).wait()

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
    # Copy ffmpeg
    for f in os.listdir(os.path.join(ffmpeg_path, "bin")):
        if f.endswith(".dll"):
            shutil.copy(os.path.join(ffmpeg_path, "bin", f), "dist\\unpacked")

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