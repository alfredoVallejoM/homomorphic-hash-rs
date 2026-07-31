# ADR 0002 — Identidad de campo

**Estado:** aceptado.

## Decisión

`FieldId` identifica característica, grado, base, módulo y encoding mediante un
JSON canónico de orden fijo. Nombres, perfiles, claims y backends quedan fuera.

`ArtifactId` incorpora `FieldId`, versión del generador, versión del IR,
perfil normalizado y familia target mediante un dominio SHA-256 distinto.

## Consecuencias

- Cambiar de backend no rompe datos persistentes.
- Dos módulos distintos nunca se consideran el mismo campo.
- La serialización de identidad necesita golden tests byte a byte.
- Cambiar el build conserva `FieldId`, pero produce otro `ArtifactId`.
- La integridad byte a byte queda separada en `ArtifactBundleDigest`, según
  ADR 0006.
