# Informe de cierre F6.G4 — incrementalidad exacta

Fecha: 2 de agosto de 2026.

## Resultado

F6.G4 añade actualización incremental al perfil rápido sin introducir una
segunda recurrencia. `IncrementalGraphState<F, K>` retiene el grafo normalizado,
las etiquetas de las rondas `0..R`, el orden de partición, la firma y un índice
de dependencias débiles. `FastGraphLabeler::update_incremental` recibe el nuevo
grafo completo, audita la diferencia exacta y publica el resultado únicamente
si todas las operaciones terminan correctamente.

La salida es exactamente la misma que `FastGraphLabeler::analyze` sobre el
grafo nuevo. «Incremental» se refiere al trabajo algebraico de propagación: el
método seguro sigue leyendo el input completo para impedir que un consumidor
omita una arista o un vértice cambiado.

## Contrato de actualización

El estado acepta cambios de:

- etiqueta o tipo de vértice;
- dirección, endpoint, relación y rol;
- multiplicidad, bucles e incidencias auxiliares;
- inserción o retirada de aristas;
- unión y separación de componentes débiles.

El número de vértices debe permanecer constante y cada `VertexId` debe mantener
su identidad lógica. Añadir, retirar o renumerar vértices requiere construir un
estado nuevo: inferir automáticamente esa correspondencia sería ambiguo y
podría publicar una actualización incorrecta.

Solo se admite `RefinementProfile::Fast`. El calendario adaptativo de `Robust`
puede cambiar su ronda de parada después de una edición y, por tanto, no posee
un radio fijo reutilizable. Se rechaza mediante `NonComposableProfile`.

## Algoritmo

### Estado persistente

Para `V` vértices, `K` lanes y `R` rondas se conservan `(R + 1)·V` etiquetas.
Esta historia permite consultar la capa anterior de cualquier ronda sin
recalcular las capas previas completas. El coste de memoria es
`O(K·R·V + V + I)`.

También se retienen:

- productos no nulos y contadores de factores cero por ronda/lane;
- transcript y metadatos exactos;
- orden final de etiquetas y partición;
- CSR simétrico de dependencias, sin duplicados por relación;
- componente débil de cada vértice y número total de componentes.

### Auditoría fail-closed

Cada actualización compara:

1. la etiqueta inicial codificada de cada vértice;
2. su fila CSR saliente semántica;
3. su fila CSR entrante semántica.

La comparación de filas usa endpoint, descriptor exacto y multiplicidad, no el
`RelationId` interno, que puede cambiar cuando se reordena el pool. Por ello una
inserción de descriptor no invalida falsamente todas las relaciones y tampoco
puede ocultar un cambio real.

### Cono de propagación

En cada ronda se recomputan:

```text
vértices con topología propia cambiada
∪ etiquetas realmente cambiadas en la ronda anterior
∪ dependientes de esas etiquetas
```

Un vértice de topología modificada entra en todas las rondas: aunque una
colisión de campo mantuviese una etiqueta intermedia, su nueva fila CSR sigue
siendo una entrada directa de la recurrencia posterior. Los demás vértices solo
se propagan cuando el valor de campo anterior cambió realmente.

El índice de dependencias se obtiene fusionando las filas entrantes y salientes
ya ordenadas. Se construye en `O(V + I)`, deduplica endpoints con múltiples
relaciones y almacena como máximo `2I` registros. Los bucles no necesitan una
entrada porque el frontier siempre incluye el propio vértice.

### Agregación algebraica diferencial

Las firmas guardan por ronda el producto de factores no nulos y el número de
factores cero. Todas las retiradas no nulas de una ronda/lane se multiplican y
se invierte una sola vez el producto agrupado; los factores nuevos se agrupan
del mismo modo y los ceros ajustan contadores exactos. Así el número de
inversiones queda acotado por `K·(R+1)`, no por el número de etiquetas
cambiadas. El transcript completo se reconstruye después en `O(K·R)`.

Esto realiza composición y descomposición algebraica sin dividir por cero. Las
ediciones que unen o separan componentes actualizan además el índice de
componentes y se verifican contra recomputación completa.

### Partición persistente

El orden lexicográfico final se conserva entre revisiones. Después de una
edición se retiran únicamente los vértices cuya etiqueta final cambió, se
ordenan esos `C` vértices y se fusionan con el orden retenido. El coste pasa de
`O(V log V)` a `O(V + C log C)` manteniendo exactamente los mismos ids de celda
que el análisis completo.

### Publicación transaccional

`IncrementalGraphWorkspace` contiene overlays de dos capas, journals de
escritura, agregados staged, partición staged e índice staged. Los journals no
se aplican a `IncrementalGraphState` hasta haber validado:

- identidad y dimensiones;
- todos los productos e inversiones;
- transcript y contadores;
- partición y componentes;
- incremento de revisión.

Un error deja grafo, etiquetas, firma, partición, componentes y revisión
anteriores intactos. El workspace puede reutilizarse incluso después del
error. `reserve_for` permite reservar el peor caso; la preparación inmutable del
grafo nuevo mantiene sus propias asignaciones fuera de esa garantía.

## Complejidad real

Sea `A_r` el frontier de la ronda `r` y `I(A_r)` sus filas incidentes:

```text
auditoría segura       O(V + I)
preparación            O(V + D)
propagación algebraica O(K · Σr (A_r + I(A_r)))
agregados              O(K · número_de_etiquetas_cambiadas)
partición              O(V + C log C)
índice tras topología  O(V + I)
```

No se afirma que la latencia total sea sublineal: la auditoría y la publicación
de la partición siguen siendo lineales. Sí se evita repetir `R` recorridos
algebraicos globales, que son la parte dominante para campos o perfiles caros.

## Pruebas

`tests/fast_graph.rs` incluye 24 pruebas del dominio, seis específicas de F6.G4:

- edición local sobre un camino de 257 vértices y frontier limitado al radio;
- no-op sin nueva revisión;
- fusión y separación exactas de componentes;
- estado intacto ante identidad o cardinalidad incompatibles;
- 48 revisiones diferenciales consecutivas en F251, Goldilocks y GF(2²⁵⁶);
- relación, rol, dirección, multiplicidad, bucle, duplicados e incidencia
  auxiliar;
- retirada e inserción explícitas de un factor agregado cero.

En todos los casos se comparan etiquetas, partición, firma, transcript y
metadatos completos con `FastGraphLabeler::analyze`, no únicamente las lanes
finales.

Gates ejecutados en verde:

- `cargo test --workspace --all-features --all-targets`;
- 450 unit tests legados, 24 tests de grafos y benches en modo test;
- Clippy del workspace con `-D warnings`;
- rustdoc del workspace con warnings como errores;
- formato y `git diff --check`;
- Miri focalizado sobre fusión/separación de componentes y deltas con factores
  cero.

## Rendimiento observado

Criterion `--quick`, Intel i7-13700HX, release LTO, F251/K=3/R=4 y cuatro arcos
salientes por vértice. El setup de Criterion realiza fuera de la medición el
clone necesario para entregar propiedad del grafo nuevo.

| Operación | 1.024 | 16.384 | 131.072 |
|---|---:|---:|---:|
| recomputación completa | 421–430 µs | 6,82–6,89 ms | 53,53–53,88 ms |
| una etiqueta incremental | 201–206 µs | 2,80–2,83 ms | 21,44–21,58 ms |
| una arista incremental | 256–257 µs | 3,38–3,39 ms | 29,58–30,36 ms |

La edición de etiqueta es aproximadamente `2,0–2,5×` más rápida y la edición de
arista `1,6–2,1×` en este host. La segunda incluye reconstrucción lineal del
índice y componentes. Son mediciones locales, no garantías ABI.

## Límites y continuación

F6.G4 no actualiza incrementalmente el canal SHA-256 híbrido: su contrato ordena
descriptores globales y debe recalcularse completo. Tampoco resuelve simetrías
ni prueba isomorfismo.

F6.G5 y F6.G6 se completaron después de este corte: diagnóstico de degeneración,
evidencia multi-campo, oráculos adversariales y canonización exacta acotada. El
cierre consolidado está en
[phase-6-g5-g6-final-report.md](phase-6-g5-g6-final-report.md).
