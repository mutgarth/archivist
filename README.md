# Archivist

Gerenciador de mídia auto-hospedado para múltiplos usuários, com catálogo compartilhado, organização e busca semântica local. O número de usuários não é fixado pelo produto; limites de capacidade dependem de validação.

**Estado: documentação inicial; aplicação, API, banco e infraestrutura ainda não implementados.** Ainda não há versão executável para baixar e testar. A entrega funcional deverá incluir instruções reproduzíveis de instalação, configuração e teste com dados de exemplo.

O Archivist deverá armazenar referências aos originais, catalogar, organizar em coleções, descrever imagens, transcrever áudio/vídeo, pesquisar texto e vetores, mostrar prévias e permitir baixar originais. A edição de vídeo acontece em ferramentas externas: não haverá timeline, cortes ou renderização de edição.

## Comece aqui

- [Índice da documentação](docs/README.md)
- [Visão, requisitos e limites](docs/01-visao.md)
- [Arquitetura](docs/02-arquitetura.md)
- [Roteiro de entrega](docs/07-entrega.md)
- [Decisões e pendências](docs/08-decisoes.md)

Primeira fatia planejada: **importar imagem → descrição local → embedding textual → persistência → busca → visualização**. Busca vetorial faz parte dessa primeira fatia.

## Direção técnica

Rust modular, dois executáveis planejados (servidor e worker), Axum/Tokio e PostgreSQL com pgvector como baseline proposto. Fila persistente no banco, sem Redis. Rust coordena ferramentas de mídia e inferência existentes. O servidor é o único escritor do banco; o worker recebe tarefas por API interna autenticada.

Originais em diretório de mídia; catálogo e correções no banco. A aplicação deve ser portátil, sem dependência de APIs específicas do TrueNAS.

## Conteúdo deste repositório

Apenas documentação e configuração de edição/Git. Não há backend, containers executáveis, migrações, modelos baixados ou testes de aplicação. O [plano de validação](docs/09-validacao.md) descreve testes futuros, sem alegar execução.

Não incluir mídias pessoais, credenciais, endereços privados ou dumps. Configurações reais ficam fora do Git. A licença de distribuição permanece a decidir; repositório público não implica uma licença de código aberto.
