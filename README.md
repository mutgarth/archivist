# Archivist

Gerenciador de mídia auto-hospedado e multiusuário, com preservação de vídeos originais, descrição local de prévias e busca semântica.

**Fatia experimental implementada:** importar imagens e vídeos já copiados → preservar original → gerar prévia → descrever via LM Studio → gerar embedding textual → buscar, visualizar e baixar o original. Vídeos têm reprodução por derivado separado, descrições de múltiplos momentos e transcrição local com busca por trechos.

## Executar

Requer Linux, FFmpeg/FFprobe com suporte a scale/reset_sar, Rust 1.88+, Python 3, Docker Compose e [LM Studio](https://lmstudio.ai/docs/developer) com modelos de visão e embedding, além de Whisper local para vídeos com áudio. A interface é servida pelo próprio binário Rust; não exige Node para uso.

```sh
git clone https://github.com/mutgarth/archivist.git
cd archivist
python3 scripts/configure.py
docker compose -p archivist up -d db
cargo build --locked --bins
```

Siga o [guia de execução](docs/12-executar.md) para carregar os modelos, iniciar servidor/worker, abrir o catálogo e importar os exemplos sintéticos. O serviço abre em `http://127.0.0.1:8098`. Em servidor remoto, use túnel SSH ou acesso restrito pelo Tailscale. O catálogo não exige login; o controle de acesso fica na rede.

## O que funciona

- Catálogo compartilhado sem login, com acesso controlado pela rede privada.
- Importação explícita de até 20 arquivos por lote; confirmação do término da cópia.
- PNG, JPEG e WebP até 20 MB e 40 megapixels por imagem.
- Vídeos MP4/MOV/M4V: cópia verificada por SHA-256, metadados técnicos e reprodução H.264/AAC separada.
- Identidade de volume para bloquear importações se a futura montagem do NAS estiver ausente ou incorreta.
- Snapshot estável do original e prévia JPEG, sem depender da IA.
- Worker separado via API autenticada, fila no PostgreSQL, heartbeat, tentativas limitadas e conclusão idempotente.
- Descrição local com Gemma 4 E4B e embeddings textuais com EmbeddingGemma pelo LM Studio.
- Busca híbrida por fusão de rankings textual/vetorial; modo somente texto quando a IA está offline.
- Visualização, status da análise, tentativa manual após falha e download com Range simples.

O catálogo é compartilhado entre as pessoas com acesso à rede do serviço. Não há acervos privados por usuário nesta versão. A quantidade de usuários não é fixada; capacidade ainda não foi medida.

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

## Vídeos

MP4, MOV e M4V com original verificado por SHA-256, thumbnail e derivado de reprodução separado. Veja [vídeos e armazenamento](docs/14-videos-armazenamento.md) para configuração, limites e migração futura para uma pasta montada do TrueNAS.

## Busca por trechos

Quadros amostrados e fala transcrita recebem timestamps e embeddings próprios. Os resultados abrem a prévia no ponto encontrado. Consulte [análise temporal](docs/15-analise-temporal.md) para configurar o Whisper, reanalisar vídeos existentes e conhecer limites.
