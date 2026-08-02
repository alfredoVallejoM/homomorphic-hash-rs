# Contratos técnicos v1

## Algoritmos derivados de Fase 3

`BatchPlan<F>` autentica operación, revisión, longitud, backend y `FieldId`.
`WorkspaceLayout` declara elementos tipados, words de máscara, alineamiento,
soporte in-place y comportamiento de asignación.

La inversión batch es tolerante a cero: un bit de máscara solo se activa para
un valor invertible; cero produce cero. Cualquier error de shape, máscara,
workspace o backend ocurre antes de modificar salida. El mismo contrato
transaccional rige scans, Horner y `mul_add_into`.

Los coeficientes de Horner están en orden de grado ascendente. Cero
coeficientes es un shape inválido; cero puntos o cero polinomios es un lote
vacío válido. `CoefficientLayout` impide transposiciones implícitas.

El IR v4 de inversión se verifica simbólicamente contra `2^degree-2` antes de
la emisión. Cambiarlo altera `ArtifactId`, no `FieldId`; codegen ABI permanece
en v3.

## Campos mantenidos

| Nombre | Módulo | Bytes |
|---|---|---:|
| `Gf2_128V1` | \(x^{128}+x^7+x^2+x+1\) | 16 |
| `Gf2_256HhV1` | \(x^{256}+x^{10}+x^5+x^2+1\) | 32 |
| `Gf2_256AltV1` | \(x^{256}+x^{16}+x^3+x+1\) | 32 |

Todos usan base polinómica. En el encoding canónico los bytes son
little-endian y el bit `i` representa el coeficiente de `x^i`.

Los tres módulos han superado Rabin mediante el validador independiente del
generador. SageMath 10.7 ha validado sus vectores y la convención de encoding.
Los tres tipos son públicos y ofrecen el mismo conjunto de capacidades. Son
newtypes nominalmente distintos: no existe conversión implícita entre
`Gf2_256HhV1` y `Gf2_256AltV1` pese a compartir cardinal.

## Identidad

`identity_bytes` es el UTF-8 de un JSON minificado, sin salto final y con
este orden fijo:

```json
{"schema":1,"characteristic":"2","degree":256,"basis":{"kind":"polynomial","coefficient_order":"ascending"},"modulus":[256,10,5,2,0],"encoding":{"byte_order":"little","bit_order":"lsb0","bytes":32}}
```

El nombre, `BuildProfile`, claims y constantes de aplicación quedan fuera.

```text
FieldId = SHA-256("microfield:field-id:v1\0" || identity_bytes)
```

El TOML normalizado es un artefacto legible, no la serialización autoritativa
del identificador.

Identidades congeladas:

| Campo | `FieldId` |
|---|---|
| `Gf2_128V1` | `4825b6d5606e34af32722a4a6a96d04a1e21337be0fb734adb9c69f9b9d77d31` |
| `Gf2_256HhV1` | `6b62fea68b968fd4f8c39a4f69b78f714c80858b1d0f667ec5a63d4417b43ca8` |
| `Gf2_256AltV1` | `5c78ea2f9ea1b2d59b88bf32e38ae33be4c2f977f0232c4441f7a16e4c9bb54d` |

`StaticFieldSpec` expone tanto `FieldId` como `ArtifactId`. `ArtifactId` usa
otro dominio SHA-256 e incorpora `FieldId`, versión del generador, versión del
IR, familia target, build normalizado y plan de optimización portable. Cambiar
el nombre del campo no altera ninguna identidad; cambiar build o codegen
conserva `FieldId` y cambia `ArtifactId`.

`ArtifactBundleDigest` usa el dominio
`microfield:artifact-bundle:v1\0` sobre la lista canónica de rutas, longitudes
y SHA-256 de los seis payloads. Cambiar el nombre o cualquier byte del conjunto
sí cambia este digest. `bundle.json` registra la lista y queda fuera de su
propio hash para evitar circularidad.

## Contrato del manifiesto v1

El esquema implementado acepta exclusivamente característica 2, base
polinómica ascendente, encoding `little`/`lsb0`, limbs de 64 bits, producto
`schoolbook` y backend portable. Rechaza recursivamente claves desconocidas.
`karatsuba` no se aceptará hasta que exista implementación medida. Los exponentes del
módulo se ordenan en forma descendente y se comprueban grado, monicidad,
término independiente y ausencia de duplicados antes de ejecutar Rabin.

El documento TOML no puede exceder 64 KiB y el grado v1 no puede superar 4096.
La política del validador puede reducir ese límite, nunca elevarlo.

## Vectores externos v2

Las operaciones son un enum cerrado y cada set debe cubrir `canonical`, suma,
producto ancho, reducción, multiplicación, cuadrado, inversión cero/no-cero,
potencia y `mul_by_x`. Los elementos ocupan el ancho canónico y los valores
anchos exactamente el doble. Se validan bits de padding, hexadecimal minúsculo,
exponentes mínimos, identidad, seed y versión del oráculo.

Límites: 8 MiB de JSON, 4096 casos y 4096 bytes por exponente.

Los tres sets mantenidos fueron producidos por SageMath 10.7. Su regeneración
es idéntica byte a byte y la suite contrasta cada resultado con una
implementación polinómica lenta independiente.

## Producto portable

Para cada par de limbs `a[i]`, `b[j]`, `clmul64` produce `(low, high)`:

```text
wide[i + j]     ^= low
wide[i + j + 1] ^= high
```

`Polynomial128<TAIL>` y `Polynomial256<TAIL>` implementan las estrategias
estáticas internas de representación, producto, reducción y cuadrado. Cada
newtype delega mediante `BinaryFieldImpl`; el compilador monomorfiza el tail y
el ancho sin `dyn Trait`, heap ni punteros almacenados en el elemento.

El producto escolar se comparte para dos y cuatro limbs. La reducción recibe
como parámetro constante el tail generado de cada módulo y realiza folds
word-level; la cota de grado del tail se comprueba en el algoritmo. El cuadrado
expande bits directamente y no llama a `mul(self, self)`.

La inversión ejecuta una cadena fija parametrizada por el grado para
`2^m-2`. La reducción rápida se contrasta con división polinómica lenta y con
Sage. Ninguna operación escalar consulta `Engine`, reserva heap o usa dispatch
dinámico.

## Factory binaria estática

Con `generator`, `BinaryFieldFactory` acepta un manifiesto v1 o un Builder de
parámetros matemáticos. El Builder no elude el esquema: construye una entrada
que vuelve a pasar por el parser estricto y por el mismo pipeline de
normalización, identidad, Rabin, planificación y artefactos.

La salida es fuente Rust previa a compilación, no un campo dinámico. El newtype
generado contiene exactamente `ceil(m / 64)` limbs privados y el encoding
contiene `ceil(m / 8)` bytes. Los bits de padding se rechazan; los bytes
polinómicos arbitrariamente anchos se reducen. La fuente actual usa ABI de
codegen 3, tomado de `CURRENT_CODEGEN_ABI_VERSION`, y cada módulo comprueba su
compatibilidad mediante una aserción `const`; el runtime conserva helpers ABI
1/2 y acepta el rango 1..=3. La matriz autoritativa está versionada en
`abi/runtime-codegen-matrix-v1.csv`.
`GeneratedFieldPackage::package_digest` autentica conjuntamente el digest del
bundle de certificados/planes y los bytes exactos del módulo Rust.

El plan portable v2 elige estáticamente producto escolar por bits activos,
cuadrado por expansión de bits, inversión Itoh–Tsujii y una de tres
reducciones: tail bajo alineado, términos dispersos o palabras densas. La
decisión depende solo del descriptor validado, queda serializada en el IR y no
añade ramas de selección al escalar.

IR v3 añade `VerifiedIsaProfile`, derivado exclusivamente tras validación. El
perfil fija layout, anchuras, backends carry-less compatibles, digest de
reducción y digest propio. Se emite como archivo del bundle y habilita una
estrategia ISA segura sin exponer `KernelSet` ni intrinsics. Todo perfil externo
es `explicit_only`: corrección certificada no autoriza selección automática sin
calibración nativa.

El contrato ABI 3 compila también sin la feature `portable`: álgebra, encoding
y metadata permanecen disponibles en `no_std`, mientras `Engine` y los adapters
ISA no se compilan. Activar batch no cambia layout, identidad ni código escalar.

La publicación crea y sincroniza un archivo de staging antes de renombrarlo. Se
rechazan directorios o targets que sean symlinks o archivos especiales. Los
nombres v1 son `snake_case` ASCII estricto y nunca se interpretan como tokens o
rutas antes de normalizarse.

## Batch portable

`Engine<F>` construye y selecciona un `KernelSet<F>` privado al construirse. Cada
operación valida las longitudes antes de escribir y realiza una única llamada
indirecta por lote. El backend portable recorre los slices sin asignar, hacer
packing, detectar CPU ni paralelizar.

Operaciones v1: `add_into`, `mul_into`, `square_into`, `mul_assign` y
`square_assign`. Los slices vacíos son válidos. Las rutas `*_into` reciben una
salida distinta por contrato de préstamos; las rutas `*_assign` expresan
aliasing intencional.

`BackendId` identifica solicitudes y diagnósticos, no disponibilidad. H2.4
compila PCLMUL en x86-64, H2.5 compila PMULL en AArch64 y H2.7 compila VPCLMUL
en x86-64. ABI 3 registra los adapters ISA compatibles para campos externos.
Un campo ABI 1/2 continúa devolviendo
`BackendUnsupportedByField`. En todos los casos se distingue CPU sin capability
(`BackendUnsupportedByCpu`) de campo sin perfil.

PCLMUL mantenido participa en selección automática con el umbral medido. PMULL,
VPCLMUL y todo perfil externo tienen `automatic_selection = false`: solo un
backend forzado tras detección puede usarlos. En VPCLMUL esta exclusión es una
decisión medida: la ganancia GF(2¹²⁸) local no generaliza y las rutas de 256
bits pierden frente a PCLMUL. `FixedSchedule` también respeta esta regla salvo
que se fuerce el backend. Portable no recibe garantía fija porque su producto
actual depende de los operandos.

`EngineBuilder::build()` usa por defecto `CpuCapabilities::portable_only()` y
nunca hace detección implícita. Con `std`, `EngineBuilder::detect()` captura una
instantánea real una vez. `CpuCapabilities` expone arquitectura y flags para
diagnóstico, pero sus bits no tienen constructor público.

`expected_batch` y `KernelMetadata::minimum_batch` son pistas de selección para
`Auto`. No limitan las longitudes válidas: todo `KernelSet` registrado debe
aceptar cualquier slice, incluido el vacío.

H2.8 congela esos valores en `calibration/selection-table-v1.csv`. El runtime
los materializa como `SelectionCalibration` privadas y constantes: no lee CSV,
no consulta modelos de CPU y no añade branches al lote. Una promoción exige
20 % de mejora conservadora, packing incluido y diversidad de familias. La CI
valida perfiles/decisiones, pero los benchmarks se capturan fuera del gate
ordinario para no confundir ruido del runner con una regresión funcional.

## Batch persistente

`Engine::packing_plan(len)` es la única factory de planes. El plan queda ligado
al backend seleccionado, al `FieldId`, al layout, a la longitud lógica/padded,
al tile y al alineamiento. Sus campos son privados y no se serializa.

H2.6 admite `PackedLayout::Aos`. H2.7 añade `AosLanePairs` exclusivamente para
VPCLMUL: dos elementos AoS forman una tesela, la longitud padded es par y el
inicio se alinea a 32 bytes. El backend realiza el interleave en registros; no
se exponen limbs ni cambia el layout de `F`.

`PackedBatch<F>` requiere `alloc`. `PackedBatchView` y
`PackedBatchViewMut` no lo requieren y toman prestado storage
`MaybeUninit<u8>` mediante `pack_into_storage`. `required_packed_bytes` incluye
el peor slack de alineamiento; longitud cero requiere cero bytes. Todo slot
padded se inicializa con `F::ZERO` antes de exponer una referencia tipada.

Producto y cuadrado packed tienen rutas distintas out-of-place e in-place. Una
operación valida todos los planes y el backend antes de escribir, y después
realiza una llamada al kernel seleccionado sobre la región padded. No asigna,
no hace repacking, no detecta CPU ni selecciona estrategia durante la operación.

Los préstamos expresan aliasing: una vista mutable conserva el préstamo
exclusivo del storage y no puede coexistir de forma segura con otra vista sobre
la misma región. No se exponen bytes, offsets, punteros ni padding.

## Errores y escrituras

- Encoding incorrecto devuelve `DecodeError`; no hace panic.
- Una longitud batch inválida se detecta antes de escribir.
- Un backend o plan packed incompatible se detecta antes de escribir.
- La salida permanece intacta si la validación falla.
- Invertir cero devuelve `None`.
- `from_canonical` nunca reduce; `from_polynomial_bytes_mod` sí.

## Timing

La Fase 1 no promete tiempo constante. `pow` es variable en bits y longitud.
El contrato prohíbe describir una operación como constante sin auditoría
específica.

PCLMUL posee schedule fijo respecto de los valores para `mul` y `square`, pero
esta propiedad de scheduling no equivale por sí sola a una garantía completa
de tiempo constante del sistema.

Los kernels PMULL de los presets también poseen calendario fijo respecto de
los valores. El perfil ABI 3 publica `schedule = fixed` solamente para
`low_tail_fold`; `sparse_term_fold` y `dense_word_fold` publican
`schedule = data_dependent`, pues su reducción inspecciona bits del producto.
`FixedSchedule` respeta esa clasificación incluso ante selección explícita.
