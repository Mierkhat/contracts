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
