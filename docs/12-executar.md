# Executar a primeira fatia

## 1. Pré-requisitos e configuração

Rust 1.88 ou superior, Python 3, Docker Compose e LM Studio com suporte a visão e embeddings. A API usa PostgreSQL 17 com pgvector, provisionado em um container exclusivo pelo Compose. Escolha portas livres se os padrões já estiverem ocupados.

Execute `python3 scripts/configure.py` uma vez. Ele cria `.env` e `.env.worker` privados, sem sobrescrever arquivos existentes. A senha do banco e uma credencial interna do worker são geradas aleatoriamente. Ambos os arquivos são ignorados pelo Git.

Configure `MEDIA_ROOT` nos dois arquivos para o mesmo diretório (ou a montagem equivalente no worker). Nesta fatia o worker pode rodar na mesma máquina; em outro host ele precisa ler o mesmo armazenamento. O servidor precisa escrever snapshots/prévias em `MEDIA_ROOT/.archivist`.

`.env.worker` não contém DATABASE_URL, senha do PostgreSQL. Se mudar o modelo, endpoint ou token do worker, atualize os arquivos correspondentes. Segredos não devem ser passados em URLs.

## 2. LM Studio

Use o CLI `lms` existente. Quando a aplicação gráfica não for adequada ao servidor, o [runtime headless oficial](https://lmstudio.ai/docs/developer/core/headless) é uma opção. Confira o espaço disponível e respeite as licenças dos modelos.

```sh
lms daemon up
lms get google/gemma-4-e4b --gguf -y
lms get google/embedding-gemma-300m --gguf -y
lms load google/gemma-4-e4b --gpu 0.2 --context-length 4096 --parallel 1 --identifier archivist-vision -y
lms load google/embedding-gemma-300m --gpu off --identifier archivist-embedding -y
lms server start --bind 127.0.0.1 --port 1234
```

O offload de 20% é um exemplo conservador, não um requisito ou promessa de desempenho. Ajuste conforme recursos disponíveis, sem descarregar modelos usados por outros serviços. O modelo visual é maior que o de embeddings; memória disponível importa além da VRAM.

A configuração inicial usa Gemma 4 E4B Q4_K_M e EmbeddingGemma 300M QAT Q4_0 (768 dimensões). Verifique a variante efetivamente baixada em `lms ls --json` e atualize VISION_VERSION/EMBEDDING_VERSION se diferente. Esses identificadores de versão devem mudar quando os pesos mudarem; não reutilizar a mesma versão para pesos incompatíveis. Ainda não existe reindexação automática.

O adaptador usa `/v1/chat/completions` para visão e `/v1/embeddings` para texto. Prompts do EmbeddingGemma: consulta com `task: search result | query:`; documento com `title: none | text:`. Outros modelos exigem adaptar essa convenção antes de substituir. Respostas visuais truncadas são rejeitadas.

## 3. Banco e processos

```sh
docker compose -p archivist up -d db
cargo build --locked --bins
```

Terminal do servidor:

```sh
set -a
. ./.env
set +a
./target/debug/server
```

Outro terminal, com ambiente limpo para o worker:

```sh
set -a
. ./.env.worker
set +a
./target/debug/worker
```

O servidor aplica migrações na inicialização. O worker não acessa o banco. Para uso em máquina compartilhada, execute o worker em sessão/processo que não herde variáveis de banco do terminal do servidor.

Abrir `http://127.0.0.1:8098`. Para um host remoto, use um túnel SSH para essa porta. Em implantação compartilhada, restrinja o acesso pelo Tailscale; quem alcança o serviço pode consultar e importar imagens. Banco e LM Studio ficam em loopback por padrão; nenhuma porta pública é necessária.

## 4. Abrir e importar

O catálogo abre diretamente, sem chave individual ou sessão. O acesso é compartilhado e controlado pela rede privada/túnel. Importações novas usam a identidade compartilhada para idempotência; registros históricos são preservados.

Copie os exemplos sintéticos:

```sh
mkdir -p .local/media/examples
cp examples/images/*.png .local/media/examples/
```

Na interface, clique em **Importar imagens** e informe:

```text
examples/blue.png
examples/red.png
```

Confirme que a cópia terminou. A imagem fica visível antes da IA; acompanhe o status até “Pronta para busca”. Pesquise “uma imagem azul”, abra a prévia e baixe o original. Esses exemplos verificam encadeamento funcional, não qualidade em fotografias reais.

Snapshots duplicam os bytes importados em uma área gerenciada para que alterações no diretório de entrada não mudem os originais catalogados. Prévia e snapshot são gravados antes da transação do catálogo; rejeição de lote pode deixar arquivos não referenciados. Não há limpeza automática nesta fatia. Não remover o diretório gerenciado manualmente.

## Contrato implementado

As rotas `/api/v1` dispensam autenticação. Todas as rotas `/internal/v1` exigem `Authorization: Bearer <ARCHIVIST_WORKER_TOKEN>`. Não publicar o serviço diretamente na internet.

| Rota | Comportamento |
| --- | --- |
| POST /api/v1/imports | files: lista de caminhos relativos; copy_confirmed: true; header Idempotency-Key obrigatório; 202 com id/assets/status |
| GET /api/v1/imports/{id} | Registro da importação; processamento acompanhado pelos assets/jobs |
| GET /api/v1/assets | limit 1–100, offset >= 0; items e next_offset |
| GET /api/v1/assets/{id} | Descrição, modelos, dimensões e status/job_id |
| GET /api/v1/assets/{id}/preview | JPEG gerado sem IA |
| GET /api/v1/assets/{id}/original | Original; Range simples; download pela rede privada |
| POST /api/v1/search | query, limit e mode: hybrid ou text; items ordenados |
| GET /api/v1/jobs/{id} | Estado, tentativa e erro sanitizado |
| POST /api/v1/jobs/{id}/retry | Somente tarefa em failed; 202 ou 409 |
| POST /internal/v1/jobs/reserve | Reserva atômica, 200 com lease_token/attempt/generation; 204 se vazia |
| POST /internal/v1/jobs/{id}/renew | Renova lease válida por 120 s |
| POST /internal/v1/jobs/{id}/complete | Descrição/vetor/proveniência; lease e geração exigidas; repetição idêntica aceita |
| POST /internal/v1/jobs/{id}/fail | Registra falha; até 3 tentativas com espera de 30 s |
| GET /health/live e /health/ready | Processo e disponibilidade de banco/raiz de mídia |

A lista explícita de arquivos substitui batch_path nesta fatia; paginação usa offset, sem cursor. Sem upload web, filtros de categoria, PATCH humano ou rota de reanálise. Retry e fail não têm o contrato completo de Idempotency-Key planejado: repetição após mudança de estado pode retornar conflito. Reserva perdida é recuperada após lease expirar, não por chave de requisição. São diferenças explícitas em relação ao [rascunho futuro](04-api-draft.md).

A busca semântica retorna os vizinhos disponíveis, mesmo com baixa relevância; não há limiar calibrado. Scores de fusão de rankings não representam confiança. O índice consulta apenas a versão de embedding configurada, evitando misturar versões.

## Operação e recuperação

Um worker processa uma imagem por vez e renova a lease a cada 30 s. Se morrer, a tarefa pode ser reservada após 120 s. Resultados com lease antiga são rejeitados. Depois de três tentativas, use “Tentar análise novamente”.

Parar os processos não apaga dados. `docker compose -p archivist stop` para o banco preserva o volume. **Não usar down -v em um acervo que precise preservar.** Backup deve incluir mídia gerenciada e banco; restauração e rotina de backup ainda não foram implementadas.
