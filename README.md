# Homomorphic Hash RS / Microfield

Workspace Rust para campos finitos portables, firmas algebraicas homomórficas
no criptográficas y análisis/canonización exacta presupuestada de grafos.

El repositorio contiene cuatro paquetes Cargo con funciones distintas:

| Paquete | Función | Publicación actual |
|---|---|---|
| `microfield` | núcleo `no_std`, campos, generación, batch e ISA | `publish = false` |
| `homomorphic-hash-rs` | firmas, deltas, reconciliación, grafos y compatibilidad legacy | candidato interno condicionado |
| `microfield-validation-lab` | campañas y artefactos F6.V | privado, no publicable |
| `structural-field-fixture` | campo externo generado para tests | fixture, no publicable |

## Estado

RC.0–RC.6 están implementados, integrados en `main` y fijados por el tag
`internal-rc6-integrated`. La suite local completa, los corpus externos, el
gate exhaustivo de grafos y la matriz remota x86-64/AArch64 están verdes.

El proyecto aún no es una release pública ni una RC interna final. Su
clasificación vigente es **candidato técnico para consumo interno
condicionado**. RC.7–RC.10 están implementados y verdes localmente; el
dictamen local RC.10 es `Conditional` hasta que el mismo commit limpio aporte
todos los gates y evidencia x86-64/AArch64 en CI.

La línea prioritaria de cierre son los hashes homomórficos —expuestos por la
API como firmas algebraicas para dejar claro que **no son criptográficos**— y
su aplicación a bases de datos. El sistema DB base ya existe: filas/schema,
particiones, transacciones versionadas, log/replay y reconciliación acotada. Lo
pendiente es integrar remotamente su endurecimiento, medición, consumidor
persistente y dictamen RC.10; no reimplementar RC.5.

La fotografía auditada, riesgos y orden siguiente están en
[`current-status-and-next.md`](docs/microfield/current-status-and-next.md). El
índice explica qué documentos son normativos y cuáles conservan historia en
[`docs/microfield/README.md`](docs/microfield/README.md).

## Qué está soportado

- campos estáticos binarios y primos con encoding e identidad canónicos;
- factories certificadas para campos externos binarios y primos;
- contextos runtime validados con assurance y límites explícitos;
- batch portable, packed y backends ISA seleccionados antes del bucle;
- firmas aditiva, secuencial y de multiconjunto, tracking exacto y snapshots;
- deltas revisionados, journals, summary tree, filas/DB y reconciliación
  acotada de conjuntos;
- filtros de grafos, comparación/canonización exacta fail-closed y DAG
  canónico.

Las firmas son resúmenes algebraicos no criptográficos. No autentican, no
prueban pertenencia y no autorizan borrados. Una igualdad compacta significa
`Indistinguishable`; solo tracking exacto o una fuente autoritativa conoce los
datos, y solo `Microcanon` completado con mapping verificado establece una
equivalencia exacta de grafos dentro del schema declarado.

La clasificación detallada y comprobada por tests vive en
[`validation/rc/supported-surface-v1.json`](validation/rc/supported-surface-v1.json).

## Features del paquete raíz

| Feature | Superficie |
|---|---|
| `signatures` | firmas y protocolos mantenidos, sin grafos ni legado |
| `dynamic-signatures` | firmas sobre campos runtime validados |
| `graph` | filtros, comparación, canonización y DAG; depende de firmas |
| `legacy` | compatibilidad del prototipo y demos; no recomendada para código nuevo |
| `dynamic-fields` | alias compatible para runtime signatures + graph |

Los defaults activan `signatures` y `graph`; `legacy` queda fuera y solo se
activa de forma explícita. Un consumidor de firmas sin grafos debería elegir:

```toml
[dependencies]
homomorphic-hash-rs = { path = "...", default-features = false, features = ["signatures"] }
```

## Ejemplo mínimo de Microfield

```rust
use microfield::{CanonicalEncoding, Gf2_256HhV1, Invert};

let value = Gf2_256HhV1::from_canonical(&[1; 32])?;
let inverse = value.invert().expect("el valor no es cero");
let mut one = [0; 32];
one[0] = 1;
assert_eq!((value * inverse).to_canonical(), one);
# Ok::<(), microfield::DecodeError>(())
```

La guía completa de campos, engines, generación y runtime está en
[`crates/microfield/README.md`](crates/microfield/README.md).

## Validación recomendada

`workspace.default-members` contiene únicamente `microfield`; por eso el gate
completo debe llevar `--workspace`:

```text
cargo fmt --all -- --check
cargo test --workspace --all-features --all-targets --locked
cargo clippy --workspace --all-features --all-targets --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --locked --no-deps
```

Gates opt-in de release:

```text
python3 tools/fetch_graph_corpus.py
cargo test -p homomorphic-hash-rs --all-features --locked \
  --test external_graph_corpus -- --ignored --nocapture

cargo test -p homomorphic-hash-rs --all-features --release --locked \
  --test graph_canonical \
  microcanon_matches_every_simple_graph_isomorphism_class_at_six_vertices \
  -- --ignored --exact

cargo run --release -p microfield-validation-lab --locked -- semantic \
  --manifest validation/f6/manifest.json \
  --out validation/f6/results/semantic-v1.json
```

Auditorías de kernels y artefactos:

```text
bash crates/microfield/tools/audit_calibration.sh
bash crates/microfield/tools/audit_unsafe_scope.sh
bash crates/microfield/tools/audit_x86_pclmul.sh
bash crates/microfield/tools/audit_x86_vpclmul.sh
bash crates/microfield/tools/audit_x86_prime.sh
bash crates/microfield/tools/audit_aarch64_pmull.sh
```

## Generación de campos

```text
cargo run -p microfield --features generator --bin microfield-gen -- \
  validate crates/microfield/fields/gf2_256_hh_v1.toml

cargo run -p microfield --features generator --bin microfield-gen -- \
  verify-primes --json
```

## Documentación

- [Estado actual y siguiente plan](docs/microfield/current-status-and-next.md)
- [Plan maestro RC](docs/microfield/release-candidate-readiness-plan.md)
- [Contratos técnicos](docs/microfield/contracts.md)
- [Arquitectura](docs/microfield/architecture.md)
- [Auditoría de `unsafe`](docs/microfield/unsafe-audit.md)
- [Índice completo y ciclo de vida documental](docs/microfield/README.md)

`planificacion.md` y los documentos `phase-*` conservan la especificación y la
evidencia de fases ejecutadas. No son el backlog vigente salvo que la
fotografía actual los marque expresamente como activos.
