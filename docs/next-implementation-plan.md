# Plan de implementación siguiente

## Contexto

Este plan parte del estado real actual de `envlt`, no del estado ideal del PDD original.

Hoy ya existe:

- Fase 1 completada y extendida
- Fase 2 funcional con `export` / `import`
- parte importante de Fase 3 funcional

La decisión estratégica actual es:

- posponer Fase 4 de sync con nube
- priorizar cierre funcional del CLI
- dejar el proyecto listo para distribución e instalación con Homebrew
- evaluar Keychain como mejora útil, pero no como bloqueo para release inicial

## Decisión de priorización

### Lo que se pospone

La Fase 4 puede esperar:

- `cloud link`
- `cloud status`
- `sync`
- detección y resolución de conflictos entre vaults

Razón:

- el producto ya tiene una vía razonable de portabilidad con `export` / `import`
- un usuario puede hacer backup manual de `vault.age`
- sync añade mucha complejidad de producto, edge cases, soporte y recovery
- no es requisito para tener una primera versión instalable y útil

### Lo que sí conviene empujar ahora

Orden recomendado:

1. cerrar Fase 3 de forma sólida
2. endurecer seguridad y UX del CLI
3. preparar distribución pública
4. decidir si Keychain entra antes o después del primer release instalable
5. dejar `envlt-bar` para después de tener una CLI estable

## Recomendación sobre Keychain

Sí vale la pena, pero no debe bloquear el primer release instalable.

Beneficio real:

- reduce fricción diaria
- mejora mucho la UX en macOS
- evita pedir passphrase en cada operación

Riesgo real:

- introduce superficie específica de plataforma
- complica tests, CI y soporte
- puede retrasar la salida pública si se mete demasiado pronto

Recomendación concreta:

- release inicial instalable sin Keychain
- siguiente iteración enfocada en Keychain para macOS
- diseñar desde ahora la interfaz de auth para no rehacer el CLI después

## Objetivo del siguiente tramo

Entregar una versión CLI:

- estable
- bien documentada
- fácil de instalar
- razonablemente segura
- usable sin fricción innecesaria

## Roadmap recomendado desde hoy

### Etapa A. Cerrar Fase 3

Objetivo:

dejar resuelta la parte de variables, generación y diff con un nivel suficientemente maduro para release.

#### A1. Completar `gen`

Trabajo:

- terminar el modo interactivo de `envlt gen`
- permitir selección guiada de preset
- permitir flujo opcional de guardado directo al vault
- ampliar tipos/presets de generación realmente útiles
- mejorar ergonomía de `--silent`, `--set` y defaults

Buenas prácticas 2026:

- prompts simples y predecibles
- defaults seguros
- separar claramente lógica de generación del flujo interactivo
- no acoplar UI del CLI con la lógica del core

Definition of Done:

- `envlt gen` funciona bien tanto en modo flag-driven como interactivo
- tests de integración cubren el flujo principal
- `usage.md` refleja todos los presets disponibles

#### A2. Madurar `VarType`

Trabajo:

- mejorar exposición de tipos en CLI
- definir mejor el rol de `Plain`
- evitar ambigüedades entre inferencia y override manual
- preparar base para metadatos futuros sin romper el formato actual

Buenas prácticas 2026:

- compatibilidad hacia atrás del formato serializado
- defaults estables en `serde`
- cambios evolutivos del modelo, no refactors destructivos

Definition of Done:

- comportamiento de tipos explicado y consistente
- sin sorpresas entre `add`, `set`, `vars`, `gen`

#### A3. Mejorar `diff`

Trabajo:

- enriquecer salidas
- agregar before/after solo cuando sea seguro mostrarlo
- diferenciar mejor cambios de presencia vs cambios de valor
- mejorar legibilidad de resultados

Buenas prácticas 2026:

- salidas seguras por defecto
- diseño “safe reveal”: nunca mostrar secretos completos salvo decisión explícita futura
- formatos consistentes y fáciles de parsear

Definition of Done:

- `diff` sirve como herramienta de diagnóstico real
- secretos siguen protegidos en la salida

### Etapa B. Hardening de producto antes de release

Objetivo:

reducir riesgo operativo y cerrar los huecos que más se sienten en una primera versión pública.

#### B1. Seguridad práctica

Trabajo:

- evaluar integración de `secrecy` y/o `zeroize`
- revisar dónde viven secretos en memoria y logs
- asegurar que errores no expongan valores
- revisar stdout en `set`, `gen --set`, `vars`, `diff`

Buenas prácticas 2026:

- modelado explícito de secretos
- minimizar exposición accidental en `Debug` y `Display`
- pruebas de regresión para no imprimir secretos

Definition of Done:

- política clara de manejo de secretos
- no hay caminos obvios que impriman secretos por error

#### B2. Validación y recovery

Trabajo:

- endurecer validación del vault y bundles
- detectar corrupción y formatos inconsistentes mejor
- mejorar mensajes de error accionables
- documentar recuperación manual básica

Definition of Done:

- errores distinguen corrupción, passphrase inválida y formato incompatible
- el usuario entiende qué hacer después de un fallo

#### B3. `doctor`

Trabajo:

- crear `envlt doctor`
- revisar presencia de vault
- revisar permisos/rutas
- revisar legibilidad del vault
- revisar compatibilidad básica de formato
- revisar `.envlt-link` cuando aplique

Por qué sí vale la pena antes de release:

- reduce muchísimo soporte manual
- ayuda onboarding
- ayuda troubleshooting para Homebrew users

Definition of Done:

- `envlt doctor` cubre problemas comunes reales
- salida clara en éxito y fallo

### Etapa C. Release engineering

Objetivo:

dejar el proyecto publicable e instalable con calidad mínima profesional.

#### C1. Documentación de release

Trabajo:

- actualizar `README.md` al estado real
- agregar quickstart realista
- documentar instalación desde source
- documentar comando por comando a alto nivel
- agregar documento de seguridad
- agregar changelog inicial

Archivos sugeridos:

- `README.md`
- `docs/security.md`
- `CHANGELOG.md`
- `CONTRIBUTING.md`

Definition of Done:

- alguien externo puede instalar, probar y entender el producto sin leer todo el PDD

#### C2. CI y calidad

Trabajo:

- GitHub Actions para `fmt`, `clippy`, `test`
- matriz mínima para macOS y Linux
- chequeos reproducibles
- política de versiones y tagging

Buenas prácticas 2026:

- CI rápida y determinista
- pasos separados para diagnóstico claro
- caché prudente de Cargo

Definition of Done:

- cada PR valida el workspace
- cada tag puede producir artefactos reproducibles

#### C3. Empaquetado e instalación

Trabajo:

- binarios release reproducibles
- `cargo install --path .` documentado
- fórmula/tap de Homebrew
- estrategia de naming/versionado

Prioridad:

- primero binario release funcional
- luego Homebrew tap
- crates.io puede esperar si el foco principal es binario CLI instalable

Definition of Done:

- instalar `envlt` con Homebrew es un flujo real y documentado

### Etapa D. Diseño previo de auth y Keychain

Objetivo:

preparar el terreno sin obligar a implementarlo ya.

#### D1. Diseñar la interfaz de auth

Trabajo:

- definir cómo se resuelve la passphrase
- establecer orden de precedencia
  `ENVLT_PASSPHRASE` -> prompt -> Keychain futuro
- aislar la lógica en una capa pequeña
- evitar dispersar llamadas a prompt por todo el CLI

Definition of Done:

- existe una estrategia clara de resolución de credenciales
- meter Keychain después no obliga a romper comandos

#### D2. Decidir entrada de Keychain

Ruta recomendada:

- si el release base sale limpio, Keychain entra justo después
- si el release todavía necesita estabilización, Keychain espera a v1.1

## Orden exacto recomendado

1. completar `gen`
2. cerrar mejoras prioritarias de `diff`
3. endurecer seguridad de salidas y errores
4. implementar `doctor`
5. actualizar documentación principal
6. montar CI y artefactos de release
7. crear instalación con Homebrew
8. diseñar y luego evaluar Keychain
9. dejar Fase 4 y `envlt-bar` para después

## Qué no conviene hacer ahora

- no abrir sync con nube antes de tener release estable
- no empezar `envlt-bar` antes de cerrar UX y contratos del CLI
- no meter demasiadas features nuevas de modelo si complican compatibilidad
- no volver a tocar el formato del vault sin necesidad fuerte

## Riesgos principales

### Riesgo 1. Crecer demasiado antes de publicar

Mitigación:

- optimizar por release usable, no por completitud del PDD

### Riesgo 2. UX inconsistente entre comandos

Mitigación:

- unificar prompts, mensajes y política de secretos antes de release

### Riesgo 3. Keychain consume demasiado tiempo

Mitigación:

- diseñarlo primero, implementarlo solo si no retrasa Homebrew

### Riesgo 4. README y docs quedan atrasados

Mitigación:

- cada feature cerrada actualiza `README.md`, `docs/usage.md` y `docs/status.md`

## Definition of Done del release instalable

Se puede considerar listo para primer release instalable cuando:

- el CLI cubre bien el flujo principal de uso local
- `gen`, `diff`, `vars`, `export`, `import`, `use` y `run` están suficientemente pulidos
- existe `doctor`
- la documentación principal está alineada con el estado real
- CI valida fmt, clippy y tests
- hay artefactos release o tap de Homebrew funcionando
- el producto puede instalarse y usarse sin depender del repo abierto en el editor

## Después de ese release

Prioridad recomendada post-release:

1. Keychain
2. hardening extra de bundles y recovery
3. sync con nube
4. `envlt-bar`

## Resumen ejecutivo

La mejor ruta hoy no es seguir el roadmap original de forma lineal.

La mejor ruta hoy es:

- terminar de pulir el CLI
- hacerlo instalable
- cerrar soporte básico y diagnósticos
- después sumar comodidad de plataforma con Keychain
- y solo luego entrar a sync o GUI
