// Spanish — full translation bundle. Falls back to English for anything missing.
import type { LocaleBundle } from './types.js';

export const es: LocaleBundle = {
  meta: { code: 'es', htmlLang: 'es', autonym: 'Español', englishName: 'Spanish', dir: 'ltr' },
  categories: {
    organize: { label: 'Organizar', descriptor: 'Reordena, une, divide, gira' },
    convert: { label: 'Convertir', descriptor: 'Pasa entre PDF, imagen y texto' },
    edit: { label: 'Editar', descriptor: 'Sella, marca y firma páginas' },
    optimize: { label: 'Optimizar', descriptor: 'Haz los archivos más pequeños' },
    forms: { label: 'Formularios', descriptor: 'Detecta y rellena campos de formulario' },
    security: { label: 'Seguridad y privacidad', descriptor: 'Elimina lo que no quieras compartir' },
  },
  layout: {
    skip: 'Saltar al contenido',
    homeAria: 'Inicio de Unfleece',
    navAria: 'Principal',
    allTools: 'Todas las herramientas',
    howItWorks: 'Cómo funciona',
    privacy: 'Privacidad',
    footerBlurb: 'Herramientas PDF gratuitas y privadas. Tus archivos nunca salen de tu navegador.',
    tools: 'Herramientas',
    project: 'Proyecto',
    trust: 'Confianza',
    github: 'GitHub ↗',
    license: 'AGPL-3.0',
    reportIssue: 'Informar de un problema ↗',
    noUpload: 'Sin subida de archivos',
    noAccount: 'Sin cuenta',
    noTracking: 'Sin rastreo',
    language: 'Idioma',
  },
  toolPage: {
    breadcrumbHome: 'Inicio',
    breadcrumbTools: 'Herramientas',
    privateWorkspace: 'Espacio de trabajo privado',
    nothingUploaded: 'No se ha subido ningún archivo',
    howEyebrow: 'Cómo funciona',
    howHeading: (toolName) => `Cómo usar ${toolName}`,
    steps: [
      { title: 'Añade tu archivo', body: 'Arrástralo o haz clic para elegirlo. Se queda en tu dispositivo.' },
      { title: 'Ajusta las opciones', body: 'Modifica los ajustes, o simplemente usa los valores predeterminados, que son sensatos.' },
      { title: 'Descarga el resultado', body: 'Un clic. Se creó aquí mismo, en tu navegador.' },
    ],
    differenceEyebrow: 'La diferencia honesta',
    differenceHeading: 'Por qué Unfleece y no el fleeceware',
    usBullets: ['Funciona en tu navegador, sin subida de archivos', 'Gratis, sin cuenta, sin tarjeta', 'Código abierto (AGPL-3.0)', 'Sin muro de pago por tamaño de archivo'],
    themHeading: 'Sitio típico de fleeceware',
    themBullets: ['Sube tu archivo a un servidor', 'Prueba de $1 → ~$50 / semana', 'Cerrado y opaco', 'Bloquea los resultados tras un pago'],
    questionsEyebrow: 'Preguntas',
    questionsHeading: 'Preguntas frecuentes',
    relatedEyebrow: 'Sigue adelante',
    relatedHeading: 'Herramientas relacionadas',
    faqs: (toolName) => [
      {
        q: `¿${toolName} es realmente gratis?`,
        a: 'Sí: gratis, para siempre, sin cuenta y sin pruebas que luego te muerden. No hay factura de servidor que recuperar porque no se sube nada.',
      },
      {
        q: '¿Mis archivos se suben a algún sitio?',
        a: `No. Tu navegador lee el archivo en memoria y ejecuta ${toolName} en tu dispositivo. Abre la pestaña Red y observa: verás cero solicitudes de subida.`,
      },
      {
        q: '¿Hay un límite de tamaño de archivo o de páginas?',
        a: 'No hay un límite estricto. El único techo es la memoria de tu navegador, así que los archivos muy grandes pueden ir lentos. Te avisamos antes de que eso se convierta en un problema.',
      },
      {
        q: '¿Funciona sin conexión?',
        a: 'Una vez cargada la página, sí. Puedes apagar el Wi-Fi y seguirá funcionando: la prueba más rotunda de que nada sale de tu dispositivo.',
      },
    ],
  },
  home: {
    heroLine1: 'Herramientas PDF gratuitas y privadas.',
    heroLine2: 'Tus archivos nunca salen de tu navegador.',
    subhead:
      'Une, divide, convierte, firma y comprime: cada herramienta funciona por completo en tu dispositivo. Gratis para siempre, sin cuenta, sin subidas. Código abierto, y honestos al respecto.',
    browseCta: 'Ver herramientas',
    howCta: 'Cómo funciona',
    proof: [
      { title: 'Privado por diseño', body: 'Los archivos se procesan localmente y nunca se suben.', linkLabel: 'Compruébalo en tu pestaña Red.' },
      { title: 'Gratis para siempre', body: 'Sin pruebas, sin suscripciones, sin muros de pago por tamaño de archivo. No hay nada que venderte de más.' },
      { title: 'Rápido y abierto', body: 'Funciona localmente con WebAssembly. Totalmente de código abierto (AGPL-3.0).' },
    ],
    toolsEyebrow: 'Todas las herramientas',
    pickHeading: 'Elige una herramienta: se abre, la usas, descargas.',
    pickBody: 'Sin registro. Sin colas. Todo ocurre en esta pestaña.',
    proveEyebrow: 'No nos creas: compruébalo',
    proveHeading: 'Funciona en 10 segundos, y puedes demostrar que es privado.',
    proveBody:
      'Ningún archivo toca jamás un servidor, así que no hay nada que cobrarte ni nada que filtrar. Gratis es, sin más, lo honesto por defecto.',
    proveSteps: [
      { strong: 'Abre cualquier herramienta', rest: ' y pulsa F12 → pestaña Red.' },
      { strong: 'Úsala con un archivo.', rest: ' Observa la lista de solicitudes.' },
      { strong: 'No verás ninguna subida.', rest: ' Apaga el Wi-Fi: sigue funcionando.' },
    ],
  },
  tools: {
    merge: {
      name: 'Unir PDF',
      tagline: 'Combina varios PDF en un solo documento.',
      description: 'Combina varios archivos PDF en un único documento, en el orden en que los añades.',
    },
    split: {
      name: 'Dividir PDF',
      tagline: 'Divide un PDF en varios archivos.',
      description: 'Divide un PDF en archivos separados: uno por página o por rangos de páginas personalizados.',
    },
    'extract-pages': {
      name: 'Extraer páginas',
      tagline: 'Saca solo las páginas que necesitas.',
      description: 'Crea un nuevo PDF que contenga solo las páginas que elijas.',
    },
    'remove-pages': {
      name: 'Eliminar páginas',
      tagline: 'Borra las páginas que no quieres.',
      description: 'Elimina las páginas que selecciones y conserva el resto.',
    },
    reorder: {
      name: 'Reordenar páginas',
      tagline: 'Coloca las páginas en un nuevo orden.',
      description: 'Reordena las páginas de un PDF en una nueva secuencia.',
    },
    rotate: {
      name: 'Girar PDF',
      tagline: 'Pon las páginas del derecho.',
      description: 'Gira todas las páginas o las seleccionadas 90°, 180° o 270°. Sin pérdidas.',
    },
    'n-up': {
      name: 'N-up por hoja',
      tagline: 'Imprime varias páginas por hoja.',
      description: 'Coloca varias páginas de origen en cada hoja A4: ideal para folletos y apuntes.',
    },
    booklet: {
      name: 'Folleto',
      tagline: 'Impón las páginas para imprimir un folleto plegado.',
      description: 'Reordena y coloca las páginas de dos en dos en hojas horizontales para que se plieguen en un folleto.',
    },
    'compare-pdf': {
      name: 'Comparar PDF',
      tagline: 'Encuentra diferencias visuales entre dos PDF.',
      description: 'Renderiza dos PDF localmente y genera un informe de diferencias visuales página por página.',
    },
    'images-to-pdf': {
      name: 'Imágenes → PDF',
      tagline: 'Convierte JPG y PNG en un PDF.',
      description: 'Convierte una o varias imágenes JPG/PNG en un PDF, con una imagen por página.',
    },
    'pdf-to-jpg': {
      name: 'PDF → JPG',
      tagline: 'Guarda cada página como un JPG.',
      description: 'Renderiza cada página del PDF en una imagen JPG y descárgalas en un ZIP.',
    },
    'pdf-to-png': {
      name: 'PDF → PNG',
      tagline: 'Guarda cada página como un PNG.',
      description: 'Renderiza cada página del PDF en una imagen PNG sin pérdidas y descárgalas en un ZIP.',
    },
    'pdf-to-text': {
      name: 'PDF → Texto',
      tagline: 'Extrae el texto seleccionable de un PDF.',
      description: 'Extrae el texto seleccionable de un PDF. (Los PDF escaneados o de imagen no tienen texto que extraer.)',
    },
    'pdf-to-epub': {
      name: 'PDF → EPUB',
      tagline: 'Crea un EPUB a partir del texto seleccionable de un PDF.',
      description: 'Crea un EPUB de texto adaptable a partir del texto seleccionable del PDF, o un EPUB de diseño fijo a partir de las páginas renderizadas para escaneos y diseños complejos.',
    },
    'pdf-to-docx': {
      name: 'Extraer a Word',
      tagline: 'Guarda el texto seleccionable del PDF como DOCX.',
      description: 'Extrae el texto seleccionable en un documento de Word sencillo. Conserva el texto, no el diseño original.',
    },
    'pdf-to-excel': {
      name: 'Extraer a Excel',
      tagline: 'Convierte filas seleccionables en una hoja de cálculo.',
      description: 'Extracción aproximada de filas de texto seleccionable en un libro de Excel. Los escaneos necesitan OCR primero.',
    },
    'pdf-to-pptx': {
      name: 'Extraer a PowerPoint',
      tagline: 'Convierte el texto del PDF en diapositivas sencillas.',
      description: 'Extrae el texto seleccionable del PDF en una presentación de PowerPoint sencilla, una diapositiva por página. No recrea el diseño original.',
    },
    'ocr-pdf': {
      name: 'PDF consultable con OCR',
      tagline: 'Añade texto seleccionable a los PDF escaneados.',
      description: 'Ejecuta el OCR de Tesseract localmente y añade una capa de texto invisible y consultable sobre cada página. Incluye el modelo de inglés.',
    },
    pdfa: {
      name: 'Exportar a PDF/A',
      tagline: 'Crea una copia de archivo en PDF/A-2b.',
      description: 'Renderiza las páginas localmente y las reconstruye como un archivo PDF/A-2b visual con el motor Rust krilla. El texto deja de ser seleccionable.',
    },
    'image-convert': {
      name: 'Convertir imagen',
      tagline: 'Cambia entre PNG, JPG, WebP, AVIF, JPEG XL y entradas de tipo TIFF.',
      description: 'Convierte imágenes del navegador y formatos como TIFF, PSD, BMP, GIF e ICO a PNG, JPG, WebP, AVIF o JPEG XL, totalmente en tu navegador.',
    },
    'html-to-pdf': {
      name: 'HTML / Markdown → PDF',
      tagline: 'Convierte un documento de texto en un PDF limpio.',
      description: 'Convierte un archivo HTML, Markdown o de texto plano en un PDF legible. Está centrado en el texto, no es un motor de maquetación de navegador.',
    },
    'page-numbers': {
      name: 'Números de página',
      tagline: 'Sella números de página en un PDF.',
      description: 'Añade números de página con formato y posición personalizables.',
    },
    bates: {
      name: 'Numeración Bates',
      tagline: 'Añade números Bates de estilo legal.',
      description: 'Sella números Bates secuenciales en cada página con un prefijo, un número rellenado con ceros y una posición.',
    },
    watermark: {
      name: 'Marca de agua',
      tagline: 'Añade una marca de agua de texto a cada página.',
      description: 'Estampa una marca de agua de texto en diagonal sobre cada página.',
    },
    crop: {
      name: 'Recortar',
      tagline: 'Recorta los márgenes de tus páginas.',
      description: 'Recorta los márgenes de cada página ajustando el cuadro de recorte.',
    },
    'auto-crop': {
      name: 'Recorte automático de márgenes',
      tagline: 'Detecta y recorta los márgenes blancos de las páginas.',
      description: 'Renderiza cada página localmente, detecta los límites del contenido que no es blanco y ajusta cuadros de recorte por página con un margen de seguridad.',
    },
    metadata: {
      name: 'Editar metadatos',
      tagline: 'Cambia el título, el autor y más.',
      description: 'Consulta y sobrescribe los metadatos del documento (título, autor, asunto, palabras clave).',
    },
    sign: {
      name: 'Firmar',
      tagline: 'Dibuja una firma y colócala.',
      description: 'Dibuja una firma y colócala en la página. Es una marca electrónica, no una firma criptográfica.',
    },
    optimize: {
      name: 'Optimizar',
      tagline: 'Reduce el archivo sin perder calidad.',
      description: 'Reduce el tamaño de un PDF sin pérdidas eliminando objetos no utilizados y comprimiendo los flujos: el texto sigue siendo seleccionable.',
    },
    compress: {
      name: 'Comprimir',
      tagline: 'Reduce PDF con muchas imágenes mediante Ghostscript.',
      description: 'Usa Ghostscript pdfwrite localmente para una compresión de PDF con pérdidas. El texto sigue siendo seleccionable cuando Ghostscript puede reescribir el archivo; la alternativa por rasterización deja el texto no seleccionable.',
    },
    'compress-image': {
      name: 'Comprimir imagen',
      tagline: 'Reduce PNG, JPG, WebP, AVIF, JPEG XL y entradas de tipo TIFF.',
      description: 'Vuelve a codificar y, opcionalmente, redimensiona localmente imágenes del navegador junto con formatos como TIFF, PSD, BMP, GIF e ICO. Elige WebP, AVIF, JPEG XL o JPG para una salida con pérdidas más pequeña.',
    },
    'fill-form': {
      name: 'Rellenar formulario',
      tagline: 'Rellena los campos de formulario detectados.',
      description: 'Detecta los campos de AcroForm y rellénalos en tu navegador.',
    },
    flatten: {
      name: 'Aplanar',
      tagline: 'Fija los campos de formulario en la página.',
      description: 'Aplana los campos de formulario y las anotaciones para convertirlos en contenido estático de la página.',
    },
    'remove-metadata': {
      name: 'Eliminar metadatos',
      tagline: 'Elimina la información oculta antes de compartir.',
      description: 'Elimina los metadatos del documento (título, autor, productor, …) para que no se compartan por accidente.',
    },
    protect: {
      name: 'Proteger PDF',
      tagline: 'Añade una contraseña para abrir un PDF.',
      description: 'Cifra un PDF localmente con una contraseña de usuario y, opcionalmente, una contraseña de propietario y permisos.',
    },
    unlock: {
      name: 'Desbloquear PDF',
      tagline: 'Quita el cifrado por contraseña cuando conoces la contraseña.',
      description: 'Abre un PDF cifrado con su contraseña y guarda una copia desbloqueada localmente.',
    },
    sanitize: {
      name: 'Sanear PDF',
      tagline: 'Elimina los extras activos y ocultos del PDF.',
      description: 'Elimina metadatos, scripts del documento, archivos incrustados, acciones de página y, opcionalmente, anotaciones/formularios antes de compartir.',
    },
    redact: {
      name: 'Censurar PDF',
      tagline: 'Fija de forma permanente los recuadros de censura.',
      description: 'Cubre las áreas seleccionadas y reconstruye las páginas como páginas de PDF solo de imagen para que no quede texto oculto debajo.',
    },
  },
};

export default es;
