"""Comprueba versiones bloqueadas; no registra URLs, rutas personales ni secretos."""
import argparse
from hashlib import sha256
from importlib import metadata
import json
from pathlib import Path
import platform
import re
import subprocess


def locked_versions(path: Path) -> dict[str, str]:
    result = {}
    for line in path.read_text(encoding='utf-8').splitlines():
        match = re.match(r'^([A-Za-z0-9_.-]+)==([^ ;\\]+)', line)
        if match:
            name = re.sub(r'[-_.]+', '-', match[1]).lower()
            if name in result:
                raise ValueError('Dependencia repetida en el lockfile.')
            result[name] = match[2]
    if not result:
        raise ValueError('Lockfile sin dependencias.')
    return result


def inventory(lock: Path) -> dict:
    locked = locked_versions(lock)
    actual = {name: metadata.version(name) for name in locked}
    if actual != locked:
        differing = sorted(name for name in locked if actual[name] != locked[name])
        raise ValueError('Versiones distintas al lockfile: ' + ', '.join(differing))
    commit = subprocess.check_output(['git','rev-parse','HEAD'], text=True).strip()
    return {'source_commit': commit, 'python': platform.python_version(),
            'platform': platform.system(), 'lockfile': lock.name,
            'lock_sha256': sha256(lock.read_bytes()).hexdigest(),
            'locked_versions_verified': True, 'dependencies': actual}


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--lock', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    proof = inventory(args.lock)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(proof, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
    print(f"Dependencias verificadas: {len(proof['dependencies'])}; lock SHA256 {proof['lock_sha256']}")
