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
para los tres campos. La aritmética GF(2^n) y los backends ISA se incorporarán
en los siguientes hitos y no se presentan todavía como funcionales.

## Comandos

```text
cargo test -p microfield --features generator --all-targets
cargo clippy -p microfield --all-features --all-targets -- -D warnings
cargo check -p microfield --no-default-features --features portable,builtin-fields
cargo test -p homomorphic-hash-rs --lib
```

```text
cargo run -p microfield --features generator --bin microfield-gen -- \
  validate crates/microfield/fields/gf2_256_hh_v1.toml
```

La especificación revisada se encuentra en `planificacion.md` y la
documentación mantenida en `docs/microfield/`. El diagnóstico vigente y el
orden del siguiente hito están en
`docs/microfield/current-status-and-next.md`.
