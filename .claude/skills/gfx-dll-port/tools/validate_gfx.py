#!/usr/bin/env python3
"""Validates the GLSL and HLSL inside compiled .gfx shaders with glslangValidator.

    validate_gfx.py crates/benilla-gfx/shaders [name ...]

The vk family is SPIR-V already validated by the compiler; this checks the gl3/gl4/gles3 text of
`shaders/` and the d3d11 HLSL of `shaders_d3/` (glslang's HLSL front end, a parse and type check,
not fxc)."""
import os, struct, subprocess, sys, tempfile

SLOTS = {0: "gl3", 1: "gl4", 2: "gles3", 5: "d3d11"}

def blobs(path):
    data = open(path, "rb").read()
    lengths = struct.unpack_from("<8I", data, 12)
    offsets = struct.unpack_from("<8I", data, 12 + 32)
    for slot, name in SLOTS.items():
        if lengths[slot]:
            yield name, data[offsets[slot]:offsets[slot] + lengths[slot]]

def main():
    root = sys.argv[1]
    names = sys.argv[2:]
    failed = 0
    for family in ("shaders", "shaders_d3"):
        d = os.path.join(root, family)
        for f in sorted(os.listdir(d)):
            base, stage = f.split(".")[0], f.split(".")[1]
            if names and base not in names:
                continue
            for backend, code in blobs(os.path.join(d, f)):
                ext = {"vs": "vert", "fs": "frag"}[stage]
                with tempfile.NamedTemporaryFile(suffix="." + ext, delete=False) as t:
                    t.write(code)
                args = ["glslangValidator"]
                if backend == "d3d11":
                    args += ["-D", "-V", "-e", "main", "-S", ext, "-o", os.devnull]
                args.append(t.name)
                r = subprocess.run(args, capture_output=True, text=True)
                os.unlink(t.name)
                ok = r.returncode == 0
                print(f"{'ok  ' if ok else 'FAIL'} {family}/{f} {backend}")
                if not ok:
                    failed = 1
                    print(r.stdout + r.stderr)
    sys.exit(failed)

main()
