# Inventario estático P0

Código de origen: `251d5c4025749edd4767183ebd3d2baf3bb9baed`.

Incluye imports en funciones y grafo transitivo. No prueba disponibilidad en Windows,
imports calculados ni invocaciones por subprocess. Las acciones son propuestas para fases posteriores.

| Módulo | Acción propuesta | Fase | Imports locales directos | Importado por |
|---|---|---|---|---|
| [descargador.apariencia](../../../descargador/apariencia.py) | CONSERVAR | P0 | 0 | 1 |
| [descargador.carga](../../../descargador/carga.py) | CONSERVAR | P0 | 1 | 3 |
| [descargador.contrato_integral](../../../descargador/contrato_integral.py) | CONSERVAR | P0 | 1 | 2 |
| [descargador.csmp_shared](../../../descargador/csmp_shared.py) | CONSERVAR parser común; comprobar paquete autónomo | P3 | 1 | 1 |
| [descargador.descargador](../../../descargador/descargador.py) | CONSERVAR | P0 | 13 | 0 |
| [descargador.evidencias](../../../descargador/evidencias.py) | CONSERVAR | P0 | 1 | 1 |
| [descargador.excel_presentacion](../../../descargador/excel_presentacion.py) | CONSERVAR | P0 | 0 | 1 |
| [descargador.filtros_respuesta](../../../descargador/filtros_respuesta.py) | CONSERVAR | P0 | 1 | 1 |
| [descargador.flujo_csmp](../../../descargador/flujo_csmp.py) | CONSERVAR | P0 | 4 | 2 |
| [descargador.lectura_bitacoras](../../../descargador/lectura_bitacoras.py) | CONSERVAR | P0 | 7 | 1 |
| [descargador.lotes](../../../descargador/lotes.py) | CONSERVAR | P0 | 3 | 5 |
| [descargador.motor](../../../descargador/motor.py) | CONSERVAR | P0 | 3 | 15 |
| [descargador.pausa](../../../descargador/pausa.py) | CONSERVAR | P0 | 0 | 1 |
| [descargador.perfiles](../../../descargador/perfiles.py) | CONSERVAR | P0 | 2 | 4 |
| [descargador.preferencias](../../../descargador/preferencias.py) | CONSERVAR | P0 | 3 | 1 |
| [descargador.preparar_entorno](../../../descargador/preparar_entorno.py) | CONSERVAR | P0 | 0 | 0 |
| [descargador.puente](../../../descargador/puente.py) | CONSERVAR | P0 | 1 | 1 |
| [descargador.resultados](../../../descargador/resultados.py) | ADAPTAR con exportador v1 aditivo después de G2 | P4 | 5 | 6 |
| [descargador.ventana_flujo](../../../descargador/ventana_flujo.py) | CONSERVAR | P0 | 4 | 1 |
| [descargador.ventana_resultados](../../../descargador/ventana_resultados.py) | CONSERVAR | P0 | 2 | 2 |
| [descargador.vinculos](../../../descargador/vinculos.py) | CONSERVAR | P0 | 2 | 3 |
| [nurus](../../../src/nurus/__init__.py) | CONSERVAR | P0 | 0 | 3 |
| [nurus.adapters](../../../src/nurus/adapters/__init__.py) | CONSERVAR | P0 | 0 | 0 |
| [nurus.adapters.outlook](../../../src/nurus/adapters/outlook.py) | CONSERVAR | P0 | 3 | 2 |
| [nurus.app](../../../src/nurus/app.py) | CONSERVAR | P0 | 1 | 0 |
| [nurus.bitacora_html](../../../src/nurus/bitacora_html.py) | CONSERVAR parser común; comprobar paquete autónomo | P3 | 0 | 4 |
| [nurus.domain](../../../src/nurus/domain/__init__.py) | CONSERVAR | P0 | 0 | 0 |
| [nurus.domain.models](../../../src/nurus/domain/models.py) | CONSERVAR | P0 | 0 | 8 |
| [nurus.personal](../../../src/nurus/personal/__init__.py) | CONSERVAR | P0 | 0 | 12 |
| [nurus.personal.activity](../../../src/nurus/personal/activity.py) | CONSERVAR | P0 | 0 | 3 |
| [nurus.personal.app](../../../src/nurus/personal/app.py) | CONSERVAR | P0 | 22 | 3 |
| [nurus.personal.app_base](../../../src/nurus/personal/app_base.py) | ADAPTAR entrada local después de G3/G5 | P5/P6 | 35 | 1 |
| [nurus.personal.appearance_view](../../../src/nurus/personal/appearance_view.py) | CONSERVAR | P0 | 2 | 1 |
| [nurus.personal.attachment_store](../../../src/nurus/personal/attachment_store.py) | CONSERVAR | P0 | 0 | 1 |
| [nurus.personal.bitacora_har](../../../src/nurus/personal/bitacora_har.py) | SEPARAR exportación/análisis sin cambiar reglas | P3 | 2 | 2 |
| [nurus.personal.bitacora_link](../../../src/nurus/personal/bitacora_link.py) | DESACOPLAR TRAS G3/G5; conservar recuperación | P6 | 4 | 2 |
| [nurus.personal.bitacora_lote](../../../src/nurus/personal/bitacora_lote.py) | SEPARAR exportación/análisis sin cambiar reglas | P3 | 2 | 2 |
| [nurus.personal.bitacoras](../../../src/nurus/personal/bitacoras.py) | SEPARAR exportación/análisis sin cambiar reglas | P3 | 3 | 5 |
| [nurus.personal.commands](../../../src/nurus/personal/commands.py) | CONSERVAR | P0 | 3 | 1 |
| [nurus.personal.config](../../../src/nurus/personal/config.py) | CONSERVAR | P0 | 3 | 11 |
| [nurus.personal.courts](../../../src/nurus/personal/courts.py) | CONSERVAR | P0 | 2 | 3 |
| [nurus.personal.current_tools](../../../src/nurus/personal/current_tools.py) | CONSERVAR | P0 | 6 | 5 |
| [nurus.personal.delivery](../../../src/nurus/personal/delivery.py) | CONSERVAR | P0 | 3 | 1 |
| [nurus.personal.download_link](../../../src/nurus/personal/download_link.py) | DESACOPLAR TRAS G3/G5; conservar recuperación | P6 | 5 | 4 |
| [nurus.personal.download_transfer](../../../src/nurus/personal/download_transfer.py) | DESACOPLAR TRAS G3/G5; conservar recuperación | P6 | 2 | 2 |
| [nurus.personal.excel_presentation](../../../src/nurus/personal/excel_presentation.py) | CONSERVAR | P0 | 0 | 1 |
| [nurus.personal.grid_edit](../../../src/nurus/personal/grid_edit.py) | CONSERVAR | P0 | 0 | 1 |
| [nurus.personal.importing](../../../src/nurus/personal/importing.py) | CONSERVAR | P0 | 2 | 3 |
| [nurus.personal.integral_contract](../../../src/nurus/personal/integral_contract.py) | DESACOPLAR TRAS G3/G5; conservar recuperación | P6 | 1 | 2 |
| [nurus.personal.interaction](../../../src/nurus/personal/interaction.py) | CONSERVAR | P0 | 0 | 2 |
| [nurus.personal.mail_category](../../../src/nurus/personal/mail_category.py) | CONSERVAR | P0 | 0 | 5 |
| [nurus.personal.mail_controls](../../../src/nurus/personal/mail_controls.py) | CONSERVAR | P0 | 3 | 1 |
| [nurus.personal.mail_presentation](../../../src/nurus/personal/mail_presentation.py) | CONSERVAR | P0 | 0 | 1 |
| [nurus.personal.mail_view](../../../src/nurus/personal/mail_view.py) | CONSERVAR | P0 | 9 | 1 |
| [nurus.personal.manual](../../../src/nurus/personal/manual.py) | CONSERVAR | P0 | 6 | 2 |
| [nurus.personal.modalities](../../../src/nurus/personal/modalities.py) | CONSERVAR | P0 | 1 | 3 |
| [nurus.personal.motor](../../../src/nurus/personal/motor/__init__.py) | CONSERVAR | P0 | 0 | 0 |
| [nurus.personal.motor.composicion](../../../src/nurus/personal/motor/composicion.py) | CONSERVAR | P0 | 2 | 5 |
| [nurus.personal.motor.contexto](../../../src/nurus/personal/motor/contexto.py) | CONSERVAR | P0 | 0 | 8 |
| [nurus.personal.motor.reglas_cumplimiento](../../../src/nurus/personal/motor/reglas_cumplimiento.py) | CONSERVAR | P0 | 5 | 2 |
| [nurus.personal.motor.reglas_espera](../../../src/nurus/personal/motor/reglas_espera.py) | CONSERVAR | P0 | 4 | 4 |
| [nurus.personal.motor.reglas_informes](../../../src/nurus/personal/motor/reglas_informes.py) | CONSERVAR | P0 | 5 | 2 |
| [nurus.personal.motor.textos](../../../src/nurus/personal/motor/textos.py) | CONSERVAR | P0 | 1 | 3 |
| [nurus.personal.motor.utilidades](../../../src/nurus/personal/motor/utilidades.py) | CONSERVAR | P0 | 1 | 5 |
| [nurus.personal.operations](../../../src/nurus/personal/operations.py) | CONSERVAR | P0 | 0 | 6 |
| [nurus.personal.outputs](../../../src/nurus/personal/outputs.py) | CONSERVAR | P0 | 17 | 17 |
| [nurus.personal.package_check](../../../src/nurus/personal/package_check.py) | CONSERVAR | P0 | 9 | 1 |
| [nurus.personal.personalization](../../../src/nurus/personal/personalization.py) | CONSERVAR | P0 | 3 | 2 |
| [nurus.personal.personalization_view](../../../src/nurus/personal/personalization_view.py) | CONSERVAR | P0 | 5 | 1 |
| [nurus.personal.product_checks](../../../src/nurus/personal/product_checks.py) | CONSERVAR | P0 | 3 | 5 |
| [nurus.personal.product_state](../../../src/nurus/personal/product_state.py) | CONSERVAR | P0 | 5 | 6 |
| [nurus.personal.record_edits](../../../src/nurus/personal/record_edits.py) | CONSERVAR | P0 | 8 | 7 |
| [nurus.personal.record_view](../../../src/nurus/personal/record_view.py) | CONSERVAR | P0 | 8 | 1 |
| [nurus.personal.registration](../../../src/nurus/personal/registration.py) | CONSERVAR | P0 | 4 | 2 |
| [nurus.personal.registration_excel](../../../src/nurus/personal/registration_excel.py) | CONSERVAR | P0 | 6 | 0 |
| [nurus.personal.reports](../../../src/nurus/personal/reports.py) | CONSERVAR | P0 | 3 | 4 |
| [nurus.personal.resolution_view](../../../src/nurus/personal/resolution_view.py) | CONSERVAR | P0 | 8 | 2 |
| [nurus.personal.resolutions](../../../src/nurus/personal/resolutions.py) | CONSERVAR | P0 | 10 | 8 |
| [nurus.personal.results_view](../../../src/nurus/personal/results_view.py) | ADAPTAR entrada local después de G3/G5 | P5/P6 | 12 | 1 |
| [nurus.personal.roster](../../../src/nurus/personal/roster.py) | CONSERVAR | P0 | 4 | 2 |
| [nurus.personal.rus_capture](../../../src/nurus/personal/rus_capture.py) | CONSERVAR | P0 | 0 | 1 |
| [nurus.personal.rus_capture_app](../../../src/nurus/personal/rus_capture_app.py) | CONSERVAR | P0 | 1 | 0 |
| [nurus.personal.selection](../../../src/nurus/personal/selection.py) | CONSERVAR | P0 | 0 | 6 |
| [nurus.personal.session](../../../src/nurus/personal/session.py) | CONSERVAR | P0 | 5 | 3 |
| [nurus.personal.sitfa](../../../src/nurus/personal/sitfa.py) | CONSERVAR | P0 | 1 | 5 |
| [nurus.personal.statistics](../../../src/nurus/personal/statistics.py) | CONSERVAR | P0 | 4 | 2 |
| [nurus.personal.sync](../../../src/nurus/personal/sync.py) | CONSERVAR | P0 | 5 | 6 |
| [nurus.personal.sync_view](../../../src/nurus/personal/sync_view.py) | CONSERVAR | P0 | 4 | 1 |
| [nurus.personal.table_update](../../../src/nurus/personal/table_update.py) | CONSERVAR | P0 | 1 | 3 |
| [nurus.personal.template_package](../../../src/nurus/personal/template_package.py) | CONSERVAR | P0 | 1 | 3 |
| [nurus.personal.template_validation](../../../src/nurus/personal/template_validation.py) | CONSERVAR | P0 | 0 | 4 |
| [nurus.personal.ui](../../../src/nurus/personal/ui.py) | CONSERVAR | P0 | 0 | 12 |
| [nurus.personal.ux_support](../../../src/nurus/personal/ux_support.py) | CONSERVAR | P0 | 3 | 2 |
| [nurus.personal.widgets](../../../src/nurus/personal/widgets.py) | CONSERVAR | P0 | 2 | 6 |
| [nurus.personal.word_preview](../../../src/nurus/personal/word_preview.py) | CONSERVAR | P0 | 3 | 1 |
| [nurus.personal.word_quality](../../../src/nurus/personal/word_quality.py) | CONSERVAR | P0 | 1 | 5 |
| [nurus.personal.work](../../../src/nurus/personal/work.py) | CONSERVAR | P0 | 18 | 8 |
| [nurus.personal.work_tools](../../../src/nurus/personal/work_tools.py) | CONSERVAR | P0 | 6 | 4 |
| [nurus.personal.work_view](../../../src/nurus/personal/work_view.py) | CONSERVAR | P0 | 5 | 1 |
| [nurus.rus](../../../src/nurus/rus/__init__.py) | CONSERVAR | P0 | 3 | 1 |
| [nurus.rus.catalog](../../../src/nurus/rus/catalog.py) | CONSERVAR | P0 | 0 | 3 |
| [nurus.rus.columns](../../../src/nurus/rus/columns.py) | CONSERVAR | P0 | 1 | 25 |
| [nurus.rus.models](../../../src/nurus/rus/models.py) | CONSERVAR | P0 | 0 | 7 |
| [nurus.rus.reader](../../../src/nurus/rus/reader.py) | CONSERVAR | P0 | 2 | 6 |
| [nurus.rus.rules](../../../src/nurus/rus/rules.py) | CONSERVAR | P0 | 3 | 14 |
| [nurus.rus.service](../../../src/nurus/rus/service.py) | CONSERVAR | P0 | 4 | 1 |
| [nurus.services](../../../src/nurus/services/__init__.py) | CONSERVAR | P0 | 0 | 0 |
| [nurus.services.communications](../../../src/nurus/services/communications.py) | CONSERVAR | P0 | 7 | 0 |
| [nurus.services.contacts](../../../src/nurus/services/contacts.py) | CONSERVAR | P0 | 2 | 1 |
| [nurus.services.delivery](../../../src/nurus/services/delivery.py) | CONSERVAR | P0 | 2 | 0 |
| [nurus.services.exports](../../../src/nurus/services/exports.py) | CONSERVAR | P0 | 2 | 1 |
| [nurus.services.file_output](../../../src/nurus/services/file_output.py) | CONSERVAR | P0 | 0 | 15 |
| [nurus.services.policy](../../../src/nurus/services/policy.py) | CONSERVAR | P0 | 0 | 2 |
| [nurus.services.products](../../../src/nurus/services/products.py) | CONSERVAR | P0 | 4 | 2 |
| [nurus.services.rendering](../../../src/nurus/services/rendering.py) | CONSERVAR | P0 | 2 | 3 |
| [nurus.services.resolutions](../../../src/nurus/services/resolutions.py) | CONSERVAR | P0 | 6 | 0 |
| [nurus.services.review_import](../../../src/nurus/services/review_import.py) | CONSERVAR | P0 | 2 | 0 |
| [nurus.services.statistics](../../../src/nurus/services/statistics.py) | CONSERVAR | P0 | 1 | 0 |
| [nurus.services.workflow](../../../src/nurus/services/workflow.py) | CONSERVAR | P0 | 3 | 0 |
| [nurus.storage](../../../src/nurus/storage/__init__.py) | CONSERVAR | P0 | 0 | 0 |
| [nurus.storage.database](../../../src/nurus/storage/database.py) | CONSERVAR | P0 | 2 | 5 |
