# Optimizador portable H2.2

## Alcance

H2.2 optimiza el código producido por `BinaryFieldFactory`; no sustituye las
especializaciones mantenidas de `Gf2_128V1`, `Gf2_256HhV1` y
`Gf2_256AltV1`. El objetivo es que un campo externo obtenga una ruta portable
razonable sin escribir matemáticas a mano y que siempre exista un oráculo v1
independiente para detectar divergencias.

## Plan generado

`GeneratedFieldPackage::portable_optimization()` expone una vista inmutable de
la decisión. El mismo plan aparece en `generation-plan.json` y participa en
`ArtifactId`.

| Dimensión | Valores v1 del optimizador |
|---|---|
| Clase de grado | `power_of_two_limb_aligned`, `limb_aligned`, `unaligned` |
| Producto | `set-bit-schoolbook-v1` |
| Cuadrado | `bit-spread-v1` |
| Reducción | `low_tail_fold`, `sparse_term_fold`, `dense_word_fold` |
| Inversión | `itoh-tsujii-binary-v1` |

`low_tail_fold` exige grado múltiplo de 64 y que el mayor exponente no líder
sea como máximo 32. Fuera de ahí se usa fold por términos si el tail cabe en
el presupuesto `max(8, 2 * limbs)`; el resto usa palabras densas. Este umbral
es parte de la versión del IR, no una heurística dependiente de la máquina.

## Evidencia de corrección

- las tres reducciones comparan producto y cuadrado contra el helper v1;
- el fold alineado cubre 64, 128, 256, 512, 1024, 2048 y 4096 bits;
- GF(2⁹) compara exhaustivamente con una implementación `u32` independiente;
- GF(2¹⁰), con módulo denso
  \(x^{10}+x^9+\ldots+x+1\), ejercita exhaustivamente el source code denso;
- GF(2²³³) contrasta producto, cuadrado, inversión, potencia y `mul_by_x` con
  vectores SageMath 10.7;
- el consumidor externo compila en `no_std` y usa el mismo `Engine` batch;
- los artefactos mantenidos se regeneran y verifican byte a byte.

## Medición local

Entorno de la primera medición:

- Intel Core i7-13700HX, 24 CPU lógicas;
- Linux 6.18.7 x86-64;
- `rustc 1.96.0-nightly (1d8897a4e 2026-03-13)`, LLVM 22.1.0;
- Criterion 0.5.1, perfil `bench`, 1 s de warm-up, 30 muestras y 2 s de
  medición.

Comando:

```bash
cargo bench -p microfield --bench portable_codegen_optimizer -- \
  --warm-up-time 1 --measurement-time 2 --sample-size 30 --noplot
```

| Campo/operación | Referencia v1 | Optimizado v2 | Factor central observado |
|---|---:|---:|---:|
| GF(2¹²⁸) producto | 478,39 ns | 88,30 ns | 5,4x |
| GF(2¹²⁸) cuadrado | 274,84 ns | 5,66 ns | 48,6x |
| GF(2²³³) producto | 1,305 µs | 658,98 ns | 2,0x |
| GF(2²³³) cuadrado | 434,17 ns | 271,34 ns | 1,6x |
| GF(2²³³) inversión | 488,33 µs | 174,46 µs | 2,8x |

Estos datos demuestran que las estrategias merecen pasar a la ruta generada en
esta máquina. No son garantías de API ni thresholds portables. Antes de aceptar
Karatsuba, SIMD o un gate porcentual contractual se repetirán procesos aislados,
orden alternado, pinning de CPU y auditoría de ensamblado para reducir sesgo de
frecuencia y orden.

Una emisión `release --emit=asm` del consumidor externo muestra llamadas
directas a las monomorfizaciones del producto ancho y ninguna referencia a
`__rust_alloc`; no aparece dispatch de estrategia en los métodos escalares.
Los símbolos de formato y panic pertenecen a rutas separadas o comprobaciones
de bounds, no a selección matemática runtime.

## Límites

- no se promete tiempo constante;
- `schoolbook` sigue siendo el único producto aceptado por el manifiesto;
- Karatsuba no se activa hasta medir cruces por rango de limbs;
- módulos densos evitan explosión de código, pero no se afirma que su ruta sea
  óptima para todos los grados;
- H2.3 ya añade detección y política sin cambiar esta ruta; H2.4/H2.5 añadirán
  PCLMUL/PMULL sin alterar la semántica portable.
