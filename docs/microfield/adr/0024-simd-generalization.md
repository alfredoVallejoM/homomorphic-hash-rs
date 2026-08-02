# ADR 0024 — Generalización SIMD por perfil verificado

## Estado

Aceptada el 2 de agosto de 2026.

## Contexto

AVX2 estaba ligado a `Fp251V1`, VPCLMUL procesaba una sola pareja por
iteración y Goldilocks permanecía portable pese a tener una reducción
vectorizable. Generalizar únicamente por anchura sería incorrecto: dos campos
con el mismo tamaño pueden usar representaciones, rangos y reducciones
incompatibles.

Tampoco debe asumirse que una factory independiente del layout reemplaza una
ruta especializada. Extraer y reconstruir cada valor permite mantener la
abstracción segura, pero puede dominar el coste para residuos pequeños.

## Decisión

Se generaliza por contratos estáticos certificados:

- `VerifiedPrimeCanonical8Field` habilita Barrett AVX2 de 32 lanes;
- `VerifiedPrimeCanonical16Field` habilita Barrett AVX2 de 16 lanes;
- Goldilocks mantenido recibe Solinas AVX2 específico de cuatro lanes;
- Montgomery radix 64 continúa bajo el factory BMI2 del ADR 0023;
- VPCLMUL desenrolla dos parejas, sin cambiar su eligibility.

El código generado aporta valores y constantes, nunca intrinsics, punteros de
función o metadata de selección construida libremente. Microfield verifica en
`const fn` módulo, recíproca, representación, reducción, lanes y packing. Las
operaciones se especializan con const generics para retirar el selector de
operación del bucle vectorial.

La selección se separa de la compatibilidad:

- Goldilocks AVX2 entra en `Auto` desde cuatro elementos por evidencia local
  favorable, incluida la suma neutral en la frontera;
- Fp251 conserva su kernel zero-copy y umbral 64;
- todo perfil generado externo es `explicit_only` hasta disponer de una
  calibración versionada del campo concreto;
- VPCLMUL sigue explícito mientras PCLMUL sea mejor en la región conjunta.

## Seguridad

Los entry points seguros solo llegan a funciones `target_feature` después de
la detección realizada por `EngineBuilder`. Las teselas locales están
totalmente inicializadas, las cargas/stores se prueban por longitud y todos los
restos se calculan escalarmente. No se añade asignación, estado global mutable
ni dispatch dentro del bucle.

El hash de las fronteras cambia y exige de nuevo diferencial, ASan, inventario
y auditoría de ensamblado. La metadata `Fixed` describe el schedule del
kernel; no constituye por sí sola una promesa de tiempo constante integral.

## Consecuencias

- futuros primos canónicos de 8/16 bits pueden reutilizar SIMD sin copiar
  intrinsics al artefacto generado;
- los campos mantenidos pueden conservar rutas específicas cuando el layout
  permite zero-copy;
- un `u32`, un canónico `u64` no Goldilocks, IFMA radix 52 o AArch64 necesitan
  perfiles nuevos y sus propias pruebas;
- Fase 5 deberá decidir si materializa representación runtime o packing
  persistente para eliminar conversiones del bridge externo;
- la API escalar y el ABI de `Engine` permanecen estables.

## Alternativas rechazadas

1. **Seleccionar SIMD por tamaño.** No demuestra reducción ni rangos
   compatibles.
2. **Activar automáticamente todo bridge externo.** Carece de evidencia de
   rendimiento asociada al campo y puede degradar severamente.
3. **Reemplazar Fp251 por el bridge genérico.** La medición local mostró una
   regresión cercana a 8x en 64 elementos.
4. **Promover VPCLMUL tras una mejora aislada.** No supera a PCLMUL de forma
   estable en la región publicada.
