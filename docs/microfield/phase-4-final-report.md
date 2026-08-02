# Informe final de Fase 4 — campos primos

Fecha: 2 de agosto de 2026.

## Resultado

Microfield dispone ahora de tres campos primos mantenidos, certificados y
utilizables tanto por la API escalar como por `Engine<F>`. La fase conserva
dispatch escalar estático, elementos sin heap ni flags de backend y selección
batch previa al bucle. Los algoritmos de Fase 3 se reutilizan sin una variante
especial para característica prima.

## API y modelo

`PrimeField` añade exclusivamente módulo, capacidad, frontera de entero
canónico y reducción explícita de bytes. `SquareRootField` permanece como una
capacidad separada. `Characteristic` conserva el decimal exacto incluso cuando
el primo no cabe en `u64`.

Internamente, `PrimeFieldSpec` entrega constantes estáticas a algoritmos
monomorfizados y `PrimeWideProduct` separa producto ancho de reducción. Ninguno
es API de usuario. `PrimeRepresentationKind` distingue residuo canónico de
Montgomery sin hacer observable la representación real.

Los tamaños/layouts comprobados son:

| Tipo | Tamaño | Alineamiento | Encoding |
|---|---:|---:|---|
| `Fp251V1` | 1 byte | 1 | 1 byte LE canónico |
| `FpGoldilocks64V1` | 8 bytes | 8 | 8 bytes LE canónicos |
| `Fp256GenericV1` | 32 bytes | 8 | 32 bytes LE canónicos |

No existen constructores desde limbs, conversiones implícitas entre campos ni
serialización del dominio Montgomery.

## Aritmética portable

`Fp251V1` usa un producto ancho de 16 bits y reducción nativa. Su dominio
completo se prueba exhaustivamente.

Goldilocks implementa cuatro folds acotados de
(2^{64}\equiv2^{32}-1\pmod p), una corrección final y una implementación
Barrett independiente basada en el high-half de un producto de 128 bits. Las
dos rutas se contrastan con el resto nativo en fronteras y muestras sembradas.
Barrett es la estrategia seleccionada porque ganó la medición final.

El primo genérico almacena (aR\bmod p) en cuatro limbs privados. Producto
escolar, carries y REDC CIOS son de anchura fija; `to_canonical` ejecuta la
conversión fuera de Montgomery. Se validan todos los bits de entrada del REDC,
round-trips y resultados Sage.

Cada inversa usa una planificación fija y verificada contra su propio (p-2).
Para 251 y el primo genérico, ambos congruentes con 3 módulo 4, `sqrt` usa la
potencia fija correspondiente y devuelve la raíz con menor encoding.

## Identidad, certificados y reproducibilidad

Identidades semánticas:

| Campo | `FieldId` |
|---|---|
| `Fp251V1` | `aef78c79e5e5e929ee046a199df8eab46633a4ea7cabf66480fe2d7909d678da` |
| `FpGoldilocks64V1` | `db27c832ee2b9e87ae66e00657a20cf705132730f5ac43e3f7031f9bb1e272ac` |
| `Fp256GenericV1` | `60cbdb42c3d6efbc7158144f6a42d015a708ca15ae47e5156204660f97681e8e` |

El nombre y la estrategia de reducción no entran en `FieldId`; representación,
reducción, inversión y ABI sí quedan ligados por `ArtifactId`. Cada directorio
incluye descriptor, certificado, metadata y `bundle.json`. El bundle autentica
los otros payloads mediante ruta, longitud y SHA-256.

El verificador Rust reproduce división completa o Pocklington sin confiar en
Sage. `microfield-gen verify-primes --json` comprueba los tres certificados.
SageMath bajo `laboratorio_np` reproduce además el primo de 256 bits desde la
semilla, confirma primalidad y genera
`reference-vectors/prime-fields-v1.json` byte a byte.

## Batch, ISA y selección

El ABI neutral de `KernelSet` no cambió. Los campos primos adjuntan
`PrimeKernelMetadata` y usan las mismas validaciones transaccionales, packed
batches y algoritmos derivados que los campos binarios.

El adapter x86 nuevo contiene dos estrategias:

- AVX2 para `Fp251V1`: widen byte→u16, producto/suma por lanes, Barrett de
  16 bits, compactación canónica y tails escalares;
- BMI2 radix-64: factory estático monomorfizado sobre
  `VerifiedPrimeMontgomery64Field<N, 2N>` y estrategia opaca
  `VerifiedPrimeIsaStrategy`; producto multi-limb con `MULX`, suma y REDC
  branchless propiedad del runtime. Cada campo aporta únicamente constantes y
  conversiones privadas. La prueba
  del constructor ancho cubre 1, 2, 3 y 4 limbs; `Fp256GenericV1` es su primera
  instancia mantenida y reutiliza REDC portable.

AVX2 resultó rentable desde 64 elementos en el i7-13700HX medido y se publica
con `minimum_batch = 64`. A 4096 elementos se observó aproximadamente un 10 %
de mejora. BMI2 mantiene `automatic_selection = false`: la reescritura fija
cerró gran parte de la distancia, pero el artefacto finalmente auditado aún
quedó ligeramente por detrás en la región medida.
Esto materializa la regla de que disponibilidad ISA no equivale a preferencia.

La extensión F4.6-SIMD añadió después una ruta AVX2 específica para
`FpGoldilocks64V1`: cuatro residuos `u64` por tesela, reconstrucción vectorial
del producto de 128 bits y cuatro folds Solinas branchless. En el mismo
i7-13700HX mejoró producto y square aproximadamente 25–33 % entre 4 y 16.384
elementos sin degradar suma en la frontera, por lo que se selecciona
automáticamente desde cuatro. También se incorporaron bridges AVX2 opacos para
primos externos canónicos `u8`/`u16`; permanecen explícitos porque compatibilidad
no aporta calibración y la conversión de layout puede dominar el coste.

VPCLMUL procesa ahora dos pares independientes por iteración. Mejoró los casos
GF(2^256) largos medidos, pero no supera a PCLMUL de forma suficientemente
uniforme para entrar en `Auto`. El detalle, la batería diferencial y los
límites están en [`phase-4-6-report.md`](phase-4-6-report.md).

La condición general no es que el módulo tenga exactamente 64, 128 o 192 bits,
sino que su representación Montgomery use `N` limbs radix 64. Esto permite que
la generación de Fase 5 produzca candidatos BMI2 en crates consumidores sin
exponer intrinsics ni punteros y sin duplicar algoritmos. La
promoción continúa siendo por campo, backend y región medida: una forma de
almacenamiento compatible no garantiza una optimización.

La frontera externa se prueba con un `Fp17` definido en un integration test:
el crate consumidor implementa el contrato const-genérico, obtiene la
estrategia opaca, fuerza BMI2 y coincide con portable en los 289 pares posibles
de suma y producto. Esto
valida el puente que consumirá el codegen primo de Fase 5, sin adelantar todavía
el manifiesto, assurance ni lockfile de esa fase.

La revisión final reescribió también el scheduling completo. El producto ya no
propaga carry con `while`: cada fila ejecuta `N` multiplicaciones y deposita su
carry final. REDC ejecuta siempre `N × N` productos, barre todos los limbs de
propagación y calcula siempre la resta final. Una máscara aislada tras una
barrera de optimización evita que LLVM reconstruya una rama; la auditoría del ELF confirma operaciones bitwise y
ningún salto condicional dentro del producto+REDC. BMI2 se declara por ello
`Fixed`, y `FixedSchedule` lo acepta cuando se fuerza. Esto no amplía la promesa
a tiempo constante integral ni a otros canales laterales.

La remedición final obtuvo, como medianas orientativas: 19,426 ns portable
frente a 19,861 ns BMI2 para 1 elemento; 1,0690 µs frente a 1,1484 µs para 64;
y 284,86 µs frente a 298,13 µs para 16.384. BMI2 queda así aproximadamente
entre 2 % y 7 % por detrás en estos puntos; no existe todavía una región de
promoción estable.

La auditoría de ensamblado exige `vpmullw`, `vpmulhuw`, `vpackuswb`,
`vzeroupper`, 16 operaciones `mulx` por producto de cuatro limbs y ausencia de
saltos condicionales dentro del producto+REDC. También rechaza división,
asignador o dispatch indirecto dentro del adapter.

## Calidad y seguridad

La suite completa de Microfield pasa con todas las features. La cobertura nueva
incluye aritmética exhaustiva/diferencial, corpus Sage tipado, certificados,
identidades, bundles, rangos, planes, tamaños normativos, tails, in-place y
cero asignaciones.

La matriz CI incorpora:

- `prime-fields` solo y junto a `portable`, con y sin `alloc`;
- cross-check AArch64 `no_std`;
- MSRV 1.89;
- Miri sobre campos primos portables;
- ASan sobre AVX2 Fp251/Goldilocks, bridges externos `u8`/`u16`, BMI2 y
  aritmética prima;
- Clippy/rustdoc sin warnings;
- auditoría del inventario `unsafe` y del ensamblado x86.

La raíz conserva `#![deny(unsafe_code)]`. `backend/x86_prime.rs` es la quinta
frontera permitida, con wrappers seguros, precondiciones `SAFETY`, hash fijado y
tests diferenciales. No se añadió `unsafe` al dominio, al encoding, a los
planes ni al portable.

## Rendimiento publicado

En la medición local final, Goldilocks Solinas se situó en 1,8774–1,8804 ns y
Barrett en 1,4569–1,4625 ns. Aun usando los extremos conservadores, Barrett
mejora más de un 22 % y por ello es la ruta productiva. La comparación no usa
una división en el kernel Barrett: calcula el high-half contra la recíproca
generada y aplica como máximo una corrección.

Las cifras son evidencia de selección para la CPU/compilador descritos, no una
garantía universal. El harness separa aritmética escalar, reducción,
conversión Montgomery, portable, fachada y backend forzado.

## Límites explícitos

- La factory externa de campos primos se desarrollará en Fase 5; la factory v1
  binaria no acepta variantes parcialmente implementadas.
- BMI2 no se promociona automáticamente mientras no gane en una región
  publicada.
- No se anuncia backend primo AArch64 ni IFMA sin hardware y medición real.
- Los bridges SIMD externos son forzables y no automáticos hasta disponer de
  calibración versionada por campo; una futura representación runtime o packed
  persistente deberá eliminar su conversión de tesela para aspirar a zero-copy.
- No se implementa Tonelli–Shanks general; `SquareRootField` solo aparece en
  los campos con shortcut demostrado.
- Los fallos históricos de ejemplos/benchmarks del paquete legado continúan
  fuera de esta fase y se abordarán en Fase 6.

Con estos límites, la definición de terminado de Fase 4 queda satisfecha. Tras
el cierre se completaron F4.6-SIMD y
[`F4.7-PACKED-SIMD`](phase-4-7-final-report.md). Esta última aporta storage
persistente `u8`/`u16`/`u32` previo a Fase 5. La generación y los contextos
externos con assurance, lock/bundle y caché segura siguen perteneciendo a
Fase 5.
