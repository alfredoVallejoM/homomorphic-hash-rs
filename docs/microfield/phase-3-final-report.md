# Informe final de Fase 3

La Fase 3 está implementada como un corte vertical completo de algoritmos
derivados. La biblioteca dispone ahora de inversión batch tolerante a cero,
scans de producto, Horner en dos orientaciones, `mul_add_into`, potencias fijas,
planes backend-bound, máscara compacta y workspace reutilizable.

## Superficie entregada

- `AlgorithmId`, `OperationKind`, `AlgorithmFamily` y `BatchPlan<F>`;
- `WorkspaceLayout` y `AllocationBehavior`;
- `BitMask`, `BitMaskViewMut` y `required_mask_words`;
- `BatchInvertPlan`, `BatchInvertWorkspace` y variante owned;
- `ProductScanPlan`, `ScanDirection` y `ScanMode`;
- `ManyPointsHornerPlan` y `ManyPolynomialsHornerPlan`;
- `CoefficientLayout::{PolynomialMajor,CoefficientMajor}`;
- `fill_fixed_base_powers` y `FixedBasePowers`;
- facade de conveniencia en `Engine<F>`;
- IR v4 y verificación simbólica de Itoh–Tsujii.

## Garantías

- mismos bytes de resultado para todo backend compatible;
- validación completa antes de escritura;
- slices vacíos con semántica explícita;
- ruta prestada sin asignaciones;
- workspace tipado y naturalmente alineado;
- planes inmutables, reutilizables y ligados a backend/campo;
- cero detección de CPU y cero selección dentro del algoritmo;
- cero `unsafe` nuevo;
- artefactos IR v4 regenerables byte a byte.

## Límites conscientes

- los scans son secuenciales; el plan no crea hilos;
- Horner «muchos polinomios» no transpone ni empaqueta automáticamente;
- `mul_add_into` fusiona el contrato, pero todavía no añade un kernel ISA FMA;
- el cruce de rendimiento publicado es evidencia de una CPU, no una regla
  universal;
- no se promueve PMULL/VPCLMUL automáticamente por resultados de Fase 3.

El detalle trazable está en [phase-3-plan.md](phase-3-plan.md) y la decisión
arquitectónica en
[ADR 0019](adr/0019-derived-algorithms-and-typed-workspaces.md).
