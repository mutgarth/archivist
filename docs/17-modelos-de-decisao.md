# Decisão local de pastas

A análise visual continua no modelo de visão; transcrição e embeddings mantêm seus serviços. A organização pode usar um modelo especializado por `POST /v1/systemone` no mesmo LM Studio. É necessário LM Studio 0.4.26 build 4 ou posterior e runtime compatível.

Configuração privada do serviço de automação:

```dotenv
DECISION_MODE=systemone
DECISION_MODEL=identificador-do-modelo-carregado
DECISION_VERSION=nome-quantizacao-politica-v1
DECISION_MIN_CONFIDENCE=0.65
```

`LMSTUDIO_URL` inclui `/v1`. O padrão `DECISION_MODE=chat` preserva instalações existentes. Ao mudar de modelo ou critérios, altere `DECISION_VERSION` para reclassificar o catálogo sem refazer a análise dos vídeos. Não reutilize a versão do modelo de visão para a decisão.

A pergunta Choice contém todas as categorias com descrições, incluindo Revisar. A aplicação valida as opções, a distribuição completa, sua soma, valores finitos e a coerência da escolha. Respostas inválidas e falhas de conexão são retentadas; não existe fallback silencioso para outro modelo. Confiança abaixo do limiar encaminha para Revisar. O piso 0.65 é uma política conservadora compartilhada com a API, não uma garantia de acerto nem um limiar universal calibrado.

A justificativa é uma mensagem gerada pelo código, não uma explicação inventada pelo modelo; tags livres não são geradas no modo System One. As descrições e transcrições continuam disponíveis para busca. A tabela `decisions` do SQLite privado registra modelo, hash da análise, distribuição retornada, tempo e data. Esses dados não devem ser publicados, pois respostas de serviços podem conter dados derivados da mídia. Nenhuma decisão é usada como caminho de arquivo ou autoriza mover/excluir originais.

Validação: `python3 tests/automation.py`. Antes de ativar um modelo, avaliar exemplos positivos, mídia sem assunto, ambiguidades e o conteúdo real; medir latência no hardware alvo. Escores de confiança de modelos diferentes não são diretamente comparáveis. Exemplos sintéticos curtos são um teste funcional, não um benchmark representativo.

Fontes:
- https://lmstudio.ai/docs/developer/jev-compat/systemone
- https://huggingface.co/blog/ggml-org/decision-models-in-llamacpp
- https://docs.typesafe.ai/primitives/choice

## Carregamento em CPU pelo LM Studio

Quando a GPU está ocupada, `--gpu off` no runtime CUDA ainda pode reservar buffers de operações na GPU. Para carregar o decisor totalmente em CPU:

```sh
lms get https://huggingface.co/ggml-org/Kev-4B-GGUF@q4_k_m -y
lms runtime get llama.cpp:cpu -y
lms runtime select llama.cpp-linux-x86_64-avx2 --latest
lms load kev-4b --identifier archivist-kev --gpu off -c 4096 --parallel 1 -y
```

Depois, restaure o runtime usado pelo modelo de visão (por exemplo, `lms runtime select llama.cpp-linux-x86_64-nvidia-cuda12-avx2 --latest`). Isso não recarrega as instâncias existentes. Os identificadores configurados precisam estar carregados após reiniciar o LM Studio; confirme com `lms ps` antes de iniciar os workers. A seleção de runtime é global para novos carregamentos: faça essa operação sem outros carregamentos simultâneos.
