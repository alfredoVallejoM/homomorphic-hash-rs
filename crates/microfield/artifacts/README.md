# Artefactos generados

Este directorio contiene descriptores normalizados y planes generados por
`microfield-gen`. No se aceptan artefactos escritos manualmente.

Cada conjunto se publicará de forma transaccional y contendrá:

- descriptor canónico;
- `FieldId`, `ArtifactId` y `ArtifactBundleDigest`;
- certificado de validación;
- plan de producto y reducción;
- perfil ISA verificado con schedule y digest propios;
- versión del generador y del IR.

La Fase 0 mínima ya puede producir estos conjuntos con:

```bash
cargo run -p microfield --features generator --bin microfield-gen -- \
  all fields/gf2_256_hh_v1.toml --out artifacts
```

Cada publicación contiene `normalized.toml`, `descriptor.json`,
`certificate.json`, `generation-plan.json`, `metadata.json`,
`verified-isa-profile.json`, `field.rs` y `bundle.json`. El último autentica
rutas, longitudes y SHA-256 de los otros siete payloads. `field.rs` contiene
solo constantes certificadas; los tres tipos aritméticos públicos consumen esas
constantes desde `src/generated` y comparten el núcleo algebraico de
`src/binary`.
