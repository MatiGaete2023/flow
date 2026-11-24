"""Custodia de la revisión en memoria; no concede autorización de registro.

Retiene el aviso y la huella de revisión que PreviewBatch por sí solo no conserva.
La comparación vuelve a cargar el ZIP vigente y exige la misma selección.
"""
from dataclasses import dataclass
from .contracts import ContractError
from .flow_batch import WorkBatchReview, build_review
from .transfer import PreviewBatch


@dataclass(frozen=True)
class ConfirmedPreview:
    review: WorkBatchReview
    batch: PreviewBatch

    def __post_init__(self):
        if type(self.review) is not WorkBatchReview or type(self.batch) is not PreviewBatch:
            raise ContractError('REVISION_NO_CUSTODIABLE')
        candidates = self.review.candidates
        if len(candidates) != len(self.batch.operations):
            raise ContractError('LOTE_CAMBIADO_REVISAR')
        for candidate, op in zip(candidates,self.batch.operations):
            if (op.source.tool != 'FLOW' or op.source.work_id != candidate.work_id or
                op.source.row_ref != candidate.row_ref or op.remote_identity != candidate.identity or
                op.origin != candidate.origin or op.destination != candidate.destination or
                op.observation_state != candidate.observation_state or op.approved_text != candidate.text or
                op.approval_version != candidate.revision+1 or self.batch.source_digest != candidate.source_digest or
                self.batch.revision != candidate.revision):
                raise ContractError('LOTE_CAMBIADO_REVISAR')

    def revalidate(self, work, requests, *, report_catalog=None):
        """No entrega permiso ni renueva la aprobación; rechaza cualquier cambio."""
        current=build_review(work,requests,work_id=self.review.candidates[0].work_id,report_catalog=report_catalog)
        if current != self.review:
            raise ContractError('LOTE_CAMBIADO_REVISAR')
        # La construcción congelada protege avisos; se valida también el sobre antes del uso.
        return PreviewBatch.decode(self.batch.encode())
