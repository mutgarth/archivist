# Infraestrutura e implantação

> Direção de produto e plano inicial. Para o estado implementado da primeira fatia, consulte [execução](12-executar.md) e [validação](13-validacao-primeira-fatia.md).

Plano técnico; implantação ainda não implementada ou validada.

## Dependências do serviço

Diretório de mídia persistente, PostgreSQL com pgvector e acesso aos motores de análise. O servidor é o único escritor do banco. Worker com acesso de leitura à mídia e espaço temporário limitado; GPU e memória dimensionadas conforme os modelos selecionados.

O produto não depende de marca de NAS ou quantidade fixa de usuários. Requisitos mínimos de CPU, RAM, GPU e armazenamento devem ser publicados após testes reproduzíveis.

## Implantação de referência

Servidor em container, volumes persistentes para mídia e banco, configuração e segredos externos ao Git. O worker pode executar em outro computador Linux. SMB é uma opção para copiar lotes; o usuário confirma o término antes da ingestão.

TrueNAS é uma opção de host, seguindo [Custom Apps](https://apps.truenas.com/managing-apps/installing-custom-apps/) conforme a versão instalada. Separar painel da aplicação e administração do armazenamento.

Acesso por LAN ou rede privada, opcionalmente [Tailscale](https://tailscale.com/docs/integrations/truenas). Contas individuais e autorização continuam obrigatórias. Não expor banco ou inferência diretamente à internet.

## Preparação do ambiente

- Verificar capacidade livre, permissões e saúde do armazenamento.
- Preservar dados existentes; instalar o serviço não exige formatar discos.
- Definir diretórios de entrada, originais e derivados.
- Dimensionar temporários e limitar ocupação e execução das análises.
- Validar leitura da mídia e comunicação autenticada entre worker e servidor.
- Documentar versões efetivamente testadas e procedimentos de atualização.

## Backup e recuperação

Backup independente de originais e catálogo/correções, com retenção e frequência definidas pelo operador. Redundância e snapshots não substituem backup.

Testar restauração em ambiente isolado, conferindo hashes, referências e correções humanas. Nenhum backup é configurado por esta documentação.

## Entrega instalável

A versão executável deverá fornecer instruções para obter uma versão identificada, configurar diretórios e credenciais, iniciar o serviço, importar exemplos não pessoais e testar busca, prévia e download. Esses comandos ainda dependem da implementação; não há instalação funcional disponível nesta etapa.
