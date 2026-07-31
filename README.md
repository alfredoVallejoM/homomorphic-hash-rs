# Homomorphic Hash RS / Microfield

Este repositorio contiene dos paquetes con ciclos de vida independientes:

- `homomorphic-hash-rs`, el prototipo legado de hashes y agregación topológica;
- `microfield`, el nuevo núcleo de campos finitos binarios portable.

`microfield` se desarrolla como un paquete único dentro del workspace. Sus
fronteras internas siguen SOLID, dispatch estático en operaciones escalares y
una futura selección de estrategia por lote.

## Estado

El scaffold y la Fase 0 mínima de `microfield` están implementados. El
generador normaliza manifiestos estrictos, calcula identidades, certifica los
tres polinomios con Rabin, deriva planes y publica artefactos
transaccionalmente. SageMath 10.7 ha producido vectores externos reproducibles
para los tres campos. El primer vertical,
`Gf2_256HhV1`, implementa ya la aritmética portable completa; los otros dos
campos, batch y los backends ISA pertenecen a los siguientes hitos.

## Comandos

```text
cargo test -p microfield --features generator --all-targets
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
`docs/microfield/current-status-and-next.md`.
