// Portuguese (European/Brazilian neutral, PT-PT base) — full translation bundle,
// mirroring the structure of en.ts and falling back to English for anything missing.
import type { LocaleBundle } from './types.js';

export const pt: LocaleBundle = {
  meta: { code: 'pt', htmlLang: 'pt', autonym: 'Português', englishName: 'Portuguese', dir: 'ltr' },
  categories: {
    organize: { label: 'Organizar', descriptor: 'Reordenar, juntar, dividir, rodar' },
    convert: { label: 'Converter', descriptor: 'Alternar entre PDF, imagem e texto' },
    edit: { label: 'Editar', descriptor: 'Carimbar, anotar e assinar páginas' },
    optimize: { label: 'Otimizar', descriptor: 'Tornar os ficheiros mais pequenos' },
    forms: { label: 'Formulários', descriptor: 'Detetar e preencher campos de formulário' },
    security: { label: 'Segurança e privacidade', descriptor: 'Remover o que não quer partilhar' },
  },
  layout: {
    skip: 'Saltar para o conteúdo',
    homeAria: 'Início do Unfleece',
    navAria: 'Principal',
    allTools: 'Todas as ferramentas',
    howItWorks: 'Como funciona',
    privacy: 'Privacidade',
    footerBlurb: 'Ferramentas PDF gratuitas e privadas. Os seus ficheiros nunca saem do navegador.',
    tools: 'Ferramentas',
    project: 'Projeto',
    trust: 'Confiança',
    github: 'GitHub ↗',
    license: 'AGPL-3.0',
    reportIssue: 'Reportar um problema ↗',
    noUpload: 'Sem envio de ficheiros',
    noAccount: 'Sem conta',
    noTracking: 'Sem rastreio',
    language: 'Idioma',
  },
  toolPage: {
    breadcrumbHome: 'Início',
    breadcrumbTools: 'Ferramentas',
    privateWorkspace: 'Espaço de trabalho privado',
    nothingUploaded: 'Nenhum ficheiro enviado',
    howEyebrow: 'Como funciona',
    howHeading: (toolName) => `Como ${toolName.toLowerCase()}`,
    steps: [
      { title: 'Adicione o seu ficheiro', body: 'Arraste-o para aqui ou clique para escolher. Fica no seu dispositivo.' },
      { title: 'Defina as opções', body: 'Ajuste as definições, ou use simplesmente as predefinições sensatas.' },
      { title: 'Transfira o resultado', body: 'Um clique. Foi criado aqui mesmo, no seu navegador.' },
    ],
    differenceEyebrow: 'A diferença honesta',
    differenceHeading: 'Porquê o Unfleece em vez de fleeceware',
    usBullets: ['Funciona no seu navegador — sem envio de ficheiros', 'Gratuito, sem conta, sem cartão', 'Código aberto (AGPL-3.0)', 'Sem cobrança por tamanho de ficheiro'],
    themHeading: 'Site típico de fleeceware',
    themBullets: ['Envia o seu ficheiro para um servidor', 'Teste de $1 → ~$50 / semana', 'Fechado e opaco', 'Bloqueia os resultados atrás de pagamento'],
    questionsEyebrow: 'Perguntas',
    questionsHeading: 'Perguntas frequentes',
    relatedEyebrow: 'Continue',
    relatedHeading: 'Ferramentas relacionadas',
    faqs: (toolName) => [
      {
        q: `${toolName} é mesmo gratuito?`,
        a: 'Sim — gratuito, para sempre, sem conta e sem aquele período de teste que se vira contra si. Não há fatura de servidor a recuperar, porque nada é enviado.',
      },
      {
        q: 'Os meus ficheiros são enviados para algum lado?',
        a: `Não. O navegador lê o ficheiro para a memória e executa o ${toolName} no seu dispositivo. Abra o separador Rede e observe — não verá qualquer pedido de envio.`,
      },
      {
        q: 'Existe um limite de tamanho ou de páginas?',
        a: 'Não há limite rígido. O único teto é a memória do navegador, por isso ficheiros muito grandes podem ficar lentos. Avisamo-lo antes de isso se tornar um problema.',
      },
      {
        q: 'Funciona offline?',
        a: 'Depois de a página carregar, sim. Pode desligar o Wi-Fi e continua a funcionar — a melhor prova de que nada sai do seu dispositivo.',
      },
    ],
  },
  home: {
    heroLine1: 'Ferramentas PDF gratuitas e privadas.',
    heroLine2: 'Os seus ficheiros nunca saem do navegador.',
    subhead:
      'Junte, divida, converta, assine e comprima — todas as ferramentas funcionam inteiramente no seu dispositivo. Gratuito para sempre, sem conta, sem envios. Código aberto — e honestos quanto a isso.',
    browseCta: 'Ver ferramentas',
    howCta: 'Como funciona',
    proof: [
      { title: 'Privado por princípio', body: 'Os ficheiros são processados localmente e nunca enviados.', linkLabel: 'Confirme no separador Rede.' },
      { title: 'Gratuito para sempre', body: 'Sem períodos de teste, sem subscrições, sem cobranças por tamanho de ficheiro. Não há nada para vender a mais.' },
      { title: 'Rápido e aberto', body: 'Funciona localmente através de WebAssembly. Totalmente de código aberto (AGPL-3.0).' },
    ],
    toolsEyebrow: 'Todas as ferramentas',
    pickHeading: 'Escolha uma ferramenta — abre, executa-a, transfere.',
    pickBody: 'Sem inscrição. Sem fila de espera. Tudo acontece neste separador.',
    proveEyebrow: 'Não confie em nós — verifique',
    proveHeading: 'Funciona em 10 segundos, e pode provar que é privado.',
    proveBody:
      'Nenhum ficheiro toca alguma vez num servidor, por isso não há nada a cobrar-lhe e nada que possa vazar. Gratuito é apenas a predefinição honesta.',
    proveSteps: [
      { strong: 'Abra qualquer ferramenta', rest: ' e prima F12 → separador Rede.' },
      { strong: 'Execute-a num ficheiro.', rest: ' Observe a lista de pedidos.' },
      { strong: 'Veja que nada é enviado.', rest: ' Desligue o Wi-Fi — continua a funcionar.' },
    ],
  },
  tools: {
    merge: {
      name: 'Juntar PDF',
      tagline: 'Combine vários PDF num só documento.',
      description: 'Combine vários ficheiros PDF num único documento, pela ordem em que os adiciona.',
    },
    split: {
      name: 'Dividir PDF',
      tagline: 'Divida um PDF em vários ficheiros.',
      description: 'Divida um PDF em ficheiros separados — um por página, ou por intervalos de páginas personalizados.',
    },
    'extract-pages': {
      name: 'Extrair páginas',
      tagline: 'Retire apenas as páginas de que precisa.',
      description: 'Crie um novo PDF apenas com as páginas que escolher.',
    },
    'remove-pages': {
      name: 'Remover páginas',
      tagline: 'Apague as páginas que não quer.',
      description: 'Remova as páginas que selecionar e mantenha as restantes.',
    },
    reorder: {
      name: 'Reordenar páginas',
      tagline: 'Coloque as páginas por uma nova ordem.',
      description: 'Reordene as páginas de um PDF para uma nova sequência.',
    },
    rotate: {
      name: 'Rodar PDF',
      tagline: 'Endireite as páginas que estão tortas.',
      description: 'Rode todas as páginas ou apenas as selecionadas em 90°, 180° ou 270°. Sem perda de qualidade.',
    },
    'n-up': {
      name: 'N páginas por folha',
      tagline: 'Imprima várias páginas por folha.',
      description: 'Coloque várias páginas de origem em cada folha A4 — ótimo para folhetos.',
    },
    booklet: {
      name: 'Livreto',
      tagline: 'Disponha as páginas para imprimir um livreto dobrado.',
      description: 'Reordene e disponha as páginas duas a duas em folhas horizontais, de modo a dobrarem-se num livreto.',
    },
    'compare-pdf': {
      name: 'Comparar PDF',
      tagline: 'Encontre diferenças visuais entre dois PDF.',
      description: 'Renderize dois PDF localmente e gere um relatório visual das diferenças, página a página.',
    },
    'images-to-pdf': {
      name: 'Imagens → PDF',
      tagline: 'Transforme JPG e PNG num PDF.',
      description: 'Transforme uma ou mais imagens JPG/PNG num PDF, uma imagem por página.',
    },
    'pdf-to-jpg': {
      name: 'PDF → JPG',
      tagline: 'Guarde cada página como JPG.',
      description: 'Renderize cada página do PDF para uma imagem JPG e transfira-as num ZIP.',
    },
    'pdf-to-png': {
      name: 'PDF → PNG',
      tagline: 'Guarde cada página como PNG.',
      description: 'Renderize cada página do PDF para uma imagem PNG sem perdas e transfira-as num ZIP.',
    },
    'pdf-to-text': {
      name: 'PDF → Texto',
      tagline: 'Extraia o texto selecionável de um PDF.',
      description: 'Extraia o texto selecionável de um PDF. (PDF digitalizados ou de imagem não têm texto para extrair.)',
    },
    'pdf-to-epub': {
      name: 'PDF → EPUB',
      tagline: 'Crie um EPUB a partir de texto selecionável de PDF.',
      description: 'Crie um EPUB de fluxo ajustável a partir de texto selecionável de PDF, ou um EPUB de esquema fixo a partir de páginas renderizadas, para digitalizações e esquemas complexos.',
    },
    'pdf-to-docx': {
      name: 'Extrair para Word',
      tagline: 'Guarde texto selecionável de PDF como DOCX.',
      description: 'Extraia texto selecionável para um documento Word simples. Preserva o texto, não o esquema original.',
    },
    'pdf-to-excel': {
      name: 'Extrair para Excel',
      tagline: 'Transforme linhas selecionáveis numa folha de cálculo.',
      description: 'Extração, na medida do possível, de linhas de texto selecionável para um livro Excel. As digitalizações precisam primeiro de OCR.',
    },
    'pdf-to-pptx': {
      name: 'Extrair para PowerPoint',
      tagline: 'Transforme o texto do PDF em diapositivos simples.',
      description: 'Extraia o texto selecionável do PDF para uma apresentação PowerPoint simples, um diapositivo por página. Não recria o esquema original.',
    },
    'ocr-pdf': {
      name: 'PDF pesquisável com OCR',
      tagline: 'Adicione texto selecionável a PDF digitalizados.',
      description: 'Execute o OCR do Tesseract localmente e adicione uma camada de texto pesquisável invisível sobre cada página. Inclui o modelo de inglês.',
    },
    pdfa: {
      name: 'Exportar PDF/A',
      tagline: 'Crie uma cópia PDF/A-2b para arquivo.',
      description: 'Renderize as páginas localmente e reconstrua-as como um ficheiro PDF/A-2b visual com o motor Rust krilla. O texto passa a não ser selecionável.',
    },
    'image-convert': {
      name: 'Converter imagem',
      tagline: 'Alterne entre PNG, JPG, WebP, AVIF, JPEG XL e entradas tipo TIFF.',
      description: 'Converta imagens do navegador e formatos como TIFF, PSD, BMP, GIF e ICO em PNG, JPG, WebP, AVIF ou JPEG XL, totalmente no seu navegador.',
    },
    'html-to-pdf': {
      name: 'HTML / Markdown → PDF',
      tagline: 'Transforme um documento de texto num PDF limpo.',
      description: 'Converta um ficheiro HTML, Markdown ou de texto simples num PDF legível. É focado em texto, não é um motor de composição de páginas de navegador.',
    },
    'page-numbers': {
      name: 'Números de página',
      tagline: 'Carimbe números de página num PDF.',
      description: 'Adicione números de página com formato e posição personalizáveis.',
    },
    bates: {
      name: 'Numeração Bates',
      tagline: 'Adicione números Bates ao estilo jurídico.',
      description: 'Carimbe números Bates sequenciais em cada página, com prefixo, número com zeros à esquerda e posição.',
    },
    watermark: {
      name: 'Marca de água',
      tagline: 'Adicione uma marca de água de texto a todas as páginas.',
      description: 'Carimbe uma marca de água de texto na diagonal sobre todas as páginas.',
    },
    crop: {
      name: 'Recortar',
      tagline: 'Corte as margens das suas páginas.',
      description: 'Corte as margens de todas as páginas definindo a caixa de recorte.',
    },
    'auto-crop': {
      name: 'Recorte automático de margens',
      tagline: 'Detete e corte as margens brancas das páginas.',
      description: 'Renderize cada página localmente, detete os limites do conteúdo não branco e defina caixas de recorte por página, com algum espaçamento.',
    },
    metadata: {
      name: 'Editar metadados',
      tagline: 'Altere o título, o autor e mais.',
      description: 'Veja e substitua os metadados do documento (título, autor, assunto, palavras-chave).',
    },
    sign: {
      name: 'Assinar',
      tagline: 'Desenhe uma assinatura e coloque-a.',
      description: 'Desenhe uma assinatura e coloque-a na página. Marca eletrónica, não uma assinatura criptográfica.',
    },
    optimize: {
      name: 'Otimizar',
      tagline: 'Reduza o ficheiro sem perda de qualidade.',
      description: 'Reduza um PDF sem perdas, removendo objetos não utilizados e comprimindo fluxos de dados — o texto continua selecionável.',
    },
    compress: {
      name: 'Comprimir',
      tagline: 'Reduza PDF cheios de imagens com o Ghostscript.',
      description: 'Use o Ghostscript pdfwrite localmente para compressão com perdas de PDF. O texto continua selecionável quando o Ghostscript consegue reescrever o ficheiro; a alternativa em rasterização torna o texto não selecionável.',
    },
    'compress-image': {
      name: 'Comprimir imagem',
      tagline: 'Reduza PNG, JPG, WebP, AVIF, JPEG XL e entradas tipo TIFF.',
      description: 'Recodifique e, opcionalmente, redimensione imagens do navegador, além de TIFF, PSD, BMP, GIF e ICO, localmente. Escolha WebP, AVIF, JPEG XL ou JPG para um resultado com perdas mais pequeno.',
    },
    'fill-form': {
      name: 'Preencher formulário',
      tagline: 'Preencha os campos de formulário detetados.',
      description: 'Detete campos AcroForm e preencha-os no seu navegador.',
    },
    flatten: {
      name: 'Achatar',
      tagline: 'Fixe os campos de formulário na página.',
      description: 'Achate os campos de formulário e as anotações, transformando-os em conteúdo estático da página.',
    },
    'remove-metadata': {
      name: 'Remover metadados',
      tagline: 'Elimine informação oculta antes de partilhar.',
      description: 'Remova os metadados do documento (título, autor, produtor, …) para que não sejam partilhados por acidente.',
    },
    protect: {
      name: 'Proteger PDF',
      tagline: 'Adicione uma palavra-passe para abrir um PDF.',
      description: 'Encripte um PDF localmente com uma palavra-passe de utilizador e, opcionalmente, palavra-passe de proprietário/permissões.',
    },
    unlock: {
      name: 'Desbloquear PDF',
      tagline: 'Remova a encriptação por palavra-passe quando souber a palavra-passe.',
      description: 'Abra um PDF encriptado com a sua palavra-passe e guarde localmente uma cópia desbloqueada.',
    },
    sanitize: {
      name: 'Limpar PDF',
      tagline: 'Remova extras ativos e ocultos do PDF.',
      description: 'Remova metadados, scripts do documento, ficheiros incorporados, ações de página e, opcionalmente, anotações/formulários antes de partilhar.',
    },
    redact: {
      name: 'Censurar PDF',
      tagline: 'Grave as caixas de censura no documento.',
      description: 'Cubra as áreas selecionadas e reconstrua as páginas como páginas PDF só de imagem, para que não fique texto oculto por baixo.',
    },
  },
};

export default pt;
