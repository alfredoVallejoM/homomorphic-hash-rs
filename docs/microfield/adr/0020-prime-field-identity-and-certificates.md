# ADR 0020 — Identidad y certificados de campos primos

## Estado

Aceptada el 2 de agosto de 2026.

## Decisión

El descriptor primo usa schema 2 y contiene característica decimal exacta,
grado uno, base prima, módulo y encoding canónico. `FieldId` se calcula sobre
ese JSON minificado con el dominio existente. Nombre, representación,
reducción, perfil ISA y seed de búsqueda quedan fuera.

Los campos mantenidos solo se publican con evidencia demostrativa reproducible:
división completa para 251 y Pocklington para Goldilocks y el primo genérico.
El verificador embebido no consulta Sage ni confía en una etiqueta del
descriptor. Sage actúa como segundo oráculo y generador de corpus.

El primo genérico queda congelado como:

```text
p = 71319327679048415160211920703270965766974670828100238494590001805011376932671
seed = microfield:fp256-generic-v1:2026-08-02
SHA-256(seed) = cf6eae7cff8f204b479357c7b75741c7d422888d8e1649d7a6db0c11ff188599
attempt = 18
```

La búsqueda multiplica un factor suave público hasta 135 bits, deriva el
cofactor inicial del hash y avanza determinísticamente hasta el primer primo de
256 bits. El factor conocido supera (sqrt p), condición necesaria para el
certificado Pocklington almacenado.

## Consecuencias

Cambiar representación o reducción conserva `FieldId` y cambia `ArtifactId`.
Cambiar módulo o encoding produce un tipo semánticamente incompatible. La
factory dinámica futura podrá distinguir `Proven` de `ProbablePrime`, pero
ningún artefacto mantenido puede usar assurance probable.
