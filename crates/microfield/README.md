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

H2.1 añade una factory build-time para nuevos campos binarios. El consumidor
describe el grado y el polinomio irreducible, genera un tipo nominal en
`build.rs` y lo usa con los mismos traits y `Engine` portable. No se crea un
contexto de campo runtime ni se limita la factory a los tres presets.

H2.2 añade un optimizador portable estático al mismo pipeline. Cada campo
generado recibe producto carry-less, cuadrado dedicado, inversión Itoh–Tsujii
y una reducción seleccionada por alineamiento y forma del módulo. El plan se
registra en el IR/`ArtifactId`; `FieldId`, layout, encoding y API permanecen
estables. La fuente actual usa ABI de codegen 2 y el runtime acepta 1..=2.

H2.3 completa la frontera previa a ISA. `CpuCapabilities::detect()` toma una
instantánea real con `std`; `portable_only()` conserva selección explícita y
determinista en `no_std`. `KernelCatalog` posee slots internos opcionales y
`EngineBuilder` valida compilación, campo, CPU y política antes de crear un
motor inmutable. Ninguna operación vuelve a detectar o seleccionar, y en H2.3
solo portable está marcado como compilado.

```rust
use microfield::{Engine, ExecutionPolicy, Gf2_256HhV1};

let engine = Engine::<Gf2_256HhV1>::builder()
    .policy(ExecutionPolicy::Throughput)
    .expected_batch(4096)
    .detect()?;
assert_eq!(engine.backend_id(), microfield::BackendId::Portable);
# Ok::<(), microfield::EngineBuildError>(())
```

```rust
let package = microfield::generator::BinaryFieldFactory::builder()
    .name("gf2_233_custom")
    .degree(233)
    .modulus_exponents(vec![233, 74, 0])
    .build()?
    .generate()?;
package.emit_rust(std::env::var_os("OUT_DIR").ok_or("OUT_DIR")?)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

La configuración Cargo completa y los contratos de actualización están en
[`docs/microfield/binary-field-factory.md`](../../docs/microfield/binary-field-factory.md).

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
cargo bench -p microfield --bench portable_codegen_optimizer
```

Ejemplo:

```text
microfield-gen validate fields/gf2_256_hh_v1.toml
microfield-gen plan fields/gf2_256_hh_v1.toml
microfield-gen all fields/gf2_256_hh_v1.toml --out artifacts
microfield-gen check fields/gf2_256_hh_v1.toml --out artifacts
```
