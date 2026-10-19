# Importação automática e pastas virtuais

O processo `scripts/automation.py` observa diretórios configurados, importa mídia pela API existente e classifica os itens concluídos usando um modelo local no LM Studio. Não move, renomeia ou apaga os arquivos de origem. As pastas virtuais são metadados no PostgreSQL; não são diretórios no NAS.

## Monitoramento

Configure `AUTOMATION_CONFIG` com o caminho de um JSON privado contendo `roots` (lista de diretórios de entrada), `state_path` (SQLite local persistente), `settle_seconds` (padrão 90) e `poll_seconds` (padrão 15). O processo também usa as variáveis do worker: `MEDIA_ROOT`, `MEDIA_VOLUME_ID`, `ARCHIVIST_URL`, `ARCHIVIST_WORKER_TOKEN`, `LMSTUDIO_URL`, `VISION_MODEL` e `VISION_VERSION`. `DECISION_MODEL` e `DECISION_VERSION` podem selecionar outro modelo/proveniência sem mudar o modelo de visão. O serviço não precisa de credenciais do banco.

O monitor percorre subpastas, ignora diretórios e arquivos ocultos, links simbólicos e extensões não suportadas. São aceitos MP4, MOV, M4V, PNG, JPG, JPEG e WebP. O diretório de armazenamento interno e `auto-imports` não são monitorados recursivamente. Para observar uma pasta dentro de `MEDIA_ROOT`, inclua-a explicitamente em `roots`.

Tamanho e timestamps devem permanecer estáveis por pelo menos 90 segundos. Isso é uma heurística: uma transferência pausada por mais tempo pode parecer concluída. Para garantir uma entrega completa, copie com extensão `.part` e renomeie para a extensão final depois que terminar. A cópia é verificada por SHA-256 e a API também verifica o snapshot. Arquivos alterados durante a cópia são rejeitados e tentados novamente.

A deduplicação automática usa o SHA-256 do catálogo inteiro, mesmo se o arquivo mudar de nome. O SQLite registra arquivos concluídos e tentativas; reiniciar não reimporta os mesmos arquivos. Uma alteração real no conteúdo gera um novo item e preserva o snapshot anterior. Falhas são tentadas novamente após cinco minutos. Entradas inválidas permanecem na origem e não são descartadas. A importação manual conserva seu contrato anterior.

A entrada é copiada para `MEDIA_ROOT/auto-imports/<sha256>/<nome>` antes de passar pela API, que mantém seu próprio original verificado. Após confirmação da importação, o monitor remove somente sua cópia temporária de entrada; o arquivo observado e o original verificado permanecem. Falhas podem deixar cópias temporárias para inspeção; não há remoção automática da origem ou do aparelho. Os limites de formato, tamanho e duração da API continuam válidos.

## Classificação

Depois que visão/transcrição terminam, o monitor envia a descrição e amostras distribuídas de imagem/fala ao `/v1/chat/completions` com JSON Schema. Há dez categorias fixas, tags, justificativa, confiança estimada e proveniência. Confiança abaixo de 0,65 leva a `Revisar`. Resposta incompleta ou inválida também fica em `Revisar`, com confiança zero e motivo explícito; falhas de conexão são tentadas novamente. Essa confiança não é uma probabilidade calibrada. A classificação não recebe ferramentas de filesystem e não executa instruções contidas na mídia.

O servidor valida a categoria e limites dos campos novamente. A gravação exige token interno e o hash exato da análise concluída; resultados atrasados não sobrescrevem uma análise nova. Itens existentes são classificados sem repetir visão ou transcrição. Após reanálise, a categoria anterior continua visível até a nova classificação.

Classificação roda sequencialmente, priorizando os jobs de análise; uma fila de análise continuamente ocupada pode adiar a organização. No máximo 24 amostras de texto entram na decisão, com limites por trecho, então informações raras podem não estar representadas. Falhas do modelo não escondem o original nem bloqueiam as importações futuras. Pastas manuais, edição da taxonomia e movimentação física não fazem parte desta versão.

## Interface e operações

`GET /api/v1/folders` lista pastas e contagens. `GET /api/v1/assets?folder=...` e `POST /api/v1/search` com `folder` filtram o catálogo. O detalhe exibe a justificativa e tags. `GET /api/v1/automation` mostra a última atividade do monitor; sem heartbeat por 15 minutos, aparece desconectado. A primeira verificação completa após reiniciar aguarda novamente o período de estabilidade.

Instale como serviço de usuário com `EnvironmentFile` do worker e `AUTOMATION_CONFIG`. Use `ExecStartPre=/usr/bin/mountpoint -q <montagem>` e `Restart=on-failure`. O marcador de volume e a readiness são conferidos antes de cada ciclo. Mantenha configurações, SQLite e arquivos de mídia fora do Git.

## Modelos avaliados

- Gemma 4 E4B: pode reutilizar o modelo de visão já carregado, sem outro runtime; validar saída estruturada na instalação antes de ativar.
- Qwen3.5-4B: alternativa compacta multimodal; GGUF pode ser executado em runtime compatível do LM Studio.
- Ollama é uma alternativa de runtime, não um modelo de decisão. Não é necessário instalá-lo se o LM Studio atende a inferência local.

Referências: [LM Studio JSON Schema](https://lmstudio.ai/docs/developer/openai-compat/structured-output), [Gemma 4](https://ai.google.dev/gemma/docs/core), [Qwen3.5-4B](https://huggingface.co/Qwen/Qwen3.5-4B), [Ollama Gemma](https://ollama.com/library/gemma4).

Validação: `python3 tests/automation.py`, `cargo test --locked`, `cargo clippy --all-targets --all-features -- -D warnings`, `python3 scripts/test.py`. A suíte de integração usa banco descartável, mídia sintética e endpoint de IA offline. Testar o modelo real e a montagem SMB complementa esses testes.
