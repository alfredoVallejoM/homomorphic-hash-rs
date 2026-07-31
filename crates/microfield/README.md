# Microfield

Núcleo portable para campos finitos binarios con abstracciones de coste cero.

El scaffold y la Fase 0 mínima están implementados. El paquete incluye:

- contratos algebraicos segregados;
- los value objects `FieldId` y `ArtifactId`;
- una implementación completa del campo base `F2`;
- fronteras internas para aritmética binaria, kernels y motor;
- parser TOML estricto, normalización, identidades y Rabin independiente;
- planes deterministas y publicación transaccional;
- CLI, puertos de infraestructura y adaptadores de filesystem/Sage;
- manifiestos normativos certificados para los tres campos de la Fase 1;
- vectores golden v2 generados con SageMath 10.7 y verificados mediante un
  modelo polinómico independiente.

Los tipos GF(2¹²⁸) y GF(2²⁵⁶) no se exportarán hasta disponer de validación de
irreducibilidad, vectores independientes y operaciones completas. Las dos
primeras condiciones ya están satisfechas.

```text
cargo test -p microfield
cargo test -p microfield --features generator --all-targets
cargo clippy -p microfield --all-features --all-targets -- -D warnings
cargo check -p microfield --no-default-features --features portable,builtin-fields
```

Ejemplo:

```text
microfield-gen validate fields/gf2_256_hh_v1.toml
microfield-gen plan fields/gf2_256_hh_v1.toml
microfield-gen all fields/gf2_256_hh_v1.toml --out artifacts
microfield-gen check fields/gf2_256_hh_v1.toml --out artifacts
```
