# Informe de cierre F6.G3

Fecha: 2 de agosto de 2026.

## Resultado

F6.G3 cierra el camino de análisis masivo sin crear un segundo algoritmo. El
flujo autoritativo sigue siendo `FastGraphLabeler<F, E, K>`; preparación,
workspace, Rayon y el experimento AVX2 cambian únicamente cómo se ejecuta la
misma recurrencia.

La migración del legado queda incluida en el cierre. El tipo histórico
`CellularGaloisCanonizer` conserva compatibilidad fuente, pero ya no ejecuta
su inicialización espectral seguida de message passing GF(2²⁵⁶). Convierte el
`TopologyProvider` a `IncidenceGraph`, conserva cada cláusula como hiperarista y
delega en `F251GraphLabeler<3>`. `try_analyze` devuelve el grafo normalizado y
el `FastGraphAnalysis` identificado; `canonize` solo empaqueta las tres lanes
F251 en el contenedor antiguo.

## Implementación

### Preparación persistente

`FastGraphLabeler::prepare` produce `PreparedGraph<'g, F, K>` ligado por
préstamo a un único grafo y por `GraphSignatureId` a un único labeler. Conserva:

- etiquetas iniciales;
- descriptores codificados;
- constante afín de salida y entrada por descriptor y lane;
- tokens de actualización y transcript por ronda.

Así se retiran del bucle repetido el framing, la reducción byte→campo y dos
sumas por incidencia/lane. Multiplicidades 2, 3 y 4 usan cadenas especializadas
de `square`/`mul`; el resto conserva el `Pow` genérico.

### Workspace y vistas

`GraphWorkspace<F, K>` es el único propietario de buffers mutables:

- labels actuales y siguientes;
- orden temporal de clasificación;
- partición actual y anterior;
- mapas bidireccionales para comparar particiones sin asignar;
- agregados de cada ronda.

`analyze_prepared_with_workspace` devuelve `FastGraphAnalysisView`; el borrow
impide reutilizar o mutar el workspace mientras el resultado está vivo. Después
de `reserve_for`, la ruta secuencial no asigna. `to_owned` es el límite explícito
que copia cuando el resultado debe persistir.

`analyze_prepared_hybrid_with_workspace` reutiliza la parte lineal. SHA-256
continúa asignando y ordenando sus entradas porque es un canal opt-in con
complejidad distinta.

### Paralelismo determinista

`GraphExecution` separa política de ejecución de identidad matemática. Cada
vértice de una ronda solo lee las labels anteriores y escribe una celda
exclusiva de salida; Rayon distribuye esos índices sin cambiar el orden del
vector final. Agregación y transcript usan la misma aritmética exacta. Pruebas
con pools de 2, 3 y 4 hilos comparan el resultado completo, no solo la firma.

La conveniencia `GraphExecution::parallel()` parte de 1.024 vértices en el host
calibrado. El consumidor puede fijar otro umbral. Un único hilo o un grafo menor
ejecutan el recorrido secuencial.

### Evaluación SIMD real

`F251BatchGraphWorkspace<K>` mantiene los buffers del bridge AoS↔SoA y selecciona
una vez un `microfield::Engine<Fp251V1>`. La reducción irregular de vecindarios
permanece sobre CSR; las cinco etapas Horner regulares se ejecutan por batches.
En x86-64 con AVX2 y batches suficientes se usa `X86PrimeAvx2`; en otros hosts
se conserva el backend portable.

La estrategia es explícita mediante `analyze_prepared_f251_batched`. No se
promueve globalmente porque el bridge recorre varias veces los arrays y compite
con la segmentación Rayon. El criterio es el flujo completo, no el kernel
aislado.

## Migración y depuración de pruebas legadas

Los 450 unit tests del paquete raíz siguen ejecutándose, pero los tests del
canonizador atraviesan ahora el adapter y el motor F251 nuevo. Dos aserciones
que exigían valores exactos de la recurrencia retirada (`S²·Φ` y absorción del
vacío como `1`) fallaron inmediatamente durante la migración; se eliminaron
como requisitos y se sustituyeron por:

- uso real del grafo de incidencias, incluida la cláusula auxiliar;
- número exacto de rondas;
- igualdad de nodos simétricos y separación del aislado;
- rechazo preciso de cero rondas y de más de 64 en `try_analyze`;
- equivalencia completa entre `try_analyze` y una llamada directa al
  `F251GraphLabeler`.

Los tests que exhiben tautologías, colisiones o falsos positivos antiguos se
mantienen como evidencia negativa y no como garantías. El motor espectral
experimental sigue disponible por compatibilidad, pero toda su suma y producto
usan ahora `microfield::Fp251V1`; ya no reimplementa módulo 251.

## Evidencia de rendimiento

Host: Intel i7-13700HX, 24 hilos lógicos, AVX2, Linux x86-64,
`rustc 1.96.0-nightly`, release LTO. Criterion `--quick`, K=3, cuatro rondas,
cuatro arcos salientes por vértice. El throughput cuenta las visitas de salida
y entrada de cada ronda.

| Ruta | 1.024 | 16.384 | 131.072 |
|---|---:|---:|---:|
| owned end-to-end | 80,2–80,6 M/s | 74,6–75,2 M/s | 79,4–80,4 M/s |
| prepared AoS secuencial | 114,3–114,6 M/s | 110,1–110,6 M/s | 110,7–111,3 M/s |
| prepared AoS Rayon | 135,3–139,5 M/s | 373,1–377,7 M/s | 466,7–490,0 M/s |
| prepared SoA AVX2 | 124,4–125,8 M/s | 127,2–131,2 M/s | 123,9–125,5 M/s |
| SoA AVX2 + Rayon CSR | 129,2–131,2 M/s | 221,2–227,5 M/s | 369,2–377,8 M/s |

La preparación persistente reduce aproximadamente un 28–32 % el tiempo frente
al resultado owned con preparación repetida. AVX2 mejora la ruta secuencial,
pero AoS+Rayon ofrece el mejor resultado grande y evita los pases SoA. Estas
cifras son evidencia local, no un ABI de rendimiento.

## Gates de corrección

- igualdad completa entre façade owned, vista preparada y ruta robusta;
- igualdad entre análisis híbrido preparado y no preparado;
- cero asignaciones secuenciales después de reservar;
- 24 multigrafos pseudoaleatorios con bucles, labels variables y
  multiplicidades 1, 2, 3, 4, 5 y 257: scalar = Rayon = batch;
- AVX2 seleccionado y comparado contra scalar cuando el host lo soporta;
- 64 renumeraciones deterministas y campo externo GF(2⁹);
- composición de unión disjunta, colisión F251 separada por SHA-256 y
  canonización únicamente discreta;
- suite raíz, workspace, Clippy, rustdoc, Miri focalizado y benchmark release.

## Límite y siguiente hito

F6.G3 no proporcionaba actualización incremental ni canonización exacta con
simetrías. F6.G4 ha cerrado posteriormente el primer límite mediante índice de
dependencias, recomputación por radio y resultado diferencialmente idéntico a
un análisis completo. Véase
[phase-6-g4-final-report.md](phase-6-g4-final-report.md). F6.G5 tratará familias
adversariales; F6.G6 reservará la búsqueda exacta para un perfil con presupuesto
explícito. Ambos cortes se completaron posteriormente y están consolidados en
[phase-6-g5-g6-final-report.md](phase-6-g5-g6-final-report.md).
