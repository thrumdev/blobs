
import hashlib
import sys

with open("statefile", 'r') as f:
    statefile = f.read().strip()

with open("wasmfile", 'r') as f:
    wasmfile = f.read().strip()

wasmfile_decode = bytes.fromhex(wasmfile[2:])
blake2_hasher = hashlib.blake2b(digest_size=32)
blake2_hasher.update(wasmfile_decode)
hash = "0x" + blake2_hasher.hexdigest()

with open("kusama.yml", 'r') as f_in, open("kusama_injected.yml", 'w') as f_out:
    for line in f_in:
        modified_line = line.replace("HEAD", statefile)
        modified_line = modified_line.replace("HASH", hash)
        modified_line = modified_line.replace("WASM", wasmfile)
        f_out.write(modified_line)
