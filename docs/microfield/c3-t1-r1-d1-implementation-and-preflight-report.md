# C3 T1/R1/D1: implementación y preflight

Fecha: 2026-08-11. Estado: **implementado y ejecutado como Smoke;
`Controlled` pendiente**.

Las estructuras de este informe emplean firmas y resúmenes algebraicos
**no criptográficos**. Ningún resultado implica autenticación o resistencia
criptográfica a colisiones.

## Cobertura cerrada

El nuevo plan factorial genera 374 celdas: 80 de archivos/summary tree, 84 de
reconciliación acotada y 210 de base de datos en memoria. Con F1–S3, C3 suma
2.984 celdas publicables definidas.

Se incorporaron 14 operaciones y 16 variantes:

- T1: construcción y framing MFFC de chunks, checkpoint MFST, restore
  verificado y ciclos append/truncate;
- R1: sketch, construcción/reconciliación del par, round-trip MFRS y rechazo
  fail-closed por encima del límite;
- D1: round-trip MFRW, mutaciones insert/update/delete, restore y replay MFTL,
  schemas narrow/medium/wide y telemetría de la ruta adaptativa.

## Resultados

Los tres preflights principales ejecutaron 16 celdas, 32 workers y 160
observaciones. Quince cumplieron el umbral Smoke del 25 % y todos conservaron
checksums estables. La mediana de semianchura relativa fue 3,62 %.

La restauración MFST de 1 MiB quedó inicialmente en 35,27 % con dos procesos.
Una calibración independiente de tres procesos y 60 observaciones produjo el
mismo checksum, 25,65 ns/byte de mediana y 0,098 % de semianchura relativa. El
problema era réplica insuficiente, no inestabilidad semántica.

Todas las acciones asignaron memoria porque construyen o restauran estado
persistente. El mayor pico fue 11.161.281 bytes en la ruta adaptativa DB de
64 mutaciones sobre 65.536 filas. Esta ruta clona candidatos transaccionales;
el dato justifica medir memoria frente a particiones y densidad, pero el Smoke
no permite calificarlo todavía como regresión.

Los mayores focos de coste fueron el rechazo de reconciliación por encima del
límite, restore de árboles con chunks pequeños y la aplicación adaptativa de
DB. Checkpoint, framing de filas y schemas estrechos fueron comparativamente
pequeños. Las cifras solo calibran la campaña: `claims_allowed=false`.

## Límites y siguiente bloque

La API mantenida de reconciliación usa Fp251 y admite universos hasta 200; la
matriz ejecutable usa 64, 128 y 160 para respetar también los puntos de
evaluación de diferencias hasta 64. Ampliar el universo requiere otro perfil,
no aumentar tamaños silenciosamente.

El siguiente bloque es G1/G2/X1/X2: pipeline rápido, exactitud acotada, DAG,
corpora externos, wires, packaging y compatibilidad. Después se ejecutará D2
con PostgreSQL real.
