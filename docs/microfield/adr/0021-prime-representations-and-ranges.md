# ADR 0021 — Representaciones, reducción y rangos primos

## Estado

Aceptada el 2 de agosto de 2026.

## Decisión

Cada campo mantiene un newtype concreto y normalizado. 251 y Goldilocks
almacenan residuos canónicos; el primo genérico almacena Montgomery radix
(2^{64}). La frontera pública solo intercambia enteros canónicos
little-endian.

`PrimeFieldSpec` y `PrimeWideProduct` son traits internos estáticos. Permiten
reutilizar producto, reducción y diagnósticos sin convertir el elemento en una
representación genérica ni exponer limbs. `Reduced`, `Lazy2` y `Lazy4` son
marcadores privados; únicamente un valor reducido puede cruzar la API.

Los planes Barrett, Montgomery y Solinas son value objects inmutables. Exponen
forma, algoritmo y cotas auditables, pero no referencias públicas a (R),
(R^2), recíprocas o residuos internos. `RangeContract` rechaza acumuladores
incapaces de contener los múltiplos declarados.

## Alternativas descartadas

- Un único `Fp<const LIMBS>` público: filtraría layout y permitiría confundir
  módulos con la misma anchura.
- `dyn PrimeReduction`: introduciría indirección escalar y estado dentro o al
  lado del elemento.
- Decodificación con reducción implícita: destruiría la biyectividad del
  encoding canónico.
- Reutilizar una cadena (p-2) entre módulos: violaría la identidad del plan y
  podría devolver inversas incorrectas.

## Consecuencias

Las abstracciones se monomorfizan y no cambian el layout. Añadir una nueva
representación requiere un tipo mantenido o generado y un plan certificado;
no requiere modificar `Field`, `Engine` ni los algoritmos de Fase 3.
