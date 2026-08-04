"""Regenerate the deterministic Phase 4 prime-field oracle corpus.

Run with SageMath and compare stdout byte-for-byte with
`reference-vectors/prime-fields-v1.json`.
"""

from sage.all import ZZ, gcd, inverse_mod, prime_range, proof
import hashlib
import json
import pathlib
import sys

proof.arithmetic(True)

SEED = b"microfield:fp256-generic-v1:2026-08-02"
EXPECTED_GENERIC = ZZ(
    "71319327679048415160211920703270965766974670828100238494590001805011376932671"
)


def derive_generic_prime():
    """Replay the frozen deterministic search, including its smooth factor."""
    smooth = ZZ(1)
    for prime in prime_range(2, 1000):
        smooth *= prime
        if smooth.nbits() >= 132:
            break
    seed_integer = ZZ(int.from_bytes(hashlib.sha256(SEED).digest(), "big"))
    lower = ((ZZ(1) << 255) + smooth - 1) // smooth
    span = ((ZZ(1) << 256) - 2) // smooth - lower
    cofactor = lower + (seed_integer % span)
    attempt = 0
    while True:
        candidate = cofactor * smooth + 1
        if candidate.nbits() == 256 and candidate.is_prime():
            assert attempt == 18
            assert candidate == EXPECTED_GENERIC
            return candidate
        cofactor += 1
        attempt += 1


def little_endian_hex(value, width):
    """Encode one canonical residue with the public Microfield convention."""
    return int(value).to_bytes(width, "little").hex()


def build_cases(modulus, width, name):
    """Produce boundary and deterministic seeded arithmetic cases."""
    pairs = [
        (ZZ(0), ZZ(0)),
        (ZZ(1), modulus - 1),
        (modulus - 1, modulus - 1),
        (ZZ(2), ZZ(3)),
    ]
    state = ZZ(int.from_bytes(hashlib.sha256(SEED + b":" + name.encode()).digest(), "big"))
    for index in range(4):
        state = (state * ZZ(0x9E3779B97F4A7C15) + ZZ(0xA4093822299F31D0) + index) % modulus
        left = state
        state = (state * ZZ(0xBF58476D1CE4E5B9) + ZZ(0x94D049BB133111EB)) % modulus
        pairs.append((left, state))

    cases = []
    for index, (left, right) in enumerate(pairs):
        cases.append(
            {
                "name": "boundary-%d" % index if index < 4 else "seeded-%d" % (index - 4),
                "a": little_endian_hex(left, width),
                "b": little_endian_hex(right, width),
                "sum": little_endian_hex((left + right) % modulus, width),
                "difference": little_endian_hex((left - right) % modulus, width),
                "product": little_endian_hex((left * right) % modulus, width),
                "square": little_endian_hex((left * left) % modulus, width),
                "inverse": None
                if left == 0
                else little_endian_hex(inverse_mod(left, modulus), width),
            }
        )
    return cases


generic = derive_generic_prime()
fields = [
    ("fp251_v1", ZZ(251), 1),
    ("fp_goldilocks64_v1", (ZZ(1) << 64) - (ZZ(1) << 32) + 1, 8),
    ("fp256_generic_v1", generic, 32),
]

document = {
    "schema": int(1),
    "oracle": "SageMath exact integers",
    "seed": SEED.decode(),
    "seed_sha256": hashlib.sha256(SEED).hexdigest(),
    "fields": [
        {
            "name": name,
            "modulus": str(modulus),
            "canonical_bytes": int(width),
            "cases": build_cases(modulus, width, name),
        }
        for name, modulus, width in fields
    ],
}

encoded = json.dumps(document, indent=2, separators=(",", ": ")) + "\n"
if len(sys.argv) == 1:
    print(encoded, end="")
elif len(sys.argv) == 2:
    pathlib.Path(sys.argv[1]).write_text(encoded, encoding="utf-8")
else:
    raise SystemExit("usage: sage generate_prime_vectors.sage [OUTPUT.json]")
