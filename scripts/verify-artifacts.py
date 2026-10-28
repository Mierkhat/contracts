#!/usr/bin/env python3
import hashlib,json,pathlib
root=pathlib.Path(__file__).resolve().parents[1]
manifest=json.loads((root/'evidence/upstream-build.json').read_text())
for name,record in manifest['artifacts'].items():
    file=root/'artifacts'/f'{name}.wasm'
    assert hashlib.sha256(file.read_bytes()).hexdigest()==record['sha256'],name
fixture=json.loads((root/'evidence/testnet-fixture.json').read_text())
wasm=root/'target/wasm32v1-none/release/mierkhat_guard.wasm'
assert hashlib.sha256(wasm.read_bytes()).hexdigest()==fixture['bundle']['guardWasmHash'],'Guard differs from deployed proof artifact'
assert {'contract':1012} in fixture['rollback']['observedDiagnosticErrors']
assert {'contract':1008} in fixture['replayProof']['observedDiagnosticErrors']
assert fixture['rollback']['beforeDigest']==fixture['rollback']['afterDigest']
assert fixture['replayProof']['atMostOneCommitted']
assert fixture['replayProof']['markerLiveUntilLedger']>fixture['replayProof']['validBeforeLedger']
print('PASS: upstream WASM hashes, guard matches deployed proof, rollback/replay evidence shape')

migration_file=root/'evidence/testnet-migration-routes.json'
if migration_file.exists():
    migration=json.loads(migration_file.read_text())
    migration_wasm=root/'target/wasm32v1-none/release/mierkhat_migration.wasm'
    migration_hash=hashlib.sha256(migration_wasm.read_bytes()).hexdigest()
    assert len(migration['bundles'])==2 and migration.get('verifiedAt')
    for bundle in migration['bundles']:
        assert bundle['guardWasmHash']==migration_hash and bundle['environment']=='testnet'
        assert bundle['profile']['vault']['address']!=bundle['migration']['destination']['address']
    print('PASS: separate immutable migration artifact matches both verified testnet profiles')
