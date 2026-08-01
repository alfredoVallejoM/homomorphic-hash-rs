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
en `main`; con ello la Fase 1 está cerrada. Los backends ISA pertenecen a la
fase posterior.

## Comandos

```text
cargo test -p microfield --features generator --all-targets
cargo test -p microfield --all-features --doc
cargo clippy -p microfield --all-features --all-targets -- -D warnings
cargo check -p microfield --no-default-features --features portable,builtin-fields
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
