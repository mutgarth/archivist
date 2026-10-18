# Vídeos locais e futura montagem do TrueNAS

## O que esta fatia faz

Importa MP4, MOV e M4V a partir de caminhos relativos em MEDIA_ROOT. Mantém o arquivo original byte a byte, incluindo áudio, faixas de dados e metadados embutidos. O SHA-256 é calculado durante a cópia e conferido com uma segunda leitura do snapshot antes da publicação. O catálogo expõe sha256 e media_metadata (streams e formato do FFprobe).

Limites: 20 arquivos por lote; imagens até 20 MB; vídeos até 64 GiB, 24 horas e dimensões de até 8192 pixels. A cópia usa buffers de 1 MiB. O download e a reprodução usam streaming HTTP com Range, sem carregar o arquivo inteiro em memória. A importação ainda espera a cópia/verificação na própria requisição: lotes grandes podem demorar e não têm progresso percentual ou retomada interna. Repita com a mesma Idempotency-Key após uma resposta perdida.

A primeira imagem de prévia é extraída perto do início. O worker verifica novamente o hash e cria um MP4 de reprodução com H.264/AAC, limitado a 1280 × 720, CRF 25 e faststart. Esse derivado é comprimido; **nunca substitui o original**. HDR não tem tone mapping calibrado nesta fatia; use o original para avaliação de cor ou edição. O derivado respeita a rotação reconhecida pelo FFmpeg; metadados técnicos do catálogo descrevem o original.

A descrição e o embedding usam somente a imagem de prévia, com essa limitação identificada na interface e pipeline_version video-poster-v1. Não há transcrição, indexação temporal ou descrição de todas as cenas. A reprodução fica disponível mesmo se a IA estiver offline, depois de terminar a conversão. Enquanto isso, o original já pode ser baixado.

## Dependências

Linux, FFmpeg e FFprobe com libx264/AAC e filtro scale com reset_sar. Instale pela distribuição ou use executáveis próprios nos campos FFMPEG_BIN e FFPROBE_BIN de .env e .env.worker. O processamento usa descritores via /proc e o comando timeout do GNU coreutils. A conversão limita threads de CPU; não usa a GPU de inferência.

O arquivo pessoal de teste, as prévias, os logs e os resultados ficam em .local, ignorado pelo Git. Os testes públicos geram vídeos sintéticos via FFmpeg.

## Organização atual

- MEDIA_ROOT/videos/: entrada copiada pelo operador.
- MEDIA_ROOT/.archivist/originals/: originais verificados, nomeados pelo hash.
- MEDIA_ROOT/.archivist/previews/: thumbnails JPEG.
- MEDIA_ROOT/.archivist/playback/: derivados MP4 de reprodução.
- MEDIA_ROOT/.archivist-volume: identificação do volume.

O arquivo de entrada é mantido: há duas cópias locais nesta fase. Não há remoção automática, backup independente nem autorização automática para apagar o arquivo do aparelho. Falhas podem deixar snapshots sem referência ou temporários; a limpeza automática ainda não existe. Reserve espaço para entrada, original e derivado.

## Preparar uma futura montagem

MEDIA_VOLUME_ID deve corresponder ao conteúdo de .archivist-volume em MEDIA_ROOT, tanto no servidor quanto no worker. scripts/configure.py cria essa identidade no armazenamento local. Em instalações anteriores, crie a identidade uma vez, explicitamente, no volume correto e configure ambos os processos. O serviço nunca cria o marcador durante uma requisição.

O marcador ausente ou diferente faz a readiness retornar 503 e bloqueia novas importações. Isso evita usar o diretório vazio que fica sob uma montagem desmontada. Não copie o marcador para esse diretório de fallback. O marcador identifica o volume; não é um mecanismo de autenticação nem substitui monitoramento do sistema de arquivos.

Quando o TrueNAS estiver disponível:

1. Pare servidor e worker.
2. Monte o dataset NFS/SMB em um caminho dedicado no Ubuntu e confira permissões e espaço.
3. Copie todo o MEDIA_ROOT, incluindo .archivist e .archivist-volume, preservando o original local até conferir hashes e backup.
4. Atualize MEDIA_ROOT nos dois arquivos de ambiente, mantendo MEDIA_VOLUME_ID. Os caminhos no banco são relativos, portanto não exigem reescrita.
5. Reinicie os processos; valide /health/ready, hashes, reprodução e download. Configure a dependência da montagem no gerenciador de serviços.

Não existe conexão TrueNAS ativa, instalação do TrueNAS ou alteração de discos nesta entrega. Se o TrueNAS vier a substituir o Ubuntu no mesmo servidor físico, será necessário planejar onde executar a aplicação e como migrar os dados antes de alterar o sistema operacional.

## Verificação

cargo test --locked e scripts/test.py verificam regressões, importação de vídeo sintético, metadados, igualdade do hash original, Range, volume ausente/incorreto e geração do derivado pelo worker com IA offline. O teste de navegador de vídeo cobre reprodução e seek. Validação com um arquivo real é complementar; não representa compatibilidade universal com codecs e câmeras.
