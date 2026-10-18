#!/usr/bin/env python3
"""Create private local settings once; never overwrite an existing .env."""
import secrets, pathlib
root=pathlib.Path(__file__).resolve().parents[1]
assert not (root/".env").exists() and not (root/".env.worker").exists(), "Configuration already exists; edit it without overwriting credentials"
password=secrets.token_hex(24)
values={
"POSTGRES_PASSWORD":password,"POSTGRES_PORT":"55439",
"DATABASE_URL":f"postgres://archivist:{password}@127.0.0.1:55439/archivist",
"ARCHIVIST_WORKER_TOKEN":secrets.token_hex(32),"MEDIA_ROOT":str(root/".local/media"),
"ARCHIVIST_BIND":"127.0.0.1:8098","ARCHIVIST_URL":"http://127.0.0.1:8098",
"LMSTUDIO_URL":"http://127.0.0.1:1234/v1","VISION_MODEL":"archivist-vision",
"VISION_VERSION":"gemma-4-E4B-it-Q4_K_M","EMBEDDING_MODEL":"archivist-embedding",
"EMBEDDING_VERSION":"embeddinggemma-300m-qat-Q4_0","EMBEDDING_DIMENSION":"768",
"RUST_LOG":"archivist=info"}
# Shell and Compose both read this file; single quotes must be escaped.
assert all("'" not in str(v) and "\n" not in str(v) for v in values.values()), "Unsupported quote/newline in configuration path"
path=root/".env"
with path.open("x") as f:
 path.chmod(0o600)
 f.write("".join(f"{k}='{v}'\n" for k,v in values.items()))
(root/".local/media").mkdir(parents=True,exist_ok=True)
assert path.stat().st_mode & 0o777 == 0o600
print("Created .env (private). Keep it out of Git; see docs/12-executar.md.")

worker_keys=['MEDIA_ROOT', 'ARCHIVIST_URL', 'ARCHIVIST_WORKER_TOKEN', 'LMSTUDIO_URL', 'VISION_MODEL', 'VISION_VERSION', 'EMBEDDING_MODEL', 'EMBEDDING_VERSION', 'EMBEDDING_DIMENSION', 'RUST_LOG']
with (root/".env.worker").open("x") as f:
 (root/".env.worker").chmod(0o600)
 f.write("".join(f"{k}='{values[k]}'\n" for k in worker_keys))
