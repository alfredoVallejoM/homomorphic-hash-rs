# Auditoría de firmas homomórficas, deltas e infraestructura de campos

Fecha: 4 de agosto de 2026.

## Veredicto

La biblioteca dispone de primitivas algebraicas correctas y, desde RC.3, de un
núcleo público de deltas por ley con revisión, persistencia y replay. Todavía no
ofrece los adapters completos de archivos, bases de datos o árboles
jerárquicos: estos necesitan schemas, fuentes autoritativas y políticas de
particionado propias del dominio.

Las firmas son deliberadamente no criptográficas. No autentican al emisor, no
prueban pertenencia y no convierten una igualdad de campo en igualdad exacta de
los datos. Su valor está en actualizar y combinar información algebraica sin
releer el conjunto completo.

## 1. Capacidad existente

| Capacidad | Estado | Observación |
|---|---|---|
| suma de particiones | implementada | `AdditiveSignature::combine` |
| concatenación ordenada | implementada | `SequenceSignature::concatenate` |
| concatenación bidireccional | implementada | conserva ambas orientaciones |
| unión de multiconjuntos | implementada | producto, cardinalidad y ceros |
| multievaluación | implementada | K puntos/bases y assurance acotado |
| ingestión transaccional | implementada | no publica lotes parciales |
| wire compacto identificado | implementado | schema `MFSG` v1 |
| retirada exacta | parcial | solo `TrackedMultiset` conoce pertenencia |
| rollback ordenado exacto | parcial | solo `TrackedSequence::pop`, último item |
| residual compacto | implementado | verifica una ecuación, no el historial |
| paridad static/runtime | implementada en firmas mantenidas | misma presentación, mismo contexto/wire |
| reconciliación acotada | mantenida desde RC.5 | API pública Fp251, perfiles, wire y límites; v1 solo conjuntos |
| delta público persistible | implementado en RC.3 | `MFDE`, revisión y tipos segregados para suma, multiset y extremos de secuencia |
| árbol jerárquico de firmas | no implementado | la concatenación ya aporta el álgebra necesaria |

## 2. Qué significa verificar un delta

Se distinguirán tres niveles sin usar terminología probatoria:

1. **Consistencia algebraica**: el estado anterior, el cambio declarado y el
   estado posterior satisfacen la ley de la firma.
2. **Cambio autorizado por la fuente**: el sistema de archivos, log de
   transacciones o índice exacto confirma que los valores retirados existían y
   que la revisión era la esperada.
3. **Equivalencia exacta reconstruida**: se reconstruyen los datos afectados y
   se comparan exactamente.

El primer nivel es O(1) u O(K), pero no demuestra que el cambio refleje el
historial real. El segundo y el tercero necesitan una fuente de verdad externa
o una variante `Tracked*`. Ninguna API se llamará `proof`, `membership` o
`authenticated` cuando solo compruebe una ecuación de campo.

## 3. Contrato de delta implementado en RC.3

No existe una única operación inversa válida para todas las leyes. Se expondrán
tipos segregados:

```text
AdditiveDelta       = removed_sum + added_sum + count transition
MultisetDelta       = removed_factors + added_factors + cardinality transition
SequenceAppend      = suffix signature
SequenceTrim        = known suffix signature
SequenceRangeDelta  = path in a hierarchical sequence index
```

Todos compartirán un envelope con:

```text
delta_schema
SignatureContext
application_namespace
source_revision
target_revision
operation_count
law-specific payload
```

Aplicar un delta seguirá una transacción de dos fases: validar identidad,
revisión, contadores, factores cero, longitudes y límites sobre temporales; solo
después publicar el estado nuevo. Un fallo deja intactos estado, revisión y
salida.

La resta compacta es una operación **asumida**: permite actualizar cuando la
fuente autoritativa ya validó el valor retirado. Si la propia estructura debe
detectar borrados inexistentes necesita tracking exacto o consultar la fuente.

## 4. Archivos y secuencias de chunks

La firma de secuencia permite calcular:

```text
H(A || B) = H(A) · base^len(B) + H(B)
```

RC.4 incorpora `FileChunkProfileId`, framing `MFFC`, longitud separada de
cardinalidad, `HomomorphicSummaryTree`, edits de rango y checkpoint `MFST`. Los
reemplazos que conservan longitud recomponen los caminos afectados; cualquier
cambio de fronteras selecciona un rebuild completo y transaccional.

Permanecen pendientes el adapter streaming/I/O, chunking content-defined, un
journal específico de edits de archivo y autenticación externa cuando el
workload la requiera.

La firma no puede descubrir de forma fiable un edit arbitrario comparando solo
dos raíces. El generador de delta debe leer metadata/chunks diferentes o usar
un índice exacto. La firma sirve para recomposición rápida, localización
candidata y comprobación algebraica del resultado.

## 5. Bases de datos

El modelo natural es un multiconjunto de filas canónicas o una colección de
particiones por clave. Un update se representa como retirada de la imagen
anterior e inserción de la nueva.

RC.5 define `DatabaseSchemaId`, encoding `MFRW`, clave primaria, versión de
fila, `TransactionDelta`, particiones y log idempotente. V1 declara claves
únicas: los duplicados necesitan un discriminante explícito en la clave.

Permanecen fuera del núcleo los adapters SQL, el mapeo concreto de LSN y la
reconstrucción diferencial contra una base real tras crash.

Para una base de datos autoritativa no es necesario retener todas las filas en
`TrackedMultiset`: el motor de almacenamiento valida la existencia y entrega la
imagen anterior. Sin before image, la firma compacta no puede certificar que un
borrado era legítimo.

## 6. Árbol jerárquico compatible con Merkle

RC.4 añade `HomomorphicSummaryTree`, con la misma topología útil de un árbol
Merkle pero semántica algebraica no criptográfica. Cada nodo conserva contexto,
longitud lógica y firma compuesta de sus hijos. Un cambio de hoja recalcula
únicamente su camino hasta la raíz.

El perfil del árbol fijará fanout, orden de hijos, tratamiento de nodos
incompletos, hojas vacías, framing y schema. Cambiar cualquiera de estos
parámetros crea otra identidad.

Una aplicación que ya use un árbol Merkle puede almacenar la firma homomórfica
como canal adicional en cada nodo. Este canal permite deltas y agregaciones;
no sustituye la autenticación que la aplicación pueda requerir por otros
medios. Dentro de `microfield` solo se ofrecerán consistencia algebraica,
recomposición y comparación exacta opcional de hojas.

## 7. La infraestructura de campos es el fundamento

Las firmas deben seguir siendo genéricas sobre la infraestructura existente:

- campos binarios mantenidos y externos generados;
- campos primos mantenidos y externos generados;
- `DynField` para descriptores runtime validados;
- `FieldId` y encoding canónico como identidad nominal;
- engines portable/ISA, batch y storage packed donde estén validados;
- certificados de irreducibilidad/primalidad y assurance de construcción.

No existe un campo universalmente mejor para todos los deltas:

- en característica dos la firma aditiva conserva paridad; una multiplicidad
  par desaparece de la suma aunque el contador global permanezca;
- un campo primo conserva multiplicidades módulo `p`, con su propio límite;
- un campo de mayor cardinalidad reduce colisiones accidentales, pero no aporta
  seguridad ni inyectividad;
- varias evaluaciones aumentan coste y estado de forma lineal en K;
- un campo runtime prioriza flexibilidad; un tipo generado permite
  monomorfización y backends optimizados.

G15 debe publicar un `SignatureFieldProfile` que seleccione familia, campo,
encoder, ley, K y límites según el workload. La selección nunca será un nombre
hardcodeado de tres campos ni una promesa implícita de seguridad.

## 8. Verificaciones necesarias para viabilidad

### Leyes y campos

- matriz idéntica sobre campos binarios, primos, generados y runtime;
- árboles de partición y asociación de todas las formas normativas;
- equivalencia static/runtime y scalar/batch;
- casos específicos de característica dos, multiplicidad `p` y factores cero;
- perfiles de colisión/degeneración por campo, encoder, ley y K.

### Deltas

- secuencias aleatorias de cambios frente a rebuild exacto después de cada paso;
- revisiones obsoletas, replay, duplicación y reordenación de deltas;
- overflow y fallo de encoding en cada posición del lote;
- retirada inexistente con y sin fuente exacta;
- round-trip de envelope y rechazo de cualquier identity drift;
- recuperación después de truncar el journal en cada frontera de escritura.

### Archivos, base de datos y árbol

- archivos vacíos, grandes, chunks frontera y ráfagas de edits;
- append/truncate/replace/insert/remove frente a relectura completa;
- transacciones de filas con duplicados, `NULL`, cambios de schema y rollback;
- raíz del árbol incremental frente a reconstrucción completa;
- complejidad observada O(log n) por hoja para una forma congelada;
- fuzzing de wires, journals, parsers y snapshots;
- benchmark de actualización completa, delta, merge y restore, separando el
  coste de crear el cambio del coste de aplicarlo.

## 9. Criterio de cierre

Las firmas serán viables para consumo interno cuando:

1. exista una API de delta por ley con errores fail-closed;
2. archivos, bases de datos y árbol jerárquico tengan schemas versionados;
3. cada operación incremental coincida con rebuild exacto en campañas largas;
4. la reconciliación deje de depender de código privado del laboratorio;
5. snapshots y journals sobrevivan restart y corrupción parcial;
6. el consumidor elija explícitamente campo y perfil o use una política
   documentada ligada al workload;
7. toda verificación declare si es algebraica, autorizada por la fuente o
   exacta;
8. exista una ventaja medida en CPU, I/O, memoria o comunicación frente a
   reconstruir el estado completo.
