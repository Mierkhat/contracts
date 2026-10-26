#!/usr/bin/env python3
"""Build unchanged pinned upstream sources and record exact artifact provenance."""
import hashlib, json, os, pathlib, shutil, subprocess
ROOT = pathlib.Path(__file__).resolve().parents[1]
PINS = {'soroswap': ('https://github.com/soroswap/core.git', '6eade008ee6794075ad941609e0fef0d1124d3eb'), 'defindex': ('https://github.com/paltalabs/defindex.git', 'a0fcabfae38f1c7b0c9c2f10ff51d225f8edc2f1')}
def run(*args, **kw): subprocess.run(args, check=True, **kw)
for name, (url, rev) in PINS.items():
    checkout = ROOT/'upstream'/name
    if not checkout.exists():
        checkout.parent.mkdir(exist_ok=True)
        run('git','clone',url,str(checkout))
    run('git','-C',str(checkout),'checkout','--detach',rev)
    if subprocess.check_output(['git','-C',str(checkout),'status','--porcelain']).strip():
        raise SystemExit('Refusing modified upstream source: '+name)
artifacts = {}
for name in ['token','pair','factory','router']:
    directory = ROOT/'upstream/soroswap/contracts'/name
    # Old contract macros declare testutils in consumers; new rustc check-cfg needs this declaration.
    env = {**os.environ, 'RUSTFLAGS': '--check-cfg=cfg(feature,values("testutils"))'}
    run('cargo','+1.91.0','build','--locked','--release','--target','wasm32v1-none','--manifest-path',str(directory/'Cargo.toml'),env=env)
    source = directory/'target/wasm32v1-none/release'/(('soroban_token_contract' if name == 'token' else 'soroswap_'+name)+'.wasm')
    # Upstream contractimport paths predate wasm32v1-none. This is an exact copy, not a WASM optimizer.
    historical = directory/'target/wasm32-unknown-unknown/release'
    historical.mkdir(parents=True,exist_ok=True)
    shutil.copyfile(source, historical/source.name)
    shutil.copyfile(source, historical/(source.stem+'.optimized.wasm'))
    artifacts['soroswap_'+name] = (source, 'soroswap', directory/'Cargo.lock')
directory=ROOT/'upstream/defindex/apps/contracts'
run('cargo','+1.91.0','build','--locked','--release','--target','wasm32v1-none','--manifest-path',str(directory/'Cargo.toml'),'-p','defindex-vault')
artifacts['defindex_vault']=(directory/'target/wasm32v1-none/release/defindex_vault.wasm','defindex',directory/'Cargo.lock')
output=ROOT/'artifacts';output.mkdir(exist_ok=True)
manifest={'toolchain':'rustc 1.91.0 (f8297e351 2025-10-28)','target':'wasm32v1-none','artifacts':{}}
for name,(source,repo,lock) in artifacts.items():
    data=source.read_bytes();shutil.copyfile(source,output/(name+'.wasm'))
    manifest['artifacts'][name]={'source_repo':PINS[repo][0],'source_commit':PINS[repo][1], 'sha256':hashlib.sha256(data).hexdigest(),'bytes':len(data),'cargo_lock_sha256':hashlib.sha256(lock.read_bytes()).hexdigest()}
(ROOT/'evidence/upstream-build.json').write_text(json.dumps(manifest,indent=2)+'\n')
print(json.dumps(manifest,indent=2))
