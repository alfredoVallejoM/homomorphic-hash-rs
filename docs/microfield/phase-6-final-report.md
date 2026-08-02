# Informe final consolidado — Fase 6

Fecha de cierre: 2 de agosto de 2026.

## Resultado ejecutivo

La Fase 6 queda cerrada, tras la corrección F6.G7, con una arquitectura única
para las firmas algebraicas legadas y el análisis estructural de grafos. El
código histórico mantenido ya
no conserva una aritmética o recurrencia paralela: delega en `microfield`, en
los contratos de `structural` y en `FastGraphLabeler`.

El producto principal sigue siendo rápido y acotado. La firma local v1 se
conserva como primitiva algebraica componible; el discriminador global v2 es la
ruta recomendada para clasificación general. La canonización exacta no se ha
infiltrado en el hot path y continúa siendo opt-in, presupuestada y fail-closed.

## Entregas acumuladas

| Track | Entrega cerrada |
|---|---|
| F6.0–F6.8 | auditoría legado, `GaloisSignature256` sobre Microfield, leyes algebraicas corregidas, encoders nominales, tracking y wire contracts |
| F6.G0–G2 | `IncidenceGraph`, refinamiento genérico, F251 prioritario, campos generados, composición disjunta y SHA-256 híbrido |
| F6.G3 | `PreparedGraph`, workspaces sin asignación tras reserva, Rayon determinista y evaluación SoA/AVX2 explícita |
| F6.G4 | actualización incremental por radio, agregados diferenciales, componentes exactos y publicación transaccional |
| F6.G5 | diagnóstico de degeneración, evidencia multi-campo identificada, corpus adversarial y oráculos externos |
| F6.G6 | canonización exacta por individualización–refinamiento con presupuesto de nodos y estado retenido |
| F6.G7 | perfil global v2, motivos acotados, canonización por componentes y corpus externo reproducible |

## Contratos finales

La API distingue siete niveles que no deben confundirse:

1. una firma diferente excluye isomorfismo bajo el mismo modelo e identidad;
2. una firma igual solo significa colisión o indistinguibilidad;
3. varios canales iguales producen `Indistinguishable`, no “isomorfo”;
4. un perfil global exacto diferente excluye isomorfismo aunque v1 colisione;
5. motivos completos diferentes añaden discriminación, no una prueba por
   igualdad;
6. una partición rápida discreta define una forma exacta sin búsqueda;
7. con simetría, solo un árbol completo produce `Exact`; cualquier límite
   produce `BudgetExhausted` sin candidato canónico.

`GraphSignatureId` liga campo, encoder, lanes, parámetros y perfil. El bundle
heterogéneo liga el conjunto completo mediante `GraphEvidenceProfileId`. La
normalización exacta conserva tipo/label de vértice, dirección, relación, rol,
bucles, hiperaristas y multiplicidad.

## Límite de degeneración establecido

No existe un tamaño a partir del cual las firmas locales se vuelvan
inyectivas. `C6` y `C3 ⊔ C3` ya son no isomorfos con la misma evidencia local;
la familia persiste hasta cualquier tamaño que admita dos ciclos. Más rondas,
lanes, F251 frente a GF(2²⁵⁶) o SHA-256 no separan descriptores que ya son
exactamente iguales.

El par fuertemente regular Shrikhande/torres 4×4 demuestra el mismo límite con
parámetros `(16,6,2,2)`. F6.G7 convierte ambos casos en escalado útil: el perfil
global separa `C6`/`C3 ⊔ C3`, y el conteo completo de `K4` separa
Shrikhande/torres. Por eso el diagnóstico separa:

- aliasing finito, donde evidencia independiente puede ayudar;
- ambigüedad 1-WL exacta, donde la garantía requiere búsqueda exacta.

La señal `highly_regular` v1 se activa con `V ≥ 4`, al menos 75 % de vértices
ambiguos y una clase exacta de al menos 25 % de `V`. Los contadores brutos se
exponen para políticas de aplicación futuras.

## Evidencia de cierre

- 450 tests unitarios del paquete legado rehabilitado;
- 24 tests del motor rápido/incremental;
- 15 tests adversariales de G5–G7;
- cuatro suites externas opt-in sobre atlas, moléculas, red dirigida e
  hipergrafo;
- 29 tests de firmas estructurales;
- suites completas de Microfield y todos los targets del workspace;
- 1.088 grafos simples exhaustivos de cuatro/cinco vértices, en 11/34 clases;
- 1.253 representantes no isomorfos del Graph Atlas hasta siete vértices, sin
  pares v2 indistinguibles;
- 128 normalizaciones aleatorias y 64 renumeraciones simétricas;
- SageMath 10.7: 35 pares de ciclos y un par fuertemente regular no isomorfos;
- Clippy `-D warnings`, rustdoc `-D warnings`, `cargo fmt --check` y
  `git diff --check` limpios;
- Miri focalizado sobre búsqueda completa y ambos límites presupuestarios;
- benchmarks release separados para análisis rápido, perfil global, v2
  completo, diagnóstico y búsqueda.

Las mediciones y comandos exactos de G5–G6 están en
[phase-6-g5-g6-final-report.md](phase-6-g5-g6-final-report.md). Los cierres
previos están en [phase-6-g3-final-report.md](phase-6-g3-final-report.md) y
[phase-6-g4-final-report.md](phase-6-g4-final-report.md). La corrección final
está en [phase-6-g7-final-report.md](phase-6-g7-final-report.md).

## Límites que permanecen explícitos

- Las firmas no son criptográficas ni resistentes a un adversario.
- SHA-256 diferencia descriptores, no añade información estructural ausente.
- El perfil global y los motivos reducen las colisiones observadas, pero una
  igualdad v2 tampoco prueba isomorfismo.
- La búsqueda exacta es exponencial en el peor caso.
- El presupuesto limita nodos y frontier retenido; no promete una latencia
  universal para cualquier familia.
- No hay canonización exacta incremental.
- Un `BudgetExhausted` no decide isomorfismo.
- La forma exacta actual está ligada al mismo `GraphSignatureId` versionado.

## Preparación para Fase 7

Fase 7 puede desarrollar torres/extensiones, FFT, reconciliación y adapters de
aplicación como tracks separados. No necesita reabrir las leyes de Fase 6 ni
convertir el buscador exacto en el flujo predeterminado. Cualquier nueva
aplicación de grafos debe elegir entre v1 componible, v2 global/adaptativo y un
presupuesto exacto final.
