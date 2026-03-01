# miga

> **Gestor de Paquetes de Utilidades para Addons de Bedrock**
>
> Una CLI rápida y sin dependencias que inicializa, compila, empaqueta y gestiona
> add-ons de Minecraft Bedrock Edition — escrita en Rust.

[![Licencia: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](../../LICENSE)

<div style="text-align: center">
<img src="../../assets/miga_banner.png" alt="Miga Banner" height="150">
</div>

---

## Tabla de Contenidos

- [Descripción General](#descripción-general)
- [Instalación](#instalación)
- [Comandos](#comandos)
    - [init](#init)
    - [add](#add)
    - [fetch](#fetch)
    - [run](#run)
    - [build](#build)
    - [remove](#remove)
- [Estructura del proyecto](#estructura-del-proyecto)
- [Variables de entorno](#variables-de-entorno)
- [Contribuir](#contribuir)
- [Licencia](#licencia)

---

## Descripción General

`miga` reemplaza toda una cadena de herramientas Node.js para el desarrollo de add-ons de Bedrock.
Se encarga de:

- **Scaffolding** — crea un proyecto BP + RP completo con soporte TypeScript tipado.
- **Tipos de TypeScript** — descarga archivos `.d.ts` directamente desde el registro de npm sin
  requerir que `npm` o `node` estén instalados.
- **Módulos del registro** — obtiene módulos comunitarios desde el registro de miga y los conecta
  a tu proyecto.
- **Compilación** — transpila y, opcionalmente, minifica TypeScript usando
  [oxc](https://oxc.rs/) (Rust nativo, ~100× más rápido que `tsc`).
- **Empaquetado** — ensambla archivos `.mcpack` y `.mcaddon` listos para distribución.
- **Hot reload** — vigila los archivos fuente y los redespliega en las carpetas de packs de desarrollo
  de Minecraft en cada guardado.

---

## Instalación

### Desde el código fuente

```bash
git clone https://github.com/HormigaDev/miga.git
cd miga
cargo install --path .
```

### Binarios precompilados

Descarga la última versión desde la
[página de Releases](https://github.com/HormigaDev/miga-cli/releases) y coloca el binario
en algún lugar de tu `PATH`.

---

## Comandos

### `init`

Inicializa un nuevo proyecto de add-on de Bedrock de forma interactiva.

```bash
miga init [--namespace <ns>] [--name <n>]
```

**Opciones**

| Bandera       | Descripción                                                        |
| ------------- | ------------------------------------------------------------------ |
| `--namespace` | Prefijo de espacio de nombres usado dentro del add-on (ej. `woc`). |
| `--name`      | Identificador interno del add-on (ej. `ecological-spawns`).        |

Las opciones faltantes se solicitan de forma interactiva. El comando crea un directorio con el
nombre del add-on que contiene un esqueleto completo de BP/RP con soporte TypeScript.

---

### `add`

Agrega un paquete de tipos `@minecraft/*` desde el registro de npm.

```bash
miga add <paquete[@versión]> [<paquete[@versión]> ...]
```

**Ejemplos**

```bash
miga add @minecraft/server@2.4.0
miga add @minecraft/server @minecraft/common
```

Los tipos se descargan en `.miga_modules/` y el paquete queda registrado en
`.miga/miga.json`.

---

### `fetch`

Instala uno o más módulos desde el **registro de miga**.

```bash
miga fetch <módulo> [<módulo> ...]
```

Los módulos se descargan, extraen y registran en `.miga/modules.lock`.
Las dependencias transitivas se resuelven automáticamente.

---

### `run`

Vigila cambios en el código fuente y recarga en caliente el add-on en Minecraft.

```bash
miga run
```

`miga run` compila TypeScript en cada cambio y copia los packs a las rutas
configuradas en `.env` (`BEHAVIOR_PACKS_PATH` / `RESOURCE_PACKS_PATH`).

---

### `build`

Compila y empaqueta el add-on.

```bash
miga build
```

Salidas:

| Archivo                   | Descripción                      |
| ------------------------- | -------------------------------- |
| `dist/<nombre>-bp.mcpack` | Solo el Behavior Pack.           |
| `dist/<nombre>-rp.mcpack` | Solo el Resource Pack.           |
| `dist/<nombre>.mcaddon`   | Archivo combinado (ambos packs). |

---

### `remove`

Elimina un módulo instalado desde el registro.

```bash
miga remove <módulo>
```

Borra los archivos del módulo y lo elimina de `.miga/modules.lock`.

---

## Estructura del proyecto

Después de ejecutar `miga init`, el proyecto tendrá la siguiente estructura:

```
<nombre-del-addon>/
├── behavior/               Behavior Pack
│   ├── manifest.json
│   ├── pack_icon.png       Reemplazar con tu propio ícono
│   ├── LICENSE
│   └── scripts/
│       ├── index.ts        Punto de entrada
│       ├── config/
│       │   └── registry.ts Registro central / espacio de nombres
│       ├── events/
│       │   └── index.ts
│       ├── components/
│       ├── features/
│       └── core/
├── resource/               Resource Pack
│   ├── manifest.json
│   ├── pack_icon.png
│   ├── LICENSE
│   ├── texts/              en_US.lang, es_ES.lang, pt_BR.lang
│   ├── textures/
│   │   ├── blocks/
│   │   ├── items/
│   │   ├── entity/
│   │   └── ui/
│   ├── models/
│   ├── sounds/
│   └── ui/
├── .miga/
│   ├── miga.json           Manifiesto del proyecto (nombre, versión, módulos)
│   └── modules.lock        Archivo de bloqueo de módulos instalados
├── .env                    Rutas de despliegue (no se sube al repositorio)
├── .env.template           Plantilla para compartir con colaboradores
├── .gitignore
├── tsconfig.json
├── LICENSE
└── README.md
```

---

## Variables de entorno

Configura `.env` (copia desde `.env.template`):

```dotenv
# Ruta absoluta a la carpeta development_behavior_packs de Minecraft
BEHAVIOR_PACKS_PATH=

# Ruta absoluta a la carpeta development_resource_packs de Minecraft
RESOURCE_PACKS_PATH=

# true = source maps en línea (solo para depuración)
SOURCE_MAPS=false
```

En Linux las rutas predeterminadas se detectan automáticamente mediante `$HOME`. En Windows apuntan
a `%LOCALAPPDATA%\Packages\Microsoft.MinecraftUWP_*`. Si no se encuentra la ruta,
miga emitirá una advertencia y omitirá el paso de copiado.

---

## Contribuir

Ver [CONTRIBUTING.md](../../CONTRIBUTING.md).

---

## Licencia

`miga` es software libre publicado bajo la
[Licencia Pública General de GNU v3.0](../../LICENSE).
