"""Hallazgos derivados: una sola construcción para edición, undo y sincronía."""

from copy import deepcopy
from typing import TypedDict
from nurus.capture.temporal import identity


class Finding(TypedDict):
    rule_id: str
    rule_version: str
    source_ids: list[str]
    record_identity: list[str]
    detected_at: str
    fact: str
    condition: str
    coverage: str
    status: str
    reason: str
    proposed_actions: list[str]
    confidence_class: str


def traditional_findings(work, row) -> list[Finding]:
    source = work.capture_sources[row.id]
    return [
        Finding(
            rule_id=rule,
            rule_version=work.ruleset_version,
            source_ids=[source['source_sha256']],
            record_identity=list(identity(source)),
            detected_at=work.as_of,
            fact=row.observation,
            condition='Regla tradicional conservada; no acredita actuaciones remotas.',
            coverage=source['coverage'],
            status='APLICA',
            reason='Evaluación del motor tradicional.',
            proposed_actions=list(row.actions),
            confidence_class='PARCIAL',
        )
        for rule in row.rules
    ]


def updated_contexts(work, rows):
    """Candidato independiente; quien modifica Work decide cuándo confirmarlo."""
    contexts = dict(getattr(work, 'capture_context', {}))
    for row in rows:
        if row.id not in contexts:
            continue
        context = deepcopy(contexts[row.id])
        contextual = [
            f for f in context['findings'] if not f['rule_id'].startswith(('ESPERA.', 'CUMPLIMIENTO.', 'INFORMES.'))
        ]
        context['findings'] = contextual + traditional_findings(work, row)
        contexts[row.id] = context
    return contexts
