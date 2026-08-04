# ADR 0029 — Degeneración diagnosticada y canonización exacta acotada

- Estado: aceptado
- Fecha: 2 de agosto de 2026

## Contexto

Una firma de campo finito puede perder información por dos motivos diferentes.
Primero, el encoding o la recurrencia pueden colisionar al reducirse en un
campo finito. Segundo, incluso con mensajes exactos, el refinamiento local
1-WL no distingue ciertas estructuras regulares. Aumentar lanes o cambiar de
campo reduce el primer riesgo, pero no resuelve el segundo.

La igualdad de una firma algebraica y de su canal SHA-256 seguía siendo una
condición necesaria útil, no una prueba de isomorfismo. Faltaba una frontera
operativa que detectase cuándo escalar y que nunca publicase un resultado
parcial como exacto.

## Decisión

1. `GraphDegeneracyReport` compara la partición del perfil de campo con una
   partición 1-WL calculada sobre bytes, dirección, relación, rol y
   multiplicidad exactos.
2. Si la partición exacta divide una clase del campo, se informa
   `field_aliasing`: perfiles independientes pueden mejorar la evidencia.
3. Si la partición exacta conserva clases no unitarias, se informa ambigüedad
   local. Más rondas o campos no se presentan como solución garantizada.
4. La marca `highly_regular` v1 se activa con al menos cuatro vértices, 75 % de
   vértices en clases exactas no unitarias y una clase de al menos 25 % de `V`.
   Es una señal de routing versionada, no una propiedad matemática universal.
5. `MultiFieldGraphEvidenceBuilder` agrupa canales heterogéneos, ordena sus
   `GraphSignatureId` y deriva `GraphEvidenceProfileId`. Su comparación solo
   devuelve `Different` o `Indistinguishable`.
6. `canonicalize_exact` es opt-in. Usa la ruta rápida si la partición de campo
   es discreta; en otro caso ejecuta individualización–refinamiento exacto.
7. El presupuesto limita nodos y celdas retenidas del frontier. Si se agota,
   el único resultado es `BudgetExhausted`; no se expone un candidato parcial.
8. La búsqueda no entra en `analyze`, `analyze_hybrid`, batch ni actualización
   incremental.

## Consecuencias

El flujo barato conserva coste lineal por ronda. Los consumidores pueden
separar aliasing aritmético de degeneración combinatoria antes de gastar más
recursos. Una forma `Exact` permite comparar representantes canónicos bajo el
mismo `GraphSignatureId`; un bundle indistinguible o una firma igual nunca se
renombra como isomorfismo.

La canonización exacta sigue teniendo complejidad exponencial en el peor caso.
El límite de memoria cubre estados retenidos por DFS, no todas las asignaciones
temporales de ordenación del refinamiento exacto. La política de alta
regularidad puede evolucionar únicamente con una nueva versión documentada.

## Alternativas rechazadas

- **Añadir rondas hasta que cambie algo:** los grafos regulares pueden conservar
  una sola clase indefinidamente.
- **Usar un campo mayor o SHA-256 como prueba:** no separa descriptores locales
  que ya son exactamente iguales.
- **Romper empates con el índice de entrada:** destruye invariancia por
  renumeración.
- **Publicar el mínimo encontrado al agotar presupuesto:** confunde una cota
  parcial con una forma canónica.
- **Activar búsqueda exacta automáticamente en el hot path:** elimina la
  predictibilidad que motiva el etiquetador rápido.
