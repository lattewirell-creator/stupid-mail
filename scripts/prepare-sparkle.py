"""Fetch the pinned, checksum-verified official Sparkle distribution."""
from pathlib import Path
import hashlib
import subprocess
import shutil

root = Path(__file__).resolve().parents[1]
version = '2.10.0'
checksum = 'c2bf58aa8387266ac179357b1415d6f2635f044da8be41042af32425dae6da0c'
cache = root / '.build'
cache.mkdir(exist_ok=True)
archive = cache / f'Sparkle-{version}.tar.xz'
if not archive.exists():
    partial = archive.with_suffix('.download')
    subprocess.run(['curl', '--fail', '--location', '--proto', '=https', '--tlsv1.2',
                    f'https://github.com/sparkle-project/Sparkle/releases/download/{version}/{archive.name}',
                    '-o', str(partial)], check=True)
    partial.replace(archive)
if hashlib.sha256(archive.read_bytes()).hexdigest() != checksum:
    raise SystemExit('Sparkle checksum mismatch. Remove .build archive and download again.')
destination = cache / 'sparkle'
# Re-extract verified bytes; never silently reuse a modified cached framework.
if destination.exists():
    shutil.rmtree(destination)
destination.mkdir()
subprocess.run(['tar', '-xf', str(archive), '-C', str(destination)], check=True)
print('Verified Sparkle', version)
