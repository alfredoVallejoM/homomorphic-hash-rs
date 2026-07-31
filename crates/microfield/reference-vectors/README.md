# Vectores de referencia

Los vectores mantenidos se produjeron con el oráculo externo SageMath 10.7 y
usan el esquema tipado v2 descrito en ADR 0005.

Cada fichero incluye:

- versión de esquema;
- `FieldId`;
- herramienta y versión del oráculo;
- seed reproducible;
- operandos y resultados;
- casos tipados (`canonical`, `add`, `wide_product`, `reduce`, `multiply`,
  `square`, `invert`, `pow` y `mul_by_x`).

Los elementos tienen ancho canónico fijo. Productos y entradas de reducción
usan exactamente el doble de bytes y declaran semántica polinómica sin
reducción. Los exponentes son bytes little-endian mínimos. La inversa de cero
se representa con resultado `null`.

La implementación portable nunca se utilizará para generar sus propios
vectores de aceptación. La suite contiene además un modelo polinómico lento e
independiente que verifica cada operación de los tres sets.

## Goldens mantenidos

| Campo | SHA-256 del JSON |
|---|---|
| `gf2_128_v1` | `6225b5282e336859307d6f9ded6a7ae82743fa9f0d9a65b866f72c91c0e7b2ad` |
| `gf2_256_alt_v1` | `c3ccc2a5b578ab2f44ef73e679de6223cbe2bcc509ae7c41b5d0f22f4a21d6be` |
| `gf2_256_hh_v1` | `f6a9797159a3be032e897e56d795d6fa2646f74a11ae82d807e542296fb03675` |

El adaptador Sage incluido se ejecuta únicamente durante certificación:

```bash
conda activate laboratorio_np
cargo run -p microfield --features generator --bin microfield-gen -- \
  vectors crates/microfield/fields/gf2_256_hh_v1.toml \
  --sage "$CONDA_PREFIX/bin/sage" \
  --sage-script crates/microfield/tools/sage/generate_vectors.sage \
  --out crates/microfield/reference-vectors/gf2_256_hh_v1.json
```

El mismo comando se ejecuta para `gf2_128_v1`, `gf2_256_hh_v1` y
`gf2_256_alt_v1`, sustituyendo el stem en manifiesto y salida. La suite H3
importa los tres ficheros y ejecuta sus 11 casos contra el tipo público
correspondiente.

Si Sage no puede escribir su caché por las restricciones del entorno, se puede
dirigir fuera del directorio personal:

```bash
DOT_SAGE=/tmp/microfield-sage-cache cargo run -p microfield \
  --features generator --bin microfield-gen -- \
  vectors crates/microfield/fields/gf2_256_hh_v1.toml \
  --sage "$CONDA_PREFIX/bin/sage" \
  --out crates/microfield/reference-vectors/gf2_256_hh_v1.json
```

La regeneración se acepta únicamente si el resultado coincide byte a byte con
el fichero mantenido. También puede importarse y volver a validar un JSON
producido previamente:

```bash
cargo run -p microfield --features generator --bin microfield-gen -- \
  vectors crates/microfield/fields/gf2_256_hh_v1.toml \
  --oracle-json crates/microfield/reference-vectors/gf2_256_hh_v1.json \
  --out /tmp/gf2_256_hh_v1.checked.json
```

El caso de uso exige cobertura normativa completa, limita recursos y rechaza
esquemas desconocidos, claves extra o cualquier `field_id` que no coincida con
el manifiesto certificado.
