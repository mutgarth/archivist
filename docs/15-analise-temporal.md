# Análise temporal de vídeos

O pipeline video-timeline-v1 combina quadros distribuídos no vídeo e transcrição local de áudio. O arquivo original continua somente leitura; hash, bytes, codecs e metadados originais não são substituídos. Reprodução e extração usam derivados separados.

## Processamento

- Valida o SHA-256 do original e mantém o MP4 de reprodução já existente.
- Extrai áudio mono PCM 16 kHz em blocos de até 60 segundos, mantidos em memória. Cada bloco é enviado ao Whisper local.
- Converte os timestamps de segundos relativos ao bloco para milissegundos absolutos do vídeo. Ignora texto vazio e segmentos com no_speech_prob acima de 0,6, quando esse campo é fornecido.
- Extrai aproximadamente um quadro a cada 30 segundos, incluindo início e fim. O limite é 24 quadros; em vídeos maiores o intervalo cresce. Imagens de amostra são guardadas em .archivist/frames.
- Descreve cada quadro com o modelo visual local e gera embeddings independentes para cada descrição e trecho de fala.
- Publica todos os trechos e sua proveniência em uma única transação, somente após validar tempos e vetores.

Esta fatia analisa vídeos de até 1 hora e até 4.000 trechos. A importação de originais continua aceitando os limites maiores documentados no armazenamento, mas a análise temporal desses arquivos maiores ainda não é suportada. Não há detecção automática de cortes ou garantia de capturar eventos curtos entre os quadros. Os timestamps de fala são aproximados; cortes entre blocos, ruído e silêncio podem causar erros ou omissões. Não há identificação de locutores.

Vídeos sem faixa de áudio recebem apenas trechos visuais. Vídeos com áudio exigem o Whisper disponível; falha na transcrição não é tratada como sucesso parcial. Um áudio sem fala reconhecida pode resultar em zero trechos de fala.

## Configuração local

Adicione ao ambiente do worker:

```sh
WHISPER_URL='http://127.0.0.1:8092'
WHISPER_MODEL='whisper-large-v3-turbo'
```

O endpoint esperado é POST /inference do [whisper.cpp](https://github.com/ggml-org/whisper.cpp/tree/master/examples/server), multipart com file WAV e response_format=verbose_json. O servidor deve responder com segments contendo text, start e end em segundos. A configuração padrão não instala nem inicia o Whisper; use um servidor local previamente instalado e atualize WHISPER_MODEL quando trocar o modelo. O serviço de referência já existente foi reutilizado sem alterar seu modelo ou interromper outros serviços.

Gemma via LM Studio continua responsável pelas descrições e EmbeddingGemma pelos embeddings. O áudio e as imagens não são enviados para serviços externos nesta configuração. Todos os endpoints são configuráveis; mantenha-os na rede local.

## Busca e navegação

POST /api/v1/search continua aceitando query e mode text/hybrid. Cada resultado temporal inclui match com kind (visual/speech), start_ms, end_ms e text. A busca textual funciona sem inferência. A híbrida combina rankings lexical e vetorial, respeitando a versão do embedding, e retorna no máximo três trechos por arquivo. Os scores não são confiança nem garantia de relevância.

GET /api/v1/assets/{id}/segments retorna os trechos ordenados por tempo, sem embeddings. Na interface, clicar no resultado abre a prévia no instante correspondente. O painel de detalhes permite filtrar imagem/fala e saltar por trechos da transcrição. Trechos visuais representam uma observação pontual (end_ms = start_ms + 1), não a duração de uma cena.

## Reanálise e recuperação

POST /api/v1/assets/{id}/reanalyze aceita vídeos com tarefa concluída ou falha. Incrementa a geração da tarefa e invalida leases anteriores. Chamadas enquanto a tarefa está na fila ou executando retornam 409.

A análise anterior permanece pesquisável até a nova transação terminar. Uma conclusão antiga não pode sobrescrever uma geração nova. Retentativa idêntica da conclusão é idempotente; uma reanálise bem-sucedida substitui os trechos antigos, sem duplicá-los. O worker mantém heartbeat durante extração e inferência.

Não existe checkpoint de respostas dos modelos nesta fatia: após uma falha, a tentativa pode repetir transcrição e descrições, embora as imagens extraídas e o derivado de reprodução sejam reutilizados. Logs de progresso registram índices e tempos, sem textos da transcrição.

## Testes

- cargo test --locked: amostragem limitada, timestamps absolutos, filtragem de silêncio e rejeição de respostas sem tempos.
- python3 scripts/test.py: banco isolado, validação de intervalos/vetores, busca exata por fala, substituição atômica, geração antiga rejeitada e manutenção dos trechos anteriores após erro.
- node tests/timeline-browser.cjs: requer um vídeo temporal processado com fala após um minuto; testa busca real por palavras transcritas, abertura no timestamp, salto pela transcrição, filtros e layout móvel.
- O teste real também compara novamente o download completo com o hash original. Arquivo, transcrição e evidências pessoais permanecem em .local, fora do Git.
