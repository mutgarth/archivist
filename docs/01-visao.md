# Visão e requisitos

> Direção de produto e plano inicial. Para o estado implementado da primeira fatia, consulte [execução](12-executar.md) e [validação](13-validacao-primeira-fatia.md).

## Problema e fluxo

Usuários precisam encontrar e reutilizar arquivos em um catálogo compartilhado. O sistema oferece contas individuais e acesso multiusuário, sem fixar quantidade de pessoas. Capacidade e concorrência serão medidas antes de declarar limites suportados.

1. Copiar um lote pelo SMB, diretamente do computador ou cartão ao diretório de entrada do NAS.
2. Confirmar no painel que a cópia terminou. Arquivo presente ou tamanho estável não prova conclusão.
3. Validar o lote e registrar os assets; apresentar erros por arquivo.
4. Manter o original estável, gerar prévias e enfileirar análises.
5. Buscar, conferir evidências, corrigir metadados, reunir coleções e baixar originais.

Categorias múltiplas e coleções são metadados; não movem arquivos e não quebram referências de editores externos. Uma classificação pode sugerir destino apenas na entrada, com confirmação e reversibilidade antes da estabilização do caminho.

## Requisitos aceitos

- Catálogo, navegação e download continuam úteis com IA indisponível; busca semântica pode ficar indisponível com status explícito.
- Busca textual e vetorial desde a primeira rodada funcional.
- Descrições, transcrições e categorias têm correções humanas preservadas.
- Proveniência de modelos, versões e análises; resultados antigos não substituem gerações recentes.
- Contas individuais mesmo dentro da Tailnet; identidade de worker separada.
- Originais e catálogo/correções precisam de backup independente, ainda não configurado.
- Aplicação depende de diretório de mídia e banco, não de TrueNAS.

## Telas planejadas

| Tela | Responsabilidade |
| --- | --- |
| Archivist | Busca, filtros, estados vazios/erros e prévias |
| Detalhe | Descrição e transcrição editáveis, categorias, análises e original |
| Importações | Confirmação da cópia, progresso e falhas por arquivo |
| Coleções | Seleções de assets para projetos sem mover originais |

Navegação por teclado, rótulos acessíveis e status não dependentes apenas de cor fazem parte da UI.

## Limites

Edição fica permanentemente fora: nenhum corte, timeline ou render de edição. Prévias técnicas são permitidas. Buscar trechos e sugerir assets para roteiro é gerenciamento, mas vem depois do fluxo mínimo.

Adiados: S3, upload web retomável, HA, múltiplas GPUs, plugins e automações de publicação. Não são necessários para validar o fluxo inicial do produto.
