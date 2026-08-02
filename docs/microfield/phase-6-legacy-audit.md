# Auditoría integral del legado previa a canonización

Fecha: 2 de agosto de 2026.

## Alcance y criterio

La auditoría cubre el paquete raíz `homomorphic-hash-rs`: álgebra, topologías,
motor, residuos históricos, canonizador, Bloom, espectral, dominios, harness,
ejemplos, pruebas y benchmarks. Separa tres clases de resultado:

1. **ley algebraica válida**, que puede migrarse con contratos explícitos;
2. **compatibilidad histórica**, que se conserva mediante un adapter nominal;
3. **afirmación no demostrada**, que se retira o se congela hasta disponer de
   una especificación exacta.

El objetivo no es seguridad criptográfica. Una firma estructural es una
evaluación pequeña y composable que captura propiedades algebraicas. Puede
colisionar y una igualdad de firmas no demuestra igualdad de objetos.

## Inventario y resolución

| Área legada | Hallazgo | Resolución |
|---|---|---|
| `algebra::GaloisSignature256` | Reimplementaba GF(2²⁵⁶), duplicaba reducción e inversión y asociaba rendimiento a código que no ejecutaba SIMD | Conserva layout público alineado y bytes históricos, pero delega toda aritmética en `microfield::Gf2_256HhV1` |
| `algebra::FiniteField` | Trait monolítico, representación fija de 32 bytes y acoplamiento a un solo campo | Queda como frontera de compatibilidad; el código nuevo usa capacidades segregadas de `microfield` |
| `SymmetricDifferenceAggregator` | La suma en característica dos conserva paridad de multiplicidad, no un conjunto exacto | `AdditiveSignature` declara esa ley, conserva un contador exacto y permite combinar particiones |
| `SequenceAggregator` | Horner válido, pero el estado omitía longitud; el índice era ignorado y cualquier elemento supuesto como último genera un residuo recomponible | `SequenceSignature` incorpora base, longitud, identidad y concatenación exacta; `TrackedSequence` garantiza `pop` real |
| `MultisetAggregator` | Producto conmutativo válido, pero un factor cero destruía información y dividir por cualquier factor no nulo fabricaba una supuesta pertenencia | `MultisetSignature` separa producto no nulo, cardinalidad y contador de ceros; `TrackedMultiset` aporta pertenencia/multiplicidad exactas fuera del hash |
| Aplanado affine del multiset | Limpiar un bit no demuestra globalmente que el factor affine sea distinto de cero tras todos los chunks | Se conserva como `LegacyAffineEncoderV1`; el estado nuevo cuenta ceros y no depende de esa hipótesis |
| Embedding lineal de bytes | Determinista y compatible, pero sin framing de longitud y con colisiones conocidas | `LegacyLinearEncoderV1` congela los bytes; los encoders nuevos enmarcan longitud implícita y dominio antes de reducir |
| `ProofGenerator` / `ProofVerifier` | La “prueba de inclusión” era una ecuación inversa. En un campo, todo factor no nulo es divisible; la propia suite mostraba falsificaciones tautológicas | Se renombra conceptualmente a `ResidualGenerator` / `ResidualVerifier`; los nombres antiguos permanecen como fachada y documentan que no prueban pertenencia |
| `TopoHasher` | Fachada útil para congelar llamadas antiguas, pero sin `FieldId`, `EncoderId`, ley ni schema | Se mantiene como adapter legacy. La fachada soportada para código nuevo son las firmas de `structural` |
| `crypto_mode` | El nombre sugería garantías de tiempo constante/seguridad inexistentes | Solo queda por compatibilidad fuente y su documentación niega expresamente garantías criptográficas |
| `TopoBloomMask` | Resumen probabilístico dependiente del universo de índices | Se congela para el track de grafos; no puede convertirse en prueba exacta ni en identidad autoritativa |
| `CellularGaloisCanonizer` | Refinamiento heurístico sobre firmas, sin forma canónica exacta ni demostración bicondicional de isomorfismo | Ya no contiene un algoritmo: es una fachada sobre `from_legacy_topology` y `F251GraphLabeler`; `try_analyze` expone el resultado mantenido y `canonize` solo empaqueta tres lanes por compatibilidad |
| `SpectralEngineF251` | Heurística espectral con colisiones y semántica ligada al grafo | Se conserva experimental fuera del canonizador autoritativo, pero su aritmética delega ahora en `microfield::Fp251V1` |
| `domains::chemistry` | Demos y experimentos interpretan ausencia observada de colisiones como evidencia más fuerte de la disponible | Se mantienen compilables, no como evidencia de corrección. Su rehabilitación semántica depende del modelo de grafo |
| `harness` | Infraestructura de medición útil, pero resultados históricos mezclaban algoritmos y claims | Se conserva compilable; los nuevos benchmarks aíslan las tres leyes estructurales |
| benches auto-descubiertos | Algunos comparaban construcciones no equivalentes, dependían de API ausente o eran placeholders | `autobenches = false`; solo se registra el benchmark mantenido `structural_signatures` |
| 450 unit tests del paquete raíz | Red amplia de regresión; algunos tests documentan intencionadamente tautologías y colisiones | Los tests diagnósticos permanecen como evidencia negativa. Las aserciones del canonizador que exigían la recurrencia retirada fueron sustituidas por contratos del puente y todos sus tests atraviesan ahora el motor nuevo |

## Semántica matemática recuperada

Sea `e(x)` el encoder identificado y `F` el campo identificado.

### Aditiva

```text
A(X) = Σ e(x)
A(X ⊎ Y) = A(X) + A(Y)
```

Se conserva además `term_count`. En característica dos, repetir dos veces un
valor cancela su evaluación, pero no el contador. Por tanto captura paridad y
composición, no igualdad exacta de multiconjuntos.

### Secuencia

```text
S([]) = 0
S(xs || [x]) = S(xs) · b + e(x)
S(A || B) = S(A) · b^len(B) + S(B)
```

La longitud forma parte del estado serializado y la base forma parte de
`SignatureId`. El residuo para un supuesto último término solo comprueba la
ecuación anterior. La lista rastreada es la que acredita cuál fue realmente el
último elemento.

### Multiconjunto

```text
factor(x) = e(x) + offset
M(X) = Π factor(x)
```

Internamente se guarda el producto de factores no nulos, el número total y el
número de factores cero. Así pueden combinarse particiones y retirarse ceros
rastreados sin perder permanentemente el producto restante. La firma compacta
no puede acreditar pertenencia: para todo factor no nulo existe un cociente.

## Fronteras de corrección

- `FieldId` fija la presentación matemática y el encoding del campo.
- `EncoderId` fija el mapeo byte→campo y el dominio; el límite local de memoria
  no altera la semántica.
- `SignatureId` liga campo, encoder, ley y parámetros como base u offset.
- SHA-256 se usa para identidad estable y compacta, no como promesa de
  resistencia criptográfica del hash estructural.
- Los estados incompatibles fallan antes de mutar la salida.
- La serialización `MFSG` schema 1 es little-endian, autocontenida y rechaza
  bytes extra, elementos no canónicos y metadatos imposibles.
- Los encoders de producción aplican framing y separación de dominio antes de
  reducción. La reducción puede colisionar porque el codominio es finito.
- Las variantes `Tracked*` conservan la estructura exacta fuera de la firma;
  son la frontera cuando una operación necesita pertenencia u orden reales.

## Código deliberadamente congelado

El canonizador, Bloom, espectral y las aplicaciones de química no se
reescriben aún porque hacerlo fijaría accidentalmente un modelo de grafo. Antes
de tocarlos hay que decidir dirección, lazos, multiaristas, etiquetas,
atributos, orden de serialización y límites de recursos. Hasta entonces son
compatibilidad/experimentos, nunca el camino autoritativo nuevo.

## Evidencia exigida al cierre

- regresión completa de los 447 tests legacy, también con `crypto_mode`;
- leyes genéricas sobre las tres familias binarias y las tres primas
  mantenidas;
- prueba exhaustiva de los 251 elementos y offsets para el caso de factor cero;
- colisiones reales de encoder verificadas contra la variante rastreada;
- round-trip y rechazo adversarial del formato canónico;
- fallos masivos transaccionales y overflow de contadores;
- compatibilidad byte a byte de los adapters legacy;
- cero asignaciones en el camino inline de las tres leyes;
- Clippy, rustdoc, Miri, workspace y benchmark mantenido.
