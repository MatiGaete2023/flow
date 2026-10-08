from concurrent.futures import ThreadPoolExecutor
from dataclasses import replace
from datetime import date, datetime, timedelta, timezone

import pytest

from nurus.personal.registration import (
    FormEvidence, Intent, Journal, RemoteEntry, RemoteIdentity, Snapshot, recover, submit,
)


class Clock:
    def __init__(self):
        self.now = datetime(2026, 10, 5, 10, tzinfo=timezone(timedelta(hours=-3)))

    def __call__(self):
        return self.now


def intent(**changes):
    data = Intent(RemoteIdentity('666', '10', '20', '30', '40', 'X-1-2026'),
                  '  Texto efectivo del Excel.\nSegunda línea.  ', 'Al Tribunal', 'Realizada',
                  True, 'usuario-ficticio', 'a' * 64, 'Cumplimiento', 8, 'Tribunal ficticio', '3', '1', '1')
    return replace(data, **changes)


class Adapter:
    def __init__(self, data, clock):
        self.intent = data
        self.clock = clock
        self.entries = []
        self.saves = 0
        self.reads = 0
        self.fail_after_save = None
        self.effective_form = data

    def read(self, identity):
        self.reads += 1
        return Snapshot(identity, self.intent.author_id, self.clock().isoformat(), 'b' * 64,
                        date(2026, 6, 5), self.clock().date(), True, tuple(self.entries),
                        self.intent.stage, self.intent.antiguo, self.intent.modality)

    def prepare_form(self, data):
        return FormEvidence(self.effective_form, 2000, self.clock().isoformat(), 'd' * 64)

    def save(self, data):
        self.saves += 1
        self.entries.append(RemoteEntry(str(100 + self.saves), self.clock().isoformat(), data.author_id,
                                        data.text, data.type, data.state, data.send_to_tribunal))
        if self.fail_after_save:
            raise self.fail_after_save


def setup(tmp_path):
    clock = Clock()
    data = intent()
    journal = Journal(tmp_path / 'operaciones.sqlite', clock=clock)
    operation = journal.prepare(data)['operation_id']
    return clock, data, journal, operation, Adapter(data, clock)


def test_timeout_after_real_save_recovers_once_and_keeps_exact_text(tmp_path):
    clock, data, journal, operation, adapter = setup(tmp_path)
    adapter.fail_after_save = TimeoutError('No se imprime esta posible información de transporte')
    receipt = submit(journal, operation, adapter)
    assert receipt['state'] == 'PENDIENTE_EXCEL' and receipt['verified'] is True
    assert receipt['text'] == data.text and receipt['cc'] == 1
    assert adapter.saves == 1
    again = submit(Journal(journal.path, clock=clock), operation, adapter)
    assert again == receipt and adapter.saves == 1


def test_crash_between_post_and_receipt_does_not_send_again(tmp_path):
    clock, data, journal, operation, adapter = setup(tmp_path)
    adapter.fail_after_save = SystemExit('Corte después del POST')
    with pytest.raises(SystemExit):
        submit(journal, operation, adapter)
    assert journal.get(operation)['state'] == 'ENVIANDO'
    receipt = recover(Journal(journal.path, clock=clock), operation, adapter)
    assert receipt['verified'] is True and adapter.saves == 1


def test_existing_same_text_does_not_prove_a_new_save(tmp_path):
    clock, data, journal, operation, adapter = setup(tmp_path)
    adapter.entries.append(RemoteEntry('old', clock().isoformat(), data.author_id,
                                       data.text, data.type, data.state, data.send_to_tribunal))
    baseline = adapter.read(data.identity)
    journal.begin(operation, baseline)
    assert recover(journal, operation, adapter)['state'] == 'NO_HALLADA'
    assert recover(journal, operation, adapter)['state'] == 'NO_HALLADA'
    assert adapter.saves == 0
    with pytest.raises(ValueError, match='ya intentó'):
        journal.begin(operation, baseline)


@pytest.mark.parametrize('changes', [
    {'author_id': 'otro-usuario'}, {'type': 'Administrativa'}, {'state': 'Pendiente'},
    {'send_to_tribunal': False}, {'text': 'Texto parecido pero distinto'},
])
def test_each_remote_field_must_match_not_just_the_text(tmp_path, changes):
    clock, data, journal, operation, adapter = setup(tmp_path)
    journal.begin(operation, adapter.read(data.identity))
    entry = RemoteEntry('101', clock().isoformat(), data.author_id, data.text, data.type, data.state, True)
    adapter.entries.append(replace(entry, **changes))
    assert recover(journal, operation, adapter)['verified'] is False
    assert adapter.saves == 0


def test_two_matching_new_entries_remain_ambiguous(tmp_path):
    clock, data, journal, operation, adapter = setup(tmp_path)
    journal.begin(operation, adapter.read(data.identity))
    adapter.entries = [RemoteEntry(str(n), clock().isoformat(), data.author_id,
                                   data.text, data.type, data.state, True) for n in (101, 102)]
    assert recover(journal, operation, adapter)['state'] == 'AMBIGUA'


@pytest.mark.parametrize('change', [
    {'complete': False}, {'source_sha256': ''}, {'author_id': 'otro'},
    {'observed_at': '2026-10-05T09:00:00-03:00'},
    {'stage': '4'}, {'modality': '2'},
    {'identity': RemoteIdentity('667', '10', '20', '30', '40', 'X-1-2026')},
])
def test_invalid_baseline_blocks_before_post(tmp_path, change):
    clock, data, journal, operation, adapter = setup(tmp_path)
    with pytest.raises(ValueError):
        journal.begin(operation, replace(adapter.read(data.identity), **change))
    assert journal.get(operation)['state'] == 'PREPARADA'
    assert adapter.saves == 0


def test_checked_checkbox_or_form_changes_block_save(tmp_path):
    clock, data, journal, operation, adapter = setup(tmp_path)
    adapter.effective_form = replace(data, send_to_tribunal=False)
    with pytest.raises(ValueError, match='campos efectivos'):
        submit(journal, operation, adapter)
    assert journal.get(operation)['state'] == 'PREPARADA' and adapter.saves == 0


def test_storage_failure_prevents_post(tmp_path, monkeypatch):
    clock, data, journal, operation, adapter = setup(tmp_path)
    def failed(*args):
        raise OSError('No se pudo conservar la intención')
    monkeypatch.setattr(journal, 'begin', failed)
    with pytest.raises(OSError):
        submit(journal, operation, adapter)
    assert adapter.saves == 0


def test_row_order_and_source_edits_do_not_duplicate_same_intent(tmp_path):
    clock, data, journal, operation, adapter = setup(tmp_path)
    second = journal.prepare(replace(data, source_row=19, source_sha256='c' * 64))
    assert second['operation_id'] == operation
    with ThreadPoolExecutor(max_workers=2) as pool:
        found = list(pool.map(lambda n: Journal(journal.path, clock=clock).prepare(data)['operation_id'], range(2)))
    assert found == [operation, operation]
    clock.now += timedelta(days=1)
    with pytest.raises(ValueError, match='operación pendiente'):
        journal.prepare(data)


def test_form_limit_counts_utf16_and_unknown_types_are_not_guessed():
    with pytest.raises(ValueError, match='2000'):
        data = intent(text='😀' * 1001)
        FormEvidence(data, 2000, Clock()().isoformat(), 'd' * 64).validate(data, Clock()())
    intent(text='😀' * 1001).validate()  # Preparar no presupone el límite institucional.
    with pytest.raises(ValueError, match='tipo y estado'):
        intent(type='Con carga').validate()
    with pytest.raises(ValueError, match='identidad real'):
        intent(identity=RemoteIdentity('666', '10', '', '30', '40', 'X-1-2026')).validate()


def test_recovery_partial_or_stale_does_not_clear_uncertainty(tmp_path):
    clock, data, journal, operation, adapter = setup(tmp_path)
    baseline = adapter.read(data.identity)
    journal.begin(operation, baseline)
    journal.uncertain(operation)
    clock.now += timedelta(seconds=121)
    with pytest.raises(ValueError):
        journal.reconcile(operation, baseline)
    assert journal.get(operation)['state'] == 'INCIERTA'


def test_corrupt_or_unrelated_database_is_not_recreated(tmp_path):
    path = tmp_path / 'other.sqlite'
    path.write_bytes(b'no es una base SQLite')
    with pytest.raises(Exception):
        Journal(path)
    assert path.read_bytes() == b'no es una base SQLite'


def test_identical_entry_already_saved_today_is_reused_without_a_post(tmp_path):
    clock, data, journal, operation, adapter = setup(tmp_path)
    adapter.entries = [RemoteEntry('existing', clock().isoformat(), data.author_id,
                                   data.text, data.type, data.state, data.send_to_tribunal)]
    receipt = submit(journal, operation, adapter)
    assert receipt['verified'] is True and receipt['new_registration'] is False
    assert receipt['remote_entry_id'] == 'existing' and adapter.saves == 0


def test_same_text_from_a_previous_day_can_be_a_new_review(tmp_path):
    clock, data, journal, operation, adapter = setup(tmp_path)
    adapter.entries = [RemoteEntry('old', '2026-10-04T10:00:00-03:00', data.author_id,
                                   data.text, data.type, data.state, data.send_to_tribunal)]
    receipt = submit(journal, operation, adapter)
    assert receipt['new_registration'] is True and adapter.saves == 1
    assert receipt['remote_entry_id'] != 'old'


def test_several_identical_entries_today_require_review_before_sending(tmp_path):
    clock, data, journal, operation, adapter = setup(tmp_path)
    adapter.entries = [RemoteEntry(str(n), clock().isoformat(), data.author_id,
                                   data.text, data.type, data.state, data.send_to_tribunal) for n in (8, 9)]
    result = submit(journal, operation, adapter)
    assert result['state'] == 'REVISAR_PREVIAS' and result['verified'] is False
    assert recover(journal, operation, adapter)['state'] == 'REVISAR_PREVIAS'
    assert adapter.saves == 0
