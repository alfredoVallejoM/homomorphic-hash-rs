# Homomorphic Hash RS / Microfield

Este repositorio contiene dos paquetes con ciclos de vida independientes:

- `homomorphic-hash-rs`, el prototipo legado de hashes y agregación topológica;
- `microfield`, el nuevo núcleo de campos finitos binarios portable.

`microfield` se desarrolla como un paquete único dentro del workspace. Sus
fronteras internas siguen SOLID, dispatch estático en operaciones escalares y
selección previa de estrategia para operaciones por lote.

## Estado

El scaffold y la Fase 0 mínima de `microfield` están implementados. El
generador normaliza manifiestos estrictos, calcula identidades, certifica los
tres polinomios con Rabin, deriva planes y publica artefactos
transaccionalmente. SageMath 10.7 ha producido vectores externos reproducibles
para los tres campos. El vertical H2 se integró en `main` y H3 generaliza la
misma aritmética portable sobre `Gf2_128V1`, `Gf2_256HhV1` y
`Gf2_256AltV1`. Los tres tipos son públicos, nominalmente distintos y comparten
algoritmos monomorfizados. H4 incorpora el motor batch portable y está integrado
en `main`; con ello la Fase 1 está cerrada. En Fase 2, H2.1 incorpora la
factory estática y H2.2 optimiza los campos externos mediante planes portables
deterministas; H2.3 cierra capabilities/selección y H2.4 añade el backend batch
x86-64 PCLMUL para los tres presets. El puente ABI 3 permite que cualquier
campo externo validado reciba perfiles ISA verificados sin abrir catálogos ni
punteros. H2.5 añade PMULL en AArch64 para presets y perfiles externos; queda
en selección explícita hasta disponer de calibración reproducible en hardware
ARM real. H2.6 incorpora `PackingPlan`, `PackedBatch` owned y vistas sobre
storage externo alineado. H2.7 añade VPCLMUL y `AosLanePairs` para presets y
campos ABI 3; queda forzable pero fuera de selección automática tras medir una
regresión en 256 bits. El siguiente hito es H2.8, calibración, auditoría y cierre
de Fase 2.

## Comandos

```text
cargo test -p microfield --features generator --all-targets
cargo test -p microfield --all-features --doc
cargo clippy -p microfield --all-features --all-targets -- -D warnings
cargo check -p microfield --no-default-features --features portable,builtin-fields
cargo check --manifest-path crates/microfield/test-fixtures/external-consumer/Cargo.toml --no-default-features --lib
cargo test --manifest-path crates/microfield/test-fixtures/external-consumer/Cargo.toml --lib
bash crates/microfield/tools/audit_aarch64_pmull.sh
bash crates/microfield/tools/audit_x86_vpclmul.sh
cargo test -p microfield --all-features --test packed_batch --test packed_views
cargo test -p homomorphic-hash-rs --lib
cargo test -p homomorphic-hash-rs --test microfield_compat
```

```text
cargo run -p microfield --features generator --bin microfield-gen -- \
  validate crates/microfield/fields/gf2_256_hh_v1.toml
```

```rust
use microfield::{CanonicalEncoding, Gf2_256HhV1, Invert};

let value = Gf2_256HhV1::from_canonical(&[1; 32])?;
let inverse = value.invert().expect("el valor no es cero");
let mut one = [0; 32];
one[0] = 1;
assert_eq!((value * inverse).to_canonical(), one);
# Ok::<(), microfield::DecodeError>(())
```

La especificación revisada se encuentra en `planificacion.md` y la
documentación mantenida en `docs/microfield/`. El diagnóstico vigente y el
orden del siguiente hito están en
`docs/microfield/current-status-and-next.md`. El resultado completo de la Fase
1 se documenta en `docs/microfield/phase-1-final-report.md`.

La Fase 2 revisada, comenzando por la factory pública de campos binarios, está
en `docs/microfield/phase-2-plan.md`.
