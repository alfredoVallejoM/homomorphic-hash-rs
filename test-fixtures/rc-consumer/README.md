# RC.9 external consumer fixture

Crate deliberadamente fuera del workspace. Depende de
`homomorphic-hash-rs` con `default-features = false` y solo usa las features
públicas `signatures` y `graph`; por tanto detecta dependencias accidentales en
módulos privados o en `legacy`.

El escenario persiste y reinicia una firma sobre un campo externo generado,
un journal de deltas, un árbol de archivo, filas/log de base de datos y un DAG
canónico. También ejerce reconciliación, fallback medido, corrupción y schema
drift.

```bash
cargo test --manifest-path test-fixtures/rc-consumer/Cargo.toml --all-targets --locked
cargo run --manifest-path test-fixtures/rc-consumer/Cargo.toml --locked -- /tmp/rc9-consumer
```

Las firmas del escenario son algebraicas y no criptográficas.
