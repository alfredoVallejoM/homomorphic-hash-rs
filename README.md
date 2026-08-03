# Homomorphic Hash RS / Microfield

Este repositorio contiene dos paquetes con ciclos de vida independientes:

- `homomorphic-hash-rs`, el prototipo legado de hashes y agregación topológica;
- `microfield`, el nuevo núcleo portable de campos finitos binarios y primos.

`microfield` se desarrolla como un paquete único dentro del workspace. Sus
fronteras internas siguen SOLID, dispatch estático en operaciones escalares y
selección previa de estrategia para operaciones por lote.

## Estado

El scaffold y la Fase 0 mínima de `microfield` están implementados. El
generador normaliza manifiestos estrictos, calcula identidades, certifica los
tres polinomios con Rabin, deriva planes y publica artefactos
transaccionalmente. SageMath 10.7 ha producido vectores externos reproducibles
para los tres campos. El vertical H2 se integró en `main` y H3 generaliza la
misma aritmética portable sobre `Gf2_128V1`, `Gf2_256HhV1` y
`Gf2_256AltV1`. Los tres tipos son públicos, nominalmente distintos y comparten
algoritmos monomorfizados. H4 incorpora el motor batch portable y está integrado
en `main`; con ello la Fase 1 está cerrada. En Fase 2, H2.1 incorpora la
factory estática y H2.2 optimiza los campos externos mediante planes portables
deterministas; H2.3 cierra capabilities/selección y H2.4 añade el backend batch
x86-64 PCLMUL para los tres presets. El puente ABI 3 permite que cualquier
campo externo validado reciba perfiles ISA verificados sin abrir catálogos ni
punteros. H2.5 añade PMULL en AArch64 para presets y perfiles externos; queda
en selección explícita hasta disponer de calibración reproducible en hardware
ARM real. H2.6 incorpora `PackingPlan`, `PackedBatch` owned y vistas sobre
storage externo alineado. H2.7 añade VPCLMUL y `AosLanePairs` para presets y
campos ABI 3; queda forzable pero fuera de selección automática tras medir una
regresión en 256 bits. H2.8 cierra la Fase 2 con tabla de selección versionada,
corpus diferencial persistente, inventario hash de `unsafe`, captura Criterion
multi-runner y matriz runtime/codegen. Solo PCLMUL participa en selección
automática; PMULL y VPCLMUL continúan disponibles mediante selección explícita.
La Fase 3 incorpora inversión batch tolerante a cero, scans, Horner batch,
potencias fijas, workspaces tipados e IR v4 de inversión verificado. No añade
asignaciones ocultas ni nuevas fronteras `unsafe`.

La Fase 4 añade `Fp251V1`, `FpGoldilocks64V1` y `Fp256GenericV1`. Los tres
campos tienen certificados reproducibles, encoding canónico, algoritmos
portables, bundles autenticados y vectores Sage. AVX2 acelera los lotes de 251
desde 64 elementos y Goldilocks desde 4. La extensión F4.6-SIMD añade además
bridges AVX2 explícitos para primos externos canónicos `u8`/`u16` y desenrolla
dos pares VPCLMUL; BMI2 dispone de un factory interno genérico para
representaciones Montgomery radix 64, con carry fijo y corrección branchless.
Es compatible con `FixedSchedule`, pero queda explícito porque aún pierde
ligeramente frente a portable en el build auditado. Los bridges externos no se
promueven sin calibración propia. La factory completa de primos externos se
reserva para Fase 5.

F4.7-PACKED-SIMD está implementada: `PackedBatch<F>` puede conservar storage
privado `u8`/`u16`/`u32`, ejecutar cinco operaciones sin repacking y convertir
solo al entrar y salir. El perfil externo `u16` supera conservadoramente el
58 % desde 64 elementos; los perfiles externos permanecen explícitos. El ABI
batch ordinario y los kernels especializados no cambiaron.

La Fase 5 está cerrada: incluye generación certificada de perfiles primos
externos, assurance probado/probable, bundles reproducibles, contextos
dinámicos y el puente hacia perfiles estáticos. F6.0–F6.8 rehabilitan ahora el
legado algebraico: `GaloisSignature256` delega en `microfield` y el nuevo módulo
`structural` ofrece suma/paridad, secuencias Horner, secuencias bidireccionales
y multiconjuntos de una o varias evaluaciones con identidades, contadores,
factores cero, tracking exacto y serialización canónica. La misma capa funciona
con campos estáticos mantenidos, campos externos generados y, mediante la
feature `dynamic-fields`, contextos validados construidos en runtime. Son
hashes homomórficos no criptográficos: capturan ecuaciones y propiedades
algebraicas, pero una colisión no prueba igualdad ni un residuo prueba
pertenencia. F6.G0–G2 introducen además `IncidenceGraph` y
`FastGraphLabeler<F, E, K>`: un motor relacional lineal por ronda, invariante
por renumeración, que preserva dirección, roles, multiplicidad e hiperaristas.
F251 es un perfil mantenido de primer nivel y el mismo algoritmo funciona sobre
campos externos generados. `analyze_hybrid` combina la firma algebraica con un
SHA-256 de descriptores invariantes adicionales. La canonización no se ejecuta
en el camino crítico: solo se emite una forma exacta cuando la partición rápida
queda discreta. Las firmas `Fast` de componentes desconectadas se combinan
exactamente mediante `combine_disjoint`.

F6.G3 cierra la optimización a gran escala. `prepare` conserva metadatos y
constantes afines; `GraphWorkspace` ofrece análisis prestados sin asignaciones
tras reservar; `GraphExecution::parallel()` reparte vértices de forma
determinista; y `F251BatchGraphWorkspace` permite comparar explícitamente la
ruta SoA/AVX2. En la máquina de calibración, el batch AVX2 mejora el escalar de
un hilo, pero AoS+Rayon es claramente mejor cuando hay varios núcleos, por lo
que SIMD no se fuerza automáticamente. El viejo `CellularGaloisCanonizer` es
ahora una fachada sobre este mismo motor F251 y `try_analyze` expone el
resultado mantenido completo; ya no existe una segunda recurrencia de grafos.
El cierre y las mediciones están en
[`phase-6-g3-final-report.md`](docs/microfield/phase-6-g3-final-report.md).

F6.G4 añade incrementalidad exacta para perfiles `Fast` con un conjunto de
vértices estable. `IncrementalGraphState` conserva las capas de ronda y un
índice acotado de dependencias; `update_incremental` audita el grafo nuevo,
recalcula solo el cono afectado y publica transaccionalmente etiquetas, firma,
partición y componentes. Cambiar una arista puede unir o separar componentes
sin abandonar el mismo contrato diferencial. Las mediciones y límites están en
[`phase-6-g4-final-report.md`](docs/microfield/phase-6-g4-final-report.md).

F6.G5–G6 cierran la Fase 6. `diagnose_degeneracy` separa colisiones del perfil
finito de la indistinguibilidad local exacta y marca alta regularidad con un
umbral versionado. Los perfiles heterogéneos pueden agruparse mediante
`MultiFieldGraphEvidenceBuilder`, cuya igualdad significa únicamente
`Indistinguishable`. `canonicalize_exact` escala de forma opt-in a
individualización–refinamiento con límites de nodos y estado retenido; si no
termina devuelve `BudgetExhausted` sin publicar una forma parcial. SageMath
10.7 confirma que `C6` y `C3 ⊔ C3` ya forman un par regular no isomorfo que no
separa ninguna cantidad de rondas locales; Shrikhande frente a torres 4×4
cubre el caso fuertemente regular. El cierre está en
[`phase-6-g5-g6-final-report.md`](docs/microfield/phase-6-g5-g6-final-report.md).
El inventario acumulado de toda la fase está en
[`phase-6-final-report.md`](docs/microfield/phase-6-final-report.md).

F6.G7 reabre y corrige ese cierre práctico: la firma local v1 por sí sola no
era una ruta discriminante suficiente. `analyze_discriminating` añade un
perfil global exacto de componentes débiles/SCC, labels, relaciones, grados y
multiplicidades; ante alta regularidad incorpora triángulos y `K4` bajo un
presupuesto invariante. Así `C6` y `C3 ⊔ C3` se separan linealmente y
Shrikhande/torres 4×4 se separan por `K4`, sin llamar “isomorfo” a una igualdad.
La búsqueda exacta descompone ahora componentes. El diseño, corpus externo y
mediciones están en
[`phase-6-g7-final-report.md`](docs/microfield/phase-6-g7-final-report.md).

F6.V1–V6 están implementadas mediante un laboratorio reproducible no
publicable. La campaña mantiene 145.636 ecuaciones metamórficas, 63.232 pares de
reconciliación recuperados, las 12.346 clases simples de orden 8 y oráculos
adversariales CFI/SRG/ciclos. Los resultados confirman que SHA híbrido o un
segundo campo no resuelven por sí solos la regularidad, mientras que motivos
adaptativos reducen los grafos ambiguos de 454 a 46 y el exacto separa los
contraejemplos comprobados. Véase el
[`informe F6.V`](docs/microfield/phase-6-validation-final-report.md). Fase 7 y
publicación continúan bloqueadas hasta completar la evidencia multi-CPU y los
baselines de dominio, no por falta de harness.

F6.G8 y el baseline G9 sustituyen la autoridad exacta histórica por
`Microcanon`: un núcleo que depende únicamente de `IncidenceGraph` y
`GraphSchemaId`. El encoding `MFC2` posee parser estricto, key de índice,
mappings inversos y verificación completa; `GraphComparison` solo responde
`Isomorphic` junto con un mapping revalidado y falla como `Inconclusive` si el
presupuesto no completa el árbol. El adapter `canonicalize_exact` ya produce
los mismos bytes entre F251, GF(2^256), encoders, lanes y perfiles. El gate
exhaustivo recorrió los 32.768 grafos simples etiquetados de orden seis y
reprodujo exactamente las 156 clases del oráculo independiente. G10 incorpora
ya el motor compacto predeterminado: arena plana, ranks enteros, workspace,
budgets de bytes/profundidad/tiempo, automorfismos verificados y poda por
órbitas/prefijo. Mantiene G9 como referencia diferencial y reproduce sus bytes;
en C32 reduce 97 nodos a 7. La expansión por loops/Green inspirada en `Theta`
permanece en G11. Véanse el
[`informe G8/G9`](docs/microfield/phase-6-g8-g9-implementation-report.md) y el
[`cierre G10`](docs/microfield/phase-6-g10-final-report.md).

## Comandos

```text
cargo test -p microfield --features generator --all-targets
cargo test -p microfield --all-features --doc
cargo clippy -p microfield --all-features --all-targets -- -D warnings
cargo check -p microfield --no-default-features --features portable,builtin-fields
cargo check -p microfield --no-default-features --features portable,prime-fields
cargo check --manifest-path crates/microfield/test-fixtures/external-consumer/Cargo.toml --no-default-features --lib
cargo test --manifest-path crates/microfield/test-fixtures/external-consumer/Cargo.toml --lib
bash crates/microfield/tools/audit_aarch64_pmull.sh
bash crates/microfield/tools/audit_x86_vpclmul.sh
bash crates/microfield/tools/audit_x86_prime.sh
bash crates/microfield/tools/audit_calibration.sh
bash crates/microfield/tools/audit_unsafe_scope.sh
cargo test -p microfield --all-features --test packed_batch --test packed_views
cargo test -p homomorphic-hash-rs --lib
cargo test -p homomorphic-hash-rs --test microfield_compat
cargo test -p homomorphic-hash-rs --all-features --test structural_signatures
cargo test -p homomorphic-hash-rs --all-features --test fast_graph
cargo test -p homomorphic-hash-rs --all-features --test graph_canonical
cargo test -p homomorphic-hash-rs --all-features --test microcanon
cargo test -p homomorphic-hash-rs --release --test graph_canonical microcanon_matches_every_simple_graph_isomorphism_class_at_six_vertices -- --ignored --exact
cargo test -p microfield-validation-lab --all-targets
cargo run --release -p microfield-validation-lab -- semantic --manifest validation/f6/manifest.json --out validation/f6/results/semantic-v1.json
cargo run --release -p microfield-validation-lab -- performance --manifest validation/f6/manifest.json --out /tmp/f6-performance.json
python3 tools/fetch_graph_corpus.py
cargo test -p homomorphic-hash-rs --test external_graph_corpus -- --ignored
conda run -n laboratorio_np sage tools/sage/verify_graph_degeneracy.sage
cargo bench -p homomorphic-hash-rs --bench fast_graph
```

```text
cargo run -p microfield --features generator --bin microfield-gen -- \
  validate crates/microfield/fields/gf2_256_hh_v1.toml
cargo run -p microfield --features generator --bin microfield-gen -- verify-primes --json
```

```rust
use microfield::{CanonicalEncoding, Gf2_256HhV1, Invert};

let value = Gf2_256HhV1::from_canonical(&[1; 32])?;
let inverse = value.invert().expect("el valor no es cero");
let mut one = [0; 32];
one[0] = 1;
assert_eq!((value * inverse).to_canonical(), one);
# Ok::<(), microfield::DecodeError>(())
```

La especificación revisada se encuentra en `planificacion.md` y la
documentación mantenida en `docs/microfield/`. El diagnóstico vigente y el
orden del siguiente hito están en
`docs/microfield/current-status-and-next.md`. El resultado completo de la Fase
1 se documenta en `docs/microfield/phase-1-final-report.md`.

El cierre, las garantías y las limitaciones de la Fase 2 están en
`docs/microfield/phase-2-final-report.md`.

El plan ejecutado de Fase 2 está en `docs/microfield/phase-2-plan.md`.

El cierre de Fase 3 está en `docs/microfield/phase-3-final-report.md`, el de
Fase 4 en `docs/microfield/phase-4-final-report.md` y la ampliación SIMD en
`docs/microfield/phase-4-6-report.md`. La planificación y el cierre del storage
SIMD persistente están en `docs/microfield/phase-4-7-plan.md` y
`docs/microfield/phase-4-7-final-report.md`. El roadmap
corregido incluye la rehabilitación del legado y canonización de grafos dentro
de Fase 6 en `docs/microfield/phases-3-7-roadmap.md`. La auditoría y el cierre
previo a canonización están en `docs/microfield/phase-6-legacy-audit.md` y
`docs/microfield/phase-6-pre-canon-final-report.md`. El contrato y la primera
vertical del motor rápido se documentan en
`docs/microfield/phase-6-fast-graph.md`.
