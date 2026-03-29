# envlt

`envlt` es una CLI local-first escrita en Rust para guardar variables de entorno en un vault cifrado, regenerar `.env` bajo demanda y compartir proyectos mediante bundles `.evlt`.

## Estado actual

Hoy el proyecto ya tiene:

- Fase 1 completada y extendida
- Fase 2 funcional con `export` / `import`
- parte importante de Fase 3 ya implementada

Lo que ya funciona:

- vault local cifrado con `age`
- importación desde `.env` y `.env.example`
- `.envlt-link` para resolver proyecto automáticamente
- `vars`, `diff`, `doctor`, `gen`
- exportación e importación de bundles `.evlt`
- backups básicos del vault

## Instalación local

Mientras preparamos la distribución por Homebrew, hoy puedes usarlo así:

### Desde el repo

```bash
cargo run -p envlt-cli -- --help
```

### Instalación local con Cargo

```bash
cargo install --path crates/envlt-cli
envlt --help
```

Requisitos actuales:

- Rust toolchain compatible con el workspace
- macOS o Linux para el flujo validado hoy

## Flujo rápido

```bash
envlt init
envlt add api-payments
envlt list
envlt vars --project api-payments
envlt set --project api-payments PORT=4000
envlt use --project api-payments
envlt run --project api-payments -- node server.js
envlt export api-payments --out bundle.evlt
envlt import bundle.evlt
envlt doctor
envlt gen --type jwt-secret --set JWT_SECRET --project api-payments --silent
```

Si existe `.envlt-link` en el directorio actual, `vars`, `diff`, `set`, `use`, `run` y parte del flujo de `gen` pueden resolver el proyecto sin `--project`.

## Comandos principales

- `envlt init`: crea el vault cifrado local
- `envlt add`: importa un `.env` o `.env.example`
- `envlt list`: lista proyectos del vault
- `envlt vars`: muestra variables y tipos
- `envlt diff`: compara contra `.env.example` o contra otro proyecto
- `envlt set`: crea o actualiza variables con tipo opcional
- `envlt use`: materializa un `.env`
- `envlt run`: ejecuta un comando con variables inyectadas
- `envlt gen`: genera valores seguros y puede guardarlos en el vault
- `envlt export` / `envlt import`: comparte proyectos vía bundles `.evlt`
- `envlt doctor`: diagnostica vault, backup y `.envlt-link`

## Seguridad hoy

- el vault vive cifrado en `~/.envlt/vault.age`
- el backup básico se guarda como `~/.envlt/vault.age.bak`
- `envlt run` inyecta variables sin escribir `.env` en disco
- los bundles `.evlt` usan passphrase separada del vault
- `vars` enmascara secretos y `diff` no imprime valores sensibles

Detalle adicional en [docs/security.md](/Users/kevinx/Developer/Projects/OBSIDIA/envlt/docs/security.md).

## Limitaciones actuales

- todavía no hay sync con nube
- todavía no hay Keychain
- `gen` no tiene aún todos los presets del PDD
- `diff` todavía no muestra un before/after detallado
- el README describe el producto actual, pero la distribución pública todavía está en preparación

## Documentación

- [Uso actual](/Users/kevinx/Developer/Projects/OBSIDIA/envlt/docs/usage.md)
- [Estado del proyecto](/Users/kevinx/Developer/Projects/OBSIDIA/envlt/docs/status.md)
- [Plan siguiente de implementación](/Users/kevinx/Developer/Projects/OBSIDIA/envlt/docs/next-implementation-plan.md)
- [Plan de Fase 1](/Users/kevinx/Developer/Projects/OBSIDIA/envlt/docs/phase-1-implementation-plan.md)
- [Definición del proyecto](/Users/kevinx/Developer/Projects/OBSIDIA/envlt/docs/project-definition.md)
- [Notas de seguridad](/Users/kevinx/Developer/Projects/OBSIDIA/envlt/docs/security.md)

## Calidad

El workspace se valida actualmente con:

- `cargo fmt --all`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
