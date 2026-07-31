#!/usr/bin/env sage
"""Emit deterministic schema-v2 reference vectors for one Microfield descriptor."""

import argparse
import hashlib
import json

from sage.all import GF, PolynomialRing
from sage.version import version as sage_version


# Sage's preparser turns integer literals into `sage.rings.integer.Integer`.
# The JSON envelope deliberately uses a native Python integer.
SCHEMA = int(2)
GENERATION_ALGORITHM = "sha256-labeled-v1"


def parse_arguments():
    parser = argparse.ArgumentParser()
    parser.add_argument("--payload", required=True)
    return parser.parse_args()


def main():
    request = json.loads(parse_arguments().payload)
    descriptor = request["descriptor"]
    degree = descriptor["degree"]
    width = descriptor["encoding"]["bytes"]

    polynomial_ring = PolynomialRing(GF(2), "x")
    x = polynomial_ring.gen()
    modulus = sum(
        (x ** exponent for exponent in descriptor["modulus"]),
        polynomial_ring.zero(),
    )
    field = GF(2 ** degree, name="a", modulus=modulus)

    def polynomial_to_integer(polynomial):
        return sum(
            int(coefficient) << index
            for index, coefficient in enumerate(polynomial)
        )

    def field_to_integer(value):
        return polynomial_to_integer(value.polynomial())

    def encode_field(value):
        return field_to_integer(value).to_bytes(width, byteorder="little").hex()

    def encode_wide(polynomial):
        return polynomial_to_integer(polynomial).to_bytes(
            width * 2,
            byteorder="little",
        ).hex()

    seed_hex = hashlib.sha256(
        b"microfield:sage-vector-seed:v2\0"
        + bytes.fromhex(request["field_id"])
    ).hexdigest()

    def seeded(label):
        digest = hashlib.sha256(
            bytes.fromhex(seed_hex) + b"\0" + label.encode("ascii")
        ).digest()
        value = int.from_bytes(digest, byteorder="little") & ((1 << degree) - 1)
        coefficients = [(value >> bit) & 1 for bit in range(degree)]
        element = field(polynomial_ring(coefficients))
        return element if element != field.zero() else field.one()

    lhs = seeded("lhs")
    rhs = seeded("rhs")
    lhs_polynomial = lhs.polynomial()
    rhs_polynomial = rhs.polynomial()
    wide_product = lhs_polynomial * rhs_polynomial
    exponent = 65537
    exponent_hex_le = exponent.to_bytes(
        (exponent.bit_length() + 7) // 8,
        byteorder="little",
    ).hex()

    vectors = [
        case("canonical_zero", "canonical", element_hex_le=encode_field(field.zero())),
        case("canonical_one", "canonical", element_hex_le=encode_field(field.one())),
        case(
            "add_seeded",
            "add",
            lhs_hex_le=encode_field(lhs),
            rhs_hex_le=encode_field(rhs),
            output_hex_le=encode_field(lhs + rhs),
        ),
        case(
            "wide_product_seeded",
            "wide_product",
            lhs_hex_le=encode_field(lhs),
            rhs_hex_le=encode_field(rhs),
            output_wide_hex_le=encode_wide(wide_product),
        ),
        case(
            "reduce_seeded_product",
            "reduce",
            input_wide_hex_le=encode_wide(wide_product),
            output_hex_le=encode_field(field(wide_product)),
        ),
        case(
            "multiply_seeded",
            "multiply",
            lhs_hex_le=encode_field(lhs),
            rhs_hex_le=encode_field(rhs),
            output_hex_le=encode_field(lhs * rhs),
        ),
        case(
            "square_seeded",
            "square",
            input_hex_le=encode_field(lhs),
            output_hex_le=encode_field(lhs * lhs),
        ),
        case(
            "invert_seeded",
            "invert",
            input_hex_le=encode_field(lhs),
            output_hex_le=encode_field(lhs ** -1),
        ),
        case(
            "invert_zero",
            "invert",
            input_hex_le=encode_field(field.zero()),
            output_hex_le=None,
        ),
        case(
            "pow_65537",
            "pow",
            base_hex_le=encode_field(rhs),
            exponent_hex_le=exponent_hex_le,
            output_hex_le=encode_field(rhs ** exponent),
        ),
        case(
            "mul_by_x_seeded",
            "mul_by_x",
            input_hex_le=encode_field(lhs),
            output_hex_le=encode_field(lhs * field(x)),
        ),
    ]
    response = {
        "schema": SCHEMA,
        "field_id": request["field_id"],
        "oracle": {
            "name": "SageMath",
            "version": sage_version,
        },
        "generation": {
            "algorithm": GENERATION_ALGORITHM,
            "seed_hex": seed_hex,
        },
        "vectors": vectors,
    }
    print(json.dumps(response, separators=(",", ":")))


def case(case_name, operation_kind, **values):
    return {
        "case": case_name,
        "operation": {
            "kind": operation_kind,
            **values,
        },
    }


if __name__ == "__main__":
    main()
