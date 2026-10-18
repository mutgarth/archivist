# Archivist

Gerenciador de mídia auto-hospedado e multiusuário, com descrição local de imagens e busca semântica.

**Primeira fatia experimental implementada:** importar PNG/JPEG/WebP já copiados → gerar prévia → descrever via LM Studio → gerar embedding textual → persistir no PostgreSQL/pgvector → buscar e visualizar → baixar original.

## Executar

Requer Rust 1.88+, Python 3, Docker Compose e [LM Studio](https://lmstudio.ai/docs/developer) com modelos de visão e embedding. A interface é servida pelo próprio binário Rust; não exige Node para uso.

```sh
git clone https://github.com/mutgarth/archivist.git
cd archivist
python3 scripts/configure.py
docker compose -p archivist up -d db
cargo build --locked --bins
```

Siga o [guia de execução](docs/12-executar.md) para carregar os modelos, iniciar servidor/worker, obter a chave individual e importar os exemplos sintéticos. O serviço abre em `http://127.0.0.1:8098`. Em servidor remoto, use túnel SSH ou um proxy HTTPS; não envie chaves por HTTP aberto.

## O que funciona

- Catálogo compartilhado por contas individuais, com chaves de acesso separadas.
- Importação explícita de até 20 imagens por lote; confirmação do término da cópia.
- PNG, JPEG e WebP até 20 MB e 40 megapixels por imagem.
- Snapshot estável do original e prévia JPEG, sem depender da IA.
- Worker separado via API autenticada, fila no PostgreSQL, heartbeat, tentativas limitadas e conclusão idempotente.
- Descrição local com Gemma 4 E4B e embeddings textuais com EmbeddingGemma pelo LM Studio.
- Busca híbrida por fusão de rankings textual/vetorial; modo somente texto quando a IA está offline.
- Visualização, status da análise, tentativa manual após falha e download com Range simples.

O catálogo é compartilhado entre todos os usuários configurados. Não há acervos privados por usuário nesta versão. A quantidade de usuários não é fixada; capacidade ainda não foi medida.

## Testar

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --locked
python3 scripts/test.py
```

O último comando cria um banco e mídia descartáveis, testa a API com IA propositalmente offline e remove o banco criado. O banco do serviço fica intacto. Consulte [validação e limites](docs/13-validacao-primeira-fatia.md).

## Estrutura

- `src/domain.rs`: invariantes de caminhos e vetores.
- `src/media.rs`: acesso restrito à raiz, inspeção e snapshots.
- `src/ai.rs`: cliente LM Studio e validação da saída.
- `src/server.rs`: casos de uso da fatia, persistência e adaptador HTTP.
- `src/bin/`: servidor e worker.
- `migrations/`, `web/`, `tests/`: banco, interface e verificações.
- [Documentação e decisões](docs/README.md).

## Limites atuais

Áudio/vídeo, coleções, categorias, correções humanas, reanálise por modelo e instalação empacotada em container ainda não estão implementados. A arquitetura completa está documentada como direção futura; as rotas disponíveis estão no [contrato implementado](docs/12-executar.md).

Edição de vídeo fica fora do produto: nenhum corte, timeline ou render de edição. A primeira fatia não instala NAS, não formata discos e não configura backup. Não há benchmark de qualidade ou capacidade; descrições podem errar.

Segredos, dados de execução e material privado ficam fora do Git. Licença de distribuição ainda a definir; visibilidade pública não equivale a uma licença de código aberto.
