# Estado del proyecto

Este documento resume qué ya está implementado, qué sigue inmediatamente y qué queda como mejora futura.

## Estado actual

Fases con avance real en código:

- Fase 1: base funcional completada y extendida
- Fase 2: export/import `.evlt` funcional
- Fase 3: base de `VarType` e inferencia automática iniciada

## Ya implementado

### Core

- vault local cifrado con `age`
- lectura y escritura atómica del vault
- backup básico `vault.age.bak`
- modelo `VaultData`, `Project`, `Variable`
- `VarType` en el modelo
- inferencia automática de `VarType` por nombre
- parser y writer de `.env`
- `add --from-example`
- formato binario `.evlt`
- cifrado de bundles con `scrypt` + `ChaCha20-Poly1305`
- import/export de bundles desde la capa de aplicación

### CLI

- `init`
- `add`
- `list`
- `vars`
- `diff --example`
- `diff <proyecto> <otro-proyecto>`
- `set`
- `set --secret|--config|--plain`
- `use`
- `run`
- `gen`
- `export`
- `import`
- `import --overwrite`

### UX actual

- resolución automática de proyecto vía `.envlt-link`
- passphrase del vault por prompt o `ENVLT_PASSPHRASE`
- passphrase del bundle por prompt o `ENVLT_BUNDLE_PASSPHRASE`
- `gen` con flujo interactivo guiado para elegir tipo y guardar opcionalmente en el vault

### Calidad

- `cargo fmt --all`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- tests de integración para flujos principales del CLI
- tests unitarios del core para bundle e inferencia de tipos

## En progreso inmediato

- seguir madurando la experiencia de `VarType`
- preparar comparación más rica y diagnósticos
- expandir `gen` con más tipos y ergonomía interactiva

## Pendiente cercano

- más tipos de `gen` y modo interactivo completo
- presets más completos para passwords memorables y otros formatos
- diff con before/after más rico cuando sea seguro mostrarlo

## Mejoras futuras

### Seguridad y UX

- ocultado parcial de secretos en salidas
- confirmaciones más ricas en operaciones sensibles
- validaciones más estrictas de bundles corruptos o inconsistentes
- posible integración con `secrecy` / `zeroize`
- salidas de diff más ricas, con mejor formateo y severidades

### Operación

- `doctor`
- mejores diagnósticos y recovery
- estrategia de migraciones de vault más explícita

### Producto

- sync con nube
- Keychain
- GUI `envlt-bar`

## Decisiones intencionales por ahora

- `import` no hace merge automático
- `Plain` existe en el modelo, pero todavía no tiene flujo de usuario dedicado
- la inferencia actual de secretos es por nombre y no por análisis semántico más profundo
