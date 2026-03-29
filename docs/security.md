# Seguridad actual

Este documento resume el modelo de seguridad implementado hoy en `envlt`.

## Qué protege hoy

### Vault local cifrado

- el vault se guarda en `~/.envlt/vault.age`
- el contenido del vault no vive en texto plano en disco
- el acceso depende de la passphrase del vault

### Backup básico

- cuando el vault se sobrescribe, `envlt` crea `vault.age.bak`
- esto ayuda en recuperación básica ante errores operativos

### Bundles separados del vault

- `export` genera bundles `.evlt`
- los bundles usan una passphrase distinta a la del vault
- compartir un bundle no obliga a compartir la passphrase maestra del vault

### Menor exposición en disco

- `envlt run` inyecta variables al proceso hijo sin escribir `.env`
- `envlt use` sí escribe un archivo `.env`, por lo que debe tratarse como artefacto temporal

### Salidas seguras por defecto

- `vars` enmascara valores `Secret`
- `diff` no imprime valores
- `doctor` reporta estado y errores, no secretos

## Qué no resuelve todavía

- no hay integración con Keychain
- no hay zeroization explícita de secretos en memoria
- no hay sync con nube ni resolución de conflictos remotos
- no hay redacción parcial sofisticada de secretos en todas las salidas
- no hay política completa de migraciones y recovery avanzado

## Recomendaciones operativas actuales

- usa una passphrase fuerte para el vault
- evita dejar `.env` materializados más tiempo del necesario
- usa `envlt run` cuando no necesites archivo en disco
- comparte bundles `.evlt` solo por canales razonables y separa siempre la passphrase
- conserva backups del home de `envlt` si el vault es importante para tu flujo

## Próximos endurecimientos previstos

- posible integración con `secrecy` / `zeroize`
- Keychain para macOS
- validación más estricta de bundles y recovery
- mejor política de salidas seguras y diagnósticos
