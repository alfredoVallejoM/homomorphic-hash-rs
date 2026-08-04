"""Verify one Phase 5 external-prime bundle with exact Sage integers.

Usage: sage verify_external_prime_bundle.sage GENERATED_FIELD_DIRECTORY
"""

import json
import pathlib
import sys

from sage.all import ZZ, inverse_mod, proof
from sage.version import version as sage_version


proof.arithmetic(True)


def decode_little(hexadecimal):
    return ZZ(int.from_bytes(bytes.fromhex(hexadecimal), "little"))


def main():
    if len(sys.argv) != 2:
        raise SystemExit(
            "usage: sage verify_external_prime_bundle.sage GENERATED_FIELD_DIRECTORY"
        )
    root = pathlib.Path(sys.argv[1])
    descriptor = json.loads((root / "descriptor.json").read_text(encoding="utf-8"))
    corpus = json.loads((root / "vectors.json").read_text(encoding="utf-8"))
    modulus = ZZ(descriptor["modulus"])
    width = int(descriptor["encoding"]["bytes"])
    assert modulus.is_prime(proof=True)
    assert corpus["field_id"] == json.loads(
        (root / "bundle.json").read_text(encoding="utf-8")
    )["field_id"]

    for vector in corpus["vectors"]:
        left = decode_little(vector["lhs_le_hex"])
        right = decode_little(vector["rhs_le_hex"])
        assert 0 <= left < modulus
        assert 0 <= right < modulus
        assert len(bytes.fromhex(vector["lhs_le_hex"])) == width
        assert decode_little(vector["add_le_hex"]) == (left + right) % modulus
        assert decode_little(vector["mul_le_hex"]) == (left * right) % modulus
        assert decode_little(vector["square_le_hex"]) == (left * left) % modulus
        if left == 0:
            assert vector["inverse_le_hex"] is None
        else:
            assert decode_little(vector["inverse_le_hex"]) == inverse_mod(left, modulus)

    print(
        json.dumps(
            {
                "ok": True,
                "oracle": "SageMath exact integers",
                "sage_version": sage_version,
                "field_id": corpus["field_id"],
                "vectors": len(corpus["vectors"]),
            },
            separators=(",", ":"),
        )
    )


if __name__ == "__main__":
    main()
