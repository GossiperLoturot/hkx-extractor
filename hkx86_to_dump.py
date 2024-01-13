import subprocess
import pathlib

def main():
    hkdump_path = pathlib.Path("resources\hkdump.exe")
    in_path = pathlib.Path("resources\hkx86files")
    out_path = pathlib.Path("resources\dumpfiles")

    for file_path in in_path.iterdir():
        if not file_path.name.startswith(".") and file_path.name.endswith(".hkx"):
            subprocess.run([hkdump_path, "-o", out_path / (file_path.stem + ".bin"), file_path])


main()
        