import os
import subprocess

# ----------------------------
# CONFIG
# ----------------------------
DITHER_METHODS = {
    "ordered_o3x3": ["-ordered-dither", "o3x3"],
    "ordered_h4x4a": ["-ordered-dither", "h4x4a"],

}

# ----------------------------
# SCRIPT
# ----------------------------
def ensure_dir(path):
    if not os.path.exists(path):
        os.makedirs(path)

def run_convert(input_path, output_path, args):
    cmd = ["convert", input_path] + args + [output_path]
    print("Running:", " ".join(cmd))
    subprocess.run(cmd, check=True)

from pathlib import Path

def main():
    input_img = "/media/user/MISC/Documents/Coding/Rust/thermal-printer-rust/assets/fop.png"
    out_dir = "/media/user/MISC/Documents/Coding/Rust/thermal-printer-rust/assets/dithers"

    ensure_dir(out_dir)

    for name, args in DITHER_METHODS.items():
        output_img = os.path.join(out_dir, f"{name}.png")
        try:
            run_convert(input_img, output_img, [
                "-resize", "384x",
                "-colorspace", "Gray",
            ] + args + ["-dither", "FloydSteinberg"])
        except:
            print(f"[ERROR] Failed on {name}")

    print("\nDone! Generated:")
    for name in DITHER_METHODS:
        print("  -", name)


if __name__ == "__main__":
    main()
