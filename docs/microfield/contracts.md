# Contratos técnicos v1

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
codegen 2 y cada módulo comprueba su compatibilidad mediante una aserción
`const`; el runtime conserva helpers ABI 1 y acepta el rango 1..=2.
`GeneratedFieldPackage::package_digest` autentica conjuntamente el digest del
bundle de certificados/planes y los bytes exactos del módulo Rust.

El plan portable v2 elige estáticamente producto escolar por bits activos,
cuadrado por expansión de bits, inversión Itoh–Tsujii y una de tres
reducciones: tail bajo alineado, términos dispersos o palabras densas. La
decisión depende solo del descriptor validado, queda serializada en el IR y no
añade ramas de selección al escalar.

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

`BackendId` identifica solicitudes y diagnósticos, no disponibilidad. En H4
solo `Portable` está certificado. Forzar PCLMUL, VPCLMUL o PMULL devuelve
`EngineBuildError::BackendUnavailable`. `FixedSchedule` se rechaza porque el
producto portable actual tiene scheduling dependiente de los operandos.

## Errores y escrituras

- Encoding incorrecto devuelve `DecodeError`; no hace panic.
- Una longitud batch inválida se detecta antes de escribir.
- La salida permanece intacta si la validación falla.
- Invertir cero devuelve `None`.
- `from_canonical` nunca reduce; `from_polynomial_bytes_mod` sí.

## Timing

La Fase 1 no promete tiempo constante. `pow` es variable en bits y longitud.
El contrato prohíbe describir una operación como constante sin auditoría
específica.
