# ADR 0026 — separar generación prima y contexto dinámico

## Estado

Aceptado.

## Contexto

La factory binaria v1 ya era una frontera certificada y estable. Ampliarla con
variantes primas o runtime habría creado estados parcialmente válidos y ciclos
entre dominio, filesystem y engine. A la vez, un primo probable no puede
recibir el mismo nivel de confianza que un módulo demostrado.

## Decisión

1. Mantener intacto el manifiesto binario v1.
2. Crear un schema primo v1 independiente y cerrado.
3. Hacer `ValidationAssurance` parte del contexto, no de `FieldId`.
4. Permitir fuente estática solo con `Proven`.
5. Probar `u64` determinísticamente y exigir Pocklington replayable por encima.
6. Generar newtypes nominales con perfil automático por rango; los candidatos
   ISA externos permanecen explícitos.
7. Usar `DynField`/`DynElement` para runtime, con checks escalares nominales y
   `DynBatch` para amortizarlos.
8. Exportar dinámico a estático reejecutando la factory y comparando `FieldId`.
9. Mantener multiprecisión fuera de los features estáticos ordinarios.

## Consecuencias

- no existe JIT, carga de plugins ni código ejecutable procedente de TOML;
- cambiar nombre, prueba o perfil no cambia semántica;
- cambiar representación/perfil sí cambia `ArtifactId`;
- un contexto dinámico es más flexible pero no recibe claims de rendimiento
  estático;
- el bundle es revisable, versionado, reproducible y cacheable;
- se acepta `Box` únicamente en el enum de exportación fuera del hot path; no
  se introduce dispatch virtual en álgebra ni kernels.

## Alternativas rechazadas

- ampliar retrospectivamente el schema binario;
- tratar probable como probado tras muchas rondas;
- factory dinámica de `KernelSet` registrable por consumidores;
- heredar umbrales AVX2/BMI2 de otro campo;
- cargar bibliotecas generadas en runtime;
- usar un `dyn Field` por elemento.
