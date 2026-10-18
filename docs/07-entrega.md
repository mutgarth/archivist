# Roteiro de entrega do serviço

> Direção de produto e plano inicial. Para o estado implementado da primeira fatia, consulte [execução](12-executar.md) e [validação](13-validacao-primeira-fatia.md).

Etapas futuras em ordem; o guia de execução registra a fatia disponível.

## 0. Documentar

Concluir visão, arquitetura, contrato inicial e pendências; versionar em Git. Não criar backend vazio nem infraestrutura que pareça operacional.

## 1. Preparar hardware e armazenamento

Preparar diretórios persistentes e permissões conforme a [implantação](06-infraestrutura.md). Validar acesso e cópia por clientes independentes com arquivos descartáveis, sem exigir uma plataforma específica de NAS.

## 2. Fechar contratos mínimos

Manter acesso pela rede privada e credencial interna do worker; confirmar PostgreSQL+pgvector, modelos, limites e entrega de derivados. Definir OpenAPI, migração mínima e fixtures sintéticas. Organizar módulos Rust conforme as dependências reais; não criar microserviços.

## 3. Entregar a primeira fatia completa

1. Copiar imagens consentidas pelo SMB e confirmar término no painel.
2. Validar caminhos e integridade; persistir assets de modo idempotente.
3. Reservar tarefa no worker, produzir descrição local e embedding textual.
4. Persistir resultado e vetor compatível, preservando correções humanas.
5. Pesquisar por texto e semântica com filtros; visualizar prévia e baixar original.
6. Desligar IA: comprovar catálogo e download disponíveis.
7. Reiniciar worker no meio da tarefa: comprovar retomada sem duplicação.

Saída exigida: demonstração reproduzível da busca vetorial de imagem ponta a ponta, com evidência real. Busca apenas por nome não encerra esta etapa.

## 4. Adicionar áudio e vídeo

Transcrição com timestamps, quadros amostrados, trechos e hits agrupados; prévias técnicas sem ferramentas de edição. Medir custo e qualidade antes de aumentar lotes.

## 5. Facilitar reutilização

Coleções, seleção para roteiro, sugestões com evidências e reranking local ou Jev opcional, com envio externo explícito.

## 6. Validar recuperação e distribuição

Restaurar originais e banco e validar referências e correções. Testar acesso com múltiplas contas e concorrência. Entregar instruções reproduzíveis para baixar uma versão, configurar, executar e testar o serviço com dados de exemplo não pessoais.
