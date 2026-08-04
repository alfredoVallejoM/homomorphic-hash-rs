# ADR 0034 — pipeline adaptativo y delta transaccional

Estado: aceptada (3 de agosto de 2026).

## Contexto

Los canales estructurales tenían distinta potencia y coste, pero el consumidor
debía orquestarlos manualmente. La actualización incremental recibía un grafo
completo y auditaba todas sus filas aunque la edición fuese conocida.

## Decisión

`AdaptiveGraphPipeline` ejecuta seis estrategias: metadatos, histogramas
exactos de grado, refinamiento finito, patrones L0–L3, refinamiento localizado
por pares y comparación exacta. Una policy fija techo y presupuestos. Cada tier
registra tiempo, trabajo estimado y skip.

Los tiers no exactos solo pueden publicar `Different`. Su igualdad conduce al
siguiente tier o a `Inconclusive`; únicamente `Microcanon` puede publicar
`Isomorphic`, siempre con `VerifiedGraphMapping`. Los preflights de patterns y
pares son atómicos.

`GraphDelta` modela cambios tipados y revisión esperada. Normaliza y valida la
transacción antes de estimar el cono y elegir replay o rebuild. Los cambios solo
de labels conservan el CSR y recanonizan su diccionario. Los topológicos aún
construyen un CSR candidato inmutable antes del replay localizado.

## Consecuencias

- Hay una fachada desde filtrado ultrarrápido hasta prueba exacta.
- El nivel de garantía se configura sin cambiar la semántica de los canales.
- Las firmas finitas son aceleradores, nunca pruebas de isomorfismo.
- A n=1.024, un label delta midió 191,9 µs frente a 532,8 µs del rebuild
  completo (2,78×) en la máquina de desarrollo.
- Optimizar inserciones topológicas end-to-end exigiría un CSR segmentado; no
  se afirma todavía esa mejora.
