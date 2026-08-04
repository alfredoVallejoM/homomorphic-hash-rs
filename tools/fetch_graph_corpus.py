#!/usr/bin/env python3
"""Fetch and verify the opt-in external graph corpus using only stdlib."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import shutil
import sys
import urllib.request
import zipfile


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_MANIFEST = ROOT / "tests" / "data" / "external" / "manifest.json"
DEFAULT_CACHE = ROOT / ".cache" / "graph-corpus"


def digest(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            value.update(block)
    return value.hexdigest()


def download(source: dict[str, object], raw: Path, offline: bool) -> Path:
    destination = raw / str(source["raw_name"])
    expected = str(source["sha256"])
    if destination.exists() and digest(destination) == expected:
        return destination
    if offline:
        raise RuntimeError(f"cache ausente o invalida para {source['id']}")
    partial = destination.with_suffix(destination.suffix + ".part")
    request = urllib.request.Request(
        str(source["url"]),
        headers={"User-Agent": "microfield-graph-corpus/1"},
    )
    try:
        with urllib.request.urlopen(request, timeout=90) as response, partial.open("wb") as out:
            shutil.copyfileobj(response, out)
        actual = digest(partial)
        if actual != expected:
            raise RuntimeError(
                f"SHA-256 invalido para {source['id']}: {actual}, esperado {expected}"
            )
        os.replace(partial, destination)
    finally:
        partial.unlink(missing_ok=True)
    return destination


def expand(source: dict[str, object], archive: Path, expanded: Path) -> None:
    expected = [str(value) for value in source["expanded"]]
    kind = source["archive"]
    if kind == "gzip":
        if len(expected) != 1:
            raise RuntimeError("un gzip debe declarar exactamente una salida")
        target = expanded / expected[0]
        target.parent.mkdir(parents=True, exist_ok=True)
        with gzip.open(archive, "rb") as source_file, target.open("wb") as target_file:
            shutil.copyfileobj(source_file, target_file)
    elif kind == "zip":
        with zipfile.ZipFile(archive) as bundle:
            available = set(bundle.namelist())
            for name in expected:
                relative = Path(name)
                if relative.is_absolute() or ".." in relative.parts or name not in available:
                    raise RuntimeError(f"entrada ZIP no autorizada: {name}")
                target = expanded / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                with bundle.open(name) as source_file, target.open("wb") as target_file:
                    shutil.copyfileobj(source_file, target_file)
    elif kind == "plain":
        if len(expected) != 1:
            raise RuntimeError("un fichero plano debe declarar exactamente una salida")
        target = expanded / expected[0]
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(archive, target)
    else:
        raise RuntimeError(f"tipo de archivo desconocido: {kind}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--cache", type=Path, default=DEFAULT_CACHE)
    parser.add_argument("--offline", action="store_true")
    args = parser.parse_args()

    manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
    if manifest.get("schema") != 1:
        raise RuntimeError("esquema de corpus no soportado")
    raw = args.cache / "raw"
    expanded = args.cache / "expanded"
    raw.mkdir(parents=True, exist_ok=True)
    expanded.mkdir(parents=True, exist_ok=True)

    results = []
    for source in manifest["sources"]:
        archive = download(source, raw, args.offline)
        expand(source, archive, expanded)
        results.append(
            {
                "id": source["id"],
                "sha256": digest(archive),
                "bytes": archive.stat().st_size,
            }
        )
    print(json.dumps({"ok": True, "cache": str(args.cache), "sources": results}, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, ValueError, zipfile.BadZipFile) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1) from error
