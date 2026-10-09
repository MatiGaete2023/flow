"""El inventario rechaza un entorno que no corresponde al lock distribuido."""
import importlib.util
from pathlib import Path
import pytest

spec=importlib.util.spec_from_file_location('dependency_inventory',Path(__file__).parents[1]/'tools/flow_dependency_inventory.py')
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)


def test_inventory_verifies_versions_and_does_not_include_install_locations(tmp_path,monkeypatch):
    lock=tmp_path/'env.lock';lock.write_text('example-package==1.0 \\\n    --hash=sha256:'+('a'*64)+'\n')
    monkeypatch.setattr(module.metadata,'version',lambda name:'1.0')
    monkeypatch.setattr(module.subprocess,'check_output',lambda *args,**kwargs:'a'*40)
    proof=module.inventory(lock)
    assert proof['locked_versions_verified'] and proof['dependencies']=={'example-package':'1.0'}
    assert proof['lockfile']=='env.lock' and 'path' not in proof
    monkeypatch.setattr(module.metadata,'version',lambda name:'2.0')
    with pytest.raises(ValueError,match='Versiones distintas'):module.inventory(lock)


def test_lock_rejects_duplicate_or_empty_inputs(tmp_path):
    lock=tmp_path/'env.lock'
    for content in ('','example-package==1\nexample_package==2\n'):
        lock.write_text(content)
        with pytest.raises(ValueError):module.locked_versions(lock)
