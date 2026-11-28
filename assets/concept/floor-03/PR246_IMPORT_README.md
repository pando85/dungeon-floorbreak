# PR #246 — THE OVER CITY · Paquete de entrega

**Repositorio:** [pando85/dungeon-floorbreak](https://github.com/pando85/dungeon-floorbreak)  
**Pull request:** [#246](https://github.com/pando85/dungeon-floorbreak/pull/246)  
**Revisión fijada:** `3e0073df6f6e4fc2003245496d6952f64815d822`  
**Fecha de preparación:** 2026-11-22  
**Estado:** PR de **diseño en borrador**. No se incluyen escenas jugables ni sprites de producción.

## Qué contiene este ZIP

Los **17 archivos originales de arte conceptual del piso 3** en sus rutas `assets/concept/floor-03/`, sin ZIP anidados: tres ilustraciones de ambiente, tres storyboards HTML para móvil y materiales de procedencia. Además encontrarás este README, un **índice de los diez documentos de la PR**, el manifiesto de revisión y un script opcional para recuperar de GitHub **los diez documentos completos de esa revisión**.

Los documentos de diseño ya están guardados en la PR #246. **No se duplican en este ZIP como copias estáticas**: el índice los enlaza al commit preciso, y el script permite descargarlos si necesitas un paquete autónomo con ambos tipos de archivos. Esto evita que una copia adjunta se confunda con la versión más reciente del repositorio.

## Importación rápida

Desde la raíz de tu clon del repositorio:

```bash
unzip PR246_The_Over_City_design_handoff.zip
# Opcional: recuperar también los documentos Markdown fijados a esta revisión
python3 assets/concept/floor-03/tools/fetch_pr246_docs_pinned.py
# Revisar antes de incorporarlos al commit
 git status --short
```

El script **no sobrescribe** archivos diferentes sin `--force`, y solo se conecta a las URLs públicas de los documentos en el commit `3e0073df6f6e4fc2003245496d6952f64815d822`.

Para descargar la rama completa directamente desde GitHub (incluye los diez documentos y todo el repositorio, pero no estos conceptos si aún no están subidos):
https://github.com/pando85/dungeon-floorbreak/archive/refs/heads/design/f03-over-city-bible.zip

## Normas de uso

- Son referencias visuales exploratorias, no capturas de Bevy ni sprites finales.
- Preserva los diseños aprobados de Carl, Donut y Mongo. No extraigas personajes nuevos como si fueran diseños canónicos aprobados.
- La ilustración de la carpa es **mood art**: Grimaldi debe rediseñarse como **Pestiferous Vine** y no copiar su aproximación humanoide.
- Todo sprite de producción necesita fuente editable nativa, manifiesto de procedencia, revisión independiente y captura a 640×360 o 640×400.
- Los permisos para arte conceptual no implican derechos de publicación de un juego distribuible.
- Mantén esta rama separada de la PR #239 (piso 2) hasta que se coordinen los cambios de progresión.

Consulta `PR246_DESIGN_DOCS_INDEX.md` para los documentos y `PR246_SOURCE_MANIFEST.json` para el inventario de origen.
