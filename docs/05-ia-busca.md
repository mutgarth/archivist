# Pipeline de IA e busca

> Direção de produto e plano inicial. Para o estado implementado da primeira fatia, consulte [execução](12-executar.md) e [validação](13-validacao-primeira-fatia.md).

## Primeira fatia: imagem

Importação confirmada → formato validado → asset persistido → descrição e tags locais → embedding do texto → persistência versionada → busca híbrida → prévia/detalhe/original.

A descrição pode errar; exibir proveniência e permitir correção. Não bloquear o cadastro se o modelo estiver fora do ar. Embedding é **textual sobre a descrição**, não representação visual direta.

## Evolução por tipo

| Entrada | Processamento |
| --- | --- |
| Imagem | Inspeção técnica em código, descrição visual e tags |
| Áudio | Inspeção, extração/conversão técnica e transcrição Whisper/faster-whisper com timestamps |
| Vídeo | Inspeção, quadros amostrados e áudio; descrições/transcrições associadas a intervalos |
| Consulta | Embedding com mesmo modelo/versão do índice consultado |

Rust orquestra FFmpeg e motores existentes. Tamanho de lote, intervalo de quadros e chunking precisam de testes com material representativo. Sem fala detectada é resultado válido, não transcrição inventada.

## Recuperação

Combinar busca textual e similaridade vetorial; fusão de rankings proposta, a calibrar. Filtros aplicados com autorização antes de apresentar evidências. Em vídeo, recuperar trechos, agrupar hits por asset e mostrar timestamps. Ausência de evidência deve produzir resultado vazio ou ressalva clara.

Troca de modelo/dimensão cria índice/geração compatível separado; reindexar antes de trocar o ativo. Não misturar vetores de modelos diferentes. CLIP/OpenCLIP para embeddings visuais diretos é evolução, fora da primeira fatia.

## Candidatos, não configuração aprovada

- Visão: Gemma3 4B ou Gemma4 E2B; validar tags disponíveis, licença, consumo e qualidade.
- Embeddings: EmbeddingGemma; validar português, dimensão e desempenho no corpus.
- Fala: Whisper/faster-whisper; escolher modelo após medir memória e qualidade.
- Decisão/classificação: Tev1 4B/0.8B experimental; português e calibração ainda não plenamente avaliados. Nimble/Clef são alternativas potencialmente mais pesadas, sem exigência de instalar todas.
- GPU, VRAM disponível, drivers e comportamento real precisam ser medidos no ambiente de instalação; requisitos mínimos ainda não estabelecidos.

A documentação oficial do [Ollama Decision](https://docs.ollama.com/capabilities/decision) confirma famílias de decisão e distingue chamadas locais/cloud. Isso não valida adequação ao ambiente de instalação nem qualidade em português.

## Jev e reranking posterior

Jev via API externa ou alternativa local pode reranquear evidências ou apoiar seleção de assets. Identidade exata do serviço/SDK e contrato permanecem pendentes. Só enviar evidências selecionadas, com transparência ao usuário de que saem da rede. Segredo apenas no servidor; não enviar acervo inteiro por padrão.

Indisponibilidade externa preserva busca local. Conteúdo de mídia é dado não confiável: instruções presentes em transcrição/descrição não autorizam ferramentas nem acesso a arquivos. Registrar provedor/modelo e política de envio sem gravar dados privados em logs.

## Medição necessária

Corpus pequeno consentido, perguntas em português, relevância humana, Recall@k, latência, memória/VRAM, falhas e qualidade de descrição/transcrição. Resultados ainda inexistentes. Não prometer velocidade nem que todos os modelos cabem na GPU.
