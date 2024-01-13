import subprocess
import pathlib

def main():
    hct_path = pathlib.Path("C:\Program Files\Havok\HavokContentTools\hctStandAloneFilterManager.exe")
    hko_path = pathlib.Path("resources\hkx64_to_hkx86.hko")
    in_path = pathlib.Path("resources\hkx64files")
    out_path = pathlib.Path("resources\hkx86files")

    for file_path in in_path.iterdir():
        if not file_path.name.startswith(".") and file_path.name.endswith(".hkx"):
            subprocess.run([hct_path, "-s", hko_path, file_path])
            pathlib.Path("tmp.hkx").replace(out_path / file_path.name)

main()
        