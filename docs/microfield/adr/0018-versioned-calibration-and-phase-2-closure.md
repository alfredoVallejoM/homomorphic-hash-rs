# ADR 0018 — Calibración versionada y cierre conservador de Fase 2

- Estado: aceptado e implementado.
- Fecha: 2 de agosto de 2026.
- Hito: H2.8.

## Contexto

Al finalizar H2.7, PCLMUL, VPCLMUL y PMULL eran correctos y forzables, pero la
evidencia de rendimiento no tenía la misma cobertura. PCLMUL disponía de un
cruce holgado contra portable, VPCLMUL solo ganaba modestamente en GF(2¹²⁸) y
perdía en 256 bits, y PMULL tenía ejecución nativa funcional sin perfil
Criterion normalizado.

Mezclar disponibilidad ISA y rendimiento habría permitido que un cambio de
orden del selector introdujera regresiones sin tocar un kernel.

## Decisión

La selección automática se considera un artefacto versionado. La tabla
`calibration/selection-table-v1.csv` tiene una fila por campo/backend y se
compila como constantes privadas `SelectionCalibration`. No existe parsing,
I/O, heap ni lookup runtime: cada `KernelSet<F>` recibe dos constantes en
compilación, `minimum_batch` y `automatic_selection`.

La tabla v1 decide:

- PCLMUL automático desde un elemento para los tres presets;
- VPCLMUL explícito; 64 es solo candidato local para GF(2¹²⁸) y no existe
  región candidata para los campos de 256 bits;
- PMULL explícito mientras falte diversidad de perfiles nativos.

Promover VPCLMUL exige mejora conservadora mínima del 20 %, pipeline con
packing incluido y dos familias x86-64. Promover PMULL exige la misma mejora y
dos familias AArch64. QEMU no cuenta como evidencia de rendimiento.

Criterion cubre lotes 1, 2, 4, 8, 16, 32, 64, 256, 1024, 4096 y 16384. El
workflow manual `microfield-calibration.yml` captura estimadores y entorno en
dos runners x86-64 y un runner ARM64; sus outputs son candidatos de evidencia,
no modifican automáticamente la tabla.

## Seguridad y estabilidad

El cierre añade tres contratos independientes:

1. Un corpus diferencial versionado con seeds y reproducción reducida al
   primer índice divergente.
2. Un inventario SHA-256 de las cuatro fronteras `unsafe`; cualquier cambio
   exige una nueva revisión explícita.
3. Una matriz runtime/codegen que fija ABI aceptado 1..=3 y ABI emitido 3 desde
   una única constante.

Los audits ISA pasan a rechazar también división y crecimiento estructural
anómalo, además de asignador y dispatch indirecto. Los límites son presupuestos
amplios de instrucciones, no snapshots byte a byte dependientes del compilador.

## Consecuencias

- Fase 2 puede cerrarse sin convertir evidencia incompleta en una promesa de
  rendimiento.
- Una CPU compatible siempre puede usar un backend explícitamente.
- `Auto` sigue siendo estable ante la mera aparición de una ISA más nueva.
- La calibración futura cambia datos/constantes privados, no tipos, encoding,
  `FieldId`, factory ni ABI escalar.
- Cambiar un archivo `unsafe` invalida el inventario aunque el diff parezca no
  funcional; esa fricción es deliberada.

## Alternativas rechazadas

- Benchmark temporal en cada push: ruidoso y dependiente de vecinos del runner.
- Autotuning al arrancar: añade latencia, estado mutable y resultados no
  reproducibles.
- Tabla por modelo de CPU dentro del elemento: penaliza layout y escalar.
- Habilitar PMULL o VPCLMUL por feature bits: disponibilidad no prueba mejora.
- Hash solo de líneas `unsafe`: podría ignorar cambios seguros que alterasen
  sus invariantes circundantes.
