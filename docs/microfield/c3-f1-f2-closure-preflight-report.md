# C3 F1/F2: cierre de referencias, backends y patrones

Fecha: 2026-08-11. Estado: **ocho huecos F1/F2 implementados; 366 celdas
publicables definidas y 85 preflights ejecutados; réplica multihost
`Controlled` pendiente**.

Los campos finitos y los resúmenes construidos sobre ellos son algebraicos y
**no criptográficos**. Esta campaña comprueba corrección y caracteriza rutas;
no estudia resistencia criptográfica.

## Matriz añadida

F1 incorpora una referencia binaria independiente bit a bit para las tres
presentaciones mantenidas, ejecución portable, selección detectada y forzada,
PCLMUL/VPCLMUL explícitos en x86 y seis patrones: cero, uno, denso,
alternante, monomio alto y pseudoaleatorio.

F2 incorpora portable, detectado/forzado, AVX2/BMI2 explícitos, los patrones
cero, uno, `p-1`, denso, alternante y pseudoaleatorio, y reducción modular
contrastada con `BigUint` independiente para entradas de 1 a 256 bytes. El
plan genera 366 celdas; sumado a los shards anteriores, C3 alcanza **3.609
celdas**.

## Resultado de corrección y precisión

Los 85 preflights iniciales usaron 170 procesos y 850 observaciones. Todos los
checksums fueron estables y todas las comparaciones algebraicas pasaron:

- 79/85 celdas fueron precisas en la primera ejecución;
- seis variantes de alta sensibilidad temporal quedaron inconclusas por IC,
  no por divergencia semántica;
- tres calibraciones añadieron 36 procesos y 480 observaciones; cada variante
  inconclusa obtuvo después al menos una ejecución precisa;
- la semianchura relativa mediana inicial fue 1,03 % y la máxima 29,58 %;
- 79/85 acciones no asignaron; las seis restantes son las referencias bit a
  bit y `BigUint`, donde la asignación forma parte deliberada del baseline.

La campaña total de cierre conserva tanto los primeros inconclusos como las
calibraciones: 206 procesos y 1.330 observaciones. No se reemplazó ni ocultó
evidencia desfavorable.

## Backends pareados en este host

El host fue un Intel i7-13700HX con PCLMUL, VPCLMUL, AVX2 y BMI2. En Smoke,
los ratios forzado/portable observados fueron:

Los entornos capturados identifican `ffff6c7` como commit base; las operaciones,
manifests y resultados F1/F2 añadidos sobre ese árbol quedan fijados juntos en
el commit de cierre de esta fase.

| Campo/backend | Escala | Ratio | Lectura diagnóstica |
|---|---:|---:|---|
| GF(2^128) PCLMUL | 8 | 0,072 | 13,9× más rápido |
| GF(2^128) VPCLMUL | 8 | 0,133 | 7,5× más rápido |
| GF(2^256)-HH PCLMUL | 8 | 0,068 | 14,8× más rápido |
| GF(2^256)-HH VPCLMUL | 8 | 0,052 | 19,4× más rápido |
| GF(2^256)-Alt PCLMUL | 8 | 0,063 | 15,8× más rápido |
| GF(2^256)-Alt VPCLMUL | 64 | 0,059 | 16,8× más rápido |
| Fp251 AVX2 | 8 | 1,363 | 36,3 % más lento |
| Goldilocks AVX2 | 8 | 0,960 | 4,0 % más rápido |
| Fp256 BMI2 | 64 | 1,652 | 65,2 % más lento |

Goldilocks se recalibró de forma pareada con ocho procesos y 240
observaciones: portable 83,066 ns/acción, AVX2 79,799 ns/acción, ratio IC95
`[0,9591, 0,9621]`. Esta acción incluye batch y checksum; por eso no sustituye
la calibración específica de kernels existente.

Los candidatos primos desfavorables respaldan que Fp251 AVX2 tenga umbral
automático 64 y que Fp256 BMI2 siga `explicit-only`. Los resultados binarios
VPCLMUL son prometedores pero no cambian la tabla de selección: son Smoke de
un solo host, una escala por comparación y sólo dos procesos. Se requiere una
curva pareada `Controlled` en al menos dos familias x86 antes de promoverlos.

## Conclusión

F1 y F2 ya no tienen huecos de implementación en el inventario. Las rutas
portable y forzadas producen exactamente los mismos elementos, las referencias
independientes concuerdan y los patrones de borde no rompen round-trip,
inversión o reducción.

Esto cierra el preflight local, no la evidencia de publicación. Quedan la
ejecución completa de las 3.609 celdas, Intel/AMD/AArch64 controlados y el soak
PostgreSQL D2. Los manifests, raw, agregados y calibraciones están bajo
`validation/benchmarks/manifests/c3-f1-f2-closure` y
`validation/benchmarks/runs/c3-f*-*`.
