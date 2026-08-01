# Microfield

Núcleo portable para campos finitos binarios con abstracciones de coste cero.

La Fase 1 portable está completa e integrada. El paquete incluye:

- contratos algebraicos segregados;
- los value objects `FieldId` y `ArtifactId`;
- una implementación completa del campo base `F2`;
- fronteras internas para aritmética binaria, kernels y motor;
- parser TOML estricto, normalización, identidades y Rabin independiente;
- planes deterministas y publicación transaccional;
- CLI, puertos de infraestructura y adaptadores de filesystem/Sage;
- manifiestos normativos certificados para los tres campos de la Fase 1;
- vectores golden v2 generados con SageMath 10.7 y verificados mediante un
  modelo polinómico independiente;
- `Gf2_128V1`, `Gf2_256HhV1` y `Gf2_256AltV1`, con encoding, producto
  carry-less, reducción, cuadrado, inversión, potencia, Frobenius, traza y
  norma;
- estrategias estáticas compartidas para 128 y 256 bits, sin dispatch
  dinámico ni asignaciones en el camino escalar;
- `Engine<F>`, `EngineBuilder`, catálogo sellado y operaciones batch portables
  sobre slices, con una validación y una llamada indirecta por lote.

Los tres campos son newtypes públicos distintos. No se exponen limbs,
productos anchos ni conversiones implícitas entre presentaciones, incluso
cuando dos campos tienen el mismo cardinal.

```text
cargo test -p microfield
cargo test -p microfield --features generator --all-targets
cargo test -p microfield --all-features --doc
cargo clippy -p microfield --all-features --all-targets -- -D warnings
cargo check -p microfield --no-default-features --features portable,builtin-fields
cargo bench -p microfield --bench portable_batch
```

Ejemplo:

```text
microfield-gen validate fields/gf2_256_hh_v1.toml
microfield-gen plan fields/gf2_256_hh_v1.toml
microfield-gen all fields/gf2_256_hh_v1.toml --out artifacts
microfield-gen check fields/gf2_256_hh_v1.toml --out artifacts
```
