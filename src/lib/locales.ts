import type { Category, Tool } from './registry.js';

export type Locale = 'en' | 'uk' | 'it';
export type NonDefaultLocale = Exclude<Locale, 'en'>;

export interface LocaleMeta {
  code: Locale;
  htmlLang: string;
  label: string;
  pathPrefix: string;
}

export const LOCALES: Record<Locale, LocaleMeta> = {
  en: { code: 'en', htmlLang: 'en', label: 'English', pathPrefix: '' },
  uk: { code: 'uk', htmlLang: 'uk', label: 'Українська', pathPrefix: '/uk' },
  it: { code: 'it', htmlLang: 'it', label: 'Italiano', pathPrefix: '/it' },
};

export const DEFAULT_LOCALE: Locale = 'en';
export const NON_DEFAULT_LOCALES: NonDefaultLocale[] = ['uk', 'it'];
export const ALL_LOCALES: Locale[] = ['en', ...NON_DEFAULT_LOCALES];

export function localizedToolPath(locale: Locale, slug: string): string {
  return `${LOCALES[locale].pathPrefix}/tools/${slug}`;
}

export function toolAlternates(slug: string): { lang: string; path: string }[] {
  return [
    ...ALL_LOCALES.map((locale) => ({ lang: LOCALES[locale].htmlLang, path: localizedToolPath(locale, slug) })),
    { lang: 'x-default', path: localizedToolPath(DEFAULT_LOCALE, slug) },
  ];
}

interface ToolText {
  name: string;
  tagline: string;
  description: string;
}

type ToolTextMap = Record<string, ToolText>;

const UK_TOOLS: ToolTextMap = {
  merge: {
    name: "Об'єднати PDF",
    tagline: 'Зберіть кілька PDF в один документ.',
    description: 'Обʼєднайте кілька PDF-файлів в один документ у тому порядку, в якому ви їх додали.',
  },
  split: {
    name: 'Розділити PDF',
    tagline: 'Розбийте один PDF на кілька файлів.',
    description: 'Розділіть PDF на окремі файли: по одній сторінці або за власними діапазонами.',
  },
  'extract-pages': {
    name: 'Витягти сторінки',
    tagline: 'Залиште тільки потрібні сторінки.',
    description: 'Створіть новий PDF лише з вибраних сторінок.',
  },
  'remove-pages': {
    name: 'Видалити сторінки',
    tagline: 'Приберіть сторінки, які не потрібні.',
    description: 'Видаліть вибрані сторінки та збережіть решту документа.',
  },
  reorder: {
    name: 'Упорядкувати сторінки',
    tagline: 'Поставте сторінки в новому порядку.',
    description: 'Змініть порядок сторінок PDF за власною послідовністю.',
  },
  rotate: {
    name: 'Повернути PDF',
    tagline: 'Поверніть сторінки правильно.',
    description: 'Поверніть усі або вибрані сторінки на 90, 180 чи 270 градусів без втрати якості.',
  },
  'n-up': {
    name: 'Кілька сторінок на аркуші',
    tagline: 'Розмістіть кілька сторінок на одному аркуші.',
    description: 'Покладіть кілька сторінок вихідного PDF на кожен аркуш A4 для роздаткових матеріалів.',
  },
  booklet: {
    name: 'Буклет',
    tagline: 'Підготуйте сторінки для друку складеного буклета.',
    description: 'Переставте й розмістіть сторінки по дві на альбомному аркуші, щоб після складання вийшов буклет.',
  },
  'compare-pdf': {
    name: 'Порівняти PDF',
    tagline: 'Знайдіть візуальні відмінності між двома PDF.',
    description: 'Відрендерте два PDF локально й отримайте посторінковий звіт про візуальні відмінності.',
  },
  'images-to-pdf': {
    name: 'Зображення → PDF',
    tagline: 'Перетворіть JPG і PNG на PDF.',
    description: 'Створіть PDF з одного або кількох JPG/PNG-зображень, по одному зображенню на сторінку.',
  },
  'pdf-to-jpg': {
    name: 'PDF → JPG',
    tagline: 'Збережіть кожну сторінку як JPG.',
    description: 'Відрендерте кожну сторінку PDF у JPG і завантажте результат ZIP-архівом.',
  },
  'pdf-to-png': {
    name: 'PDF → PNG',
    tagline: 'Збережіть кожну сторінку як PNG.',
    description: 'Відрендерте кожну сторінку PDF у PNG без втрат і завантажте результат ZIP-архівом.',
  },
  'pdf-to-text': {
    name: 'PDF → текст',
    tagline: 'Витягніть виділюваний текст з PDF.',
    description: 'Витягніть виділюваний текст з PDF. Скановані документи без текстового шару потребують OCR.',
  },
  'pdf-to-epub': {
    name: 'PDF → EPUB',
    tagline: 'Створіть EPUB з виділюваного тексту PDF.',
    description: 'Створіть EPUB з адаптивним текстом або fixed-layout EPUB зі сторінок-зображень для сканів і складних макетів.',
  },
  'pdf-to-docx': {
    name: 'Витягти у Word',
    tagline: 'Збережіть текст PDF як DOCX.',
    description: 'Витягніть виділюваний текст у простий документ Word. Це текст, а не відтворення оригінального макета.',
  },
  'pdf-to-excel': {
    name: 'Витягти в Excel',
    tagline: 'Перетворіть текстові рядки на таблицю.',
    description: 'Найкраща можлива клієнтська спроба витягти виділювані рядки тексту в книгу Excel. Для сканів спершу потрібен OCR.',
  },
  'pdf-to-pptx': {
    name: 'Витягти в PowerPoint',
    tagline: 'Перетворіть текст PDF на прості слайди.',
    description: 'Витягніть виділюваний текст PDF у просту презентацію PowerPoint, один слайд на сторінку. Це не відтворює оригінальний макет.',
  },
  'ocr-pdf': {
    name: 'OCR для PDF',
    tagline: 'Додайте виділюваний текст до сканованих PDF.',
    description: 'Локально запустіть Tesseract OCR і додайте невидимий пошуковий текстовий шар на кожну сторінку. Англійська модель включена.',
  },
  pdfa: {
    name: 'Експорт PDF/A',
    tagline: 'Створіть архівну копію PDF/A-2b.',
    description: 'Локально відрендерте сторінки й перебудуйте їх як візуальний PDF/A-2b файл через Rust-рушій krilla. Текст стане невиділюваним.',
  },
  'image-convert': {
    name: 'Конвертувати зображення',
    tagline: 'PNG, JPG, WebP, AVIF, JPEG XL і TIFF-подібні формати.',
    description: 'Конвертуйте зображення та формати на кшталт TIFF, PSD, BMP, GIF і ICO у PNG, JPG, WebP, AVIF або JPEG XL повністю у браузері.',
  },
  'html-to-pdf': {
    name: 'HTML / Markdown → PDF',
    tagline: 'Перетворіть текстовий документ на PDF.',
    description: 'Створіть читабельний PDF з HTML, Markdown або plain-text файлу. Це текстовий конвертер, не браузерний рушій макета.',
  },
  'page-numbers': {
    name: 'Номери сторінок',
    tagline: 'Додайте номери сторінок у PDF.',
    description: 'Додайте номери сторінок з налаштовуваним форматом і позицією.',
  },
  bates: {
    name: 'Bates-нумерація',
    tagline: 'Додайте юридичні Bates-номери.',
    description: 'Поставте послідовні Bates-номери на кожній сторінці з префіксом, фіксованою кількістю цифр і позицією.',
  },
  watermark: {
    name: 'Водяний знак',
    tagline: 'Додайте текстовий водяний знак.',
    description: 'Поставте діагональний текстовий водяний знак на кожну сторінку PDF.',
  },
  crop: {
    name: 'Обрізати PDF',
    tagline: 'Приберіть зайві поля сторінок.',
    description: 'Обріжте поля на кожній сторінці, задавши crop box.',
  },
  'auto-crop': {
    name: 'Автообрізання полів',
    tagline: 'Знайдіть і приберіть білі поля сторінок.',
    description: 'Локально відрендерте кожну сторінку, знайдіть межі небілого вмісту та задайте crop box з відступом для кожної сторінки.',
  },
  metadata: {
    name: 'Редагувати метадані',
    tagline: 'Змініть назву, автора та інші дані.',
    description: 'Перегляньте й перезапишіть метадані документа: назву, автора, тему, ключові слова.',
  },
  sign: {
    name: 'Підписати',
    tagline: 'Намалюйте підпис і розмістіть його.',
    description: 'Намалюйте підпис і поставте його на сторінку. Це візуальна позначка, не криптографічний підпис.',
  },
  optimize: {
    name: 'Оптимізувати',
    tagline: 'Зменшіть файл без втрати якості.',
    description: 'Стисніть PDF без втрат: видаліть невикористані обʼєкти й стисніть потоки, зберігши виділюваний текст.',
  },
  compress: {
    name: 'Стиснути PDF',
    tagline: 'Стискайте PDF із зображеннями через Ghostscript.',
    description: 'Локально використайте Ghostscript pdfwrite для стиснення PDF. Якщо спрацює растровий запасний режим, текст стане невиділюваним.',
  },
  'compress-image': {
    name: 'Стиснути зображення',
    tagline: 'Стискайте PNG, JPG, WebP, AVIF, JPEG XL і TIFF-подібні формати.',
    description: 'Перекодуйте і за потреби зменшіть зображення та формати на кшталт TIFF, PSD, BMP, GIF і ICO локально.',
  },
  'fill-form': {
    name: 'Заповнити форму',
    tagline: 'Заповніть знайдені поля форми.',
    description: 'Знайдіть поля AcroForm і заповніть їх у браузері.',
  },
  flatten: {
    name: 'Звести форму',
    tagline: 'Зафіксуйте поля форми на сторінці.',
    description: 'Перетворіть поля форми й анотації на статичний вміст сторінки.',
  },
  'remove-metadata': {
    name: 'Видалити метадані',
    tagline: 'Приберіть приховану інформацію перед надсиланням.',
    description: 'Видаліть метадані документа: назву, автора, програму створення та інші поля.',
  },
  protect: {
    name: 'Захистити PDF',
    tagline: 'Додайте пароль для відкриття PDF.',
    description: 'Локально зашифруйте PDF паролем користувача та, за потреби, окремим паролем власника й дозволами.',
  },
  unlock: {
    name: 'Розблокувати PDF',
    tagline: 'Зніміть шифрування, якщо знаєте пароль.',
    description: 'Відкрийте зашифрований PDF з паролем і збережіть локально розблоковану копію.',
  },
  sanitize: {
    name: 'Очистити PDF',
    tagline: 'Приберіть активні й приховані PDF-додатки.',
    description: 'Видаліть метадані, скрипти документа, вкладені файли, дії сторінок і за потреби анотації та форми перед надсиланням.',
  },
  redact: {
    name: 'Заредагувати PDF',
    tagline: 'Зафіксуйте області редагування.',
    description: 'Закрийте вибрані області й перебудуйте сторінки як зображення, щоб прихований текст не лишався під ними.',
  },
};

const IT_TOOLS: ToolTextMap = {
  merge: {
    name: 'Unisci PDF',
    tagline: 'Combina più PDF in un solo documento.',
    description: "Combina più file PDF in un unico documento, nell'ordine in cui li aggiungi.",
  },
  split: {
    name: 'Dividi PDF',
    tagline: 'Dividi un PDF in più file.',
    description: 'Dividi un PDF in file separati: una pagina per file oppure intervalli personalizzati.',
  },
  'extract-pages': {
    name: 'Estrai pagine',
    tagline: 'Tieni solo le pagine che ti servono.',
    description: 'Crea un nuovo PDF contenente solo le pagine che scegli.',
  },
  'remove-pages': {
    name: 'Rimuovi pagine',
    tagline: 'Elimina le pagine che non vuoi.',
    description: 'Rimuovi le pagine selezionate e conserva il resto del documento.',
  },
  reorder: {
    name: 'Riordina pagine',
    tagline: 'Metti le pagine in un nuovo ordine.',
    description: 'Riordina le pagine di un PDF secondo una nuova sequenza.',
  },
  rotate: {
    name: 'Ruota PDF',
    tagline: 'Gira le pagine nel verso giusto.',
    description: 'Ruota tutte le pagine, o solo quelle selezionate, di 90, 180 o 270 gradi senza perdita.',
  },
  'n-up': {
    name: 'Più pagine per foglio',
    tagline: 'Stampa più pagine su ogni foglio.',
    description: 'Disponi più pagine sorgente su ogni foglio A4, utile per dispense e bozze.',
  },
  booklet: {
    name: 'Opuscolo',
    tagline: 'Prepara le pagine per la stampa piegata.',
    description: 'Riordina e dispone le pagine due per foglio orizzontale, così si piegano in un opuscolo.',
  },
  'compare-pdf': {
    name: 'Confronta PDF',
    tagline: 'Trova differenze visive tra due PDF.',
    description: 'Renderizza due PDF localmente e genera un report pagina per pagina sulle differenze visive.',
  },
  'images-to-pdf': {
    name: 'Immagini → PDF',
    tagline: 'Trasforma JPG e PNG in PDF.',
    description: 'Crea un PDF da una o più immagini JPG/PNG, una immagine per pagina.',
  },
  'pdf-to-jpg': {
    name: 'PDF → JPG',
    tagline: 'Salva ogni pagina come JPG.',
    description: 'Renderizza ogni pagina PDF in JPG e scarica le immagini in un archivio ZIP.',
  },
  'pdf-to-png': {
    name: 'PDF → PNG',
    tagline: 'Salva ogni pagina come PNG.',
    description: 'Renderizza ogni pagina PDF in PNG senza perdita e scarica le immagini in un archivio ZIP.',
  },
  'pdf-to-text': {
    name: 'PDF → testo',
    tagline: 'Estrai il testo selezionabile da un PDF.',
    description: 'Estrai il testo selezionabile da un PDF. I PDF scansionati non hanno testo da estrarre senza OCR.',
  },
  'pdf-to-epub': {
    name: 'PDF → EPUB',
    tagline: 'Crea un EPUB dal testo selezionabile del PDF.',
    description: 'Crea un EPUB con testo reflowable oppure un EPUB a layout fisso da pagine renderizzate per scansioni e layout complessi.',
  },
  'pdf-to-docx': {
    name: 'Estrai in Word',
    tagline: 'Salva il testo del PDF come DOCX.',
    description: 'Estrai il testo selezionabile in un semplice documento Word. Preserva il testo, non il layout originale.',
  },
  'pdf-to-excel': {
    name: 'Estrai in Excel',
    tagline: 'Trasforma righe di testo in un foglio di calcolo.',
    description: 'Estrazione best-effort delle righe di testo selezionabile in una cartella Excel. Le scansioni richiedono OCR.',
  },
  'pdf-to-pptx': {
    name: 'Estrai in PowerPoint',
    tagline: 'Trasforma il testo del PDF in slide semplici.',
    description: 'Estrai il testo selezionabile del PDF in una presentazione PowerPoint semplice, una slide per pagina. Non ricrea il layout originale.',
  },
  'ocr-pdf': {
    name: 'OCR PDF ricercabile',
    tagline: 'Aggiungi testo selezionabile ai PDF scansionati.',
    description: 'Esegue Tesseract OCR localmente e aggiunge un livello di testo invisibile e ricercabile su ogni pagina. Modello inglese incluso.',
  },
  pdfa: {
    name: 'Esporta PDF/A',
    tagline: 'Crea una copia archivistica PDF/A-2b.',
    description: 'Renderizza le pagine localmente e le ricostruisce come PDF/A-2b visivo con il motore Rust krilla. Il testo diventa non selezionabile.',
  },
  'image-convert': {
    name: 'Converti immagine',
    tagline: 'PNG, JPG, WebP, AVIF, JPEG XL e formati tipo TIFF.',
    description: 'Converti immagini e formati come TIFF, PSD, BMP, GIF e ICO in PNG, JPG, WebP, AVIF o JPEG XL interamente nel browser.',
  },
  'html-to-pdf': {
    name: 'HTML / Markdown → PDF',
    tagline: 'Trasforma un documento testuale in PDF.',
    description: 'Converti un file HTML, Markdown o testo semplice in un PDF leggibile. È un convertitore testuale, non un motore di layout browser.',
  },
  'page-numbers': {
    name: 'Numeri di pagina',
    tagline: 'Aggiungi numeri di pagina al PDF.',
    description: 'Aggiungi numeri di pagina con formato e posizione personalizzabili.',
  },
  bates: {
    name: 'Numerazione Bates',
    tagline: 'Aggiungi numeri Bates in stile legale.',
    description: 'Applica numeri Bates sequenziali a ogni pagina con prefisso, cifre fisse e posizione.',
  },
  watermark: {
    name: 'Filigrana',
    tagline: 'Aggiungi una filigrana testuale.',
    description: 'Applica una filigrana testuale diagonale a ogni pagina del PDF.',
  },
  crop: {
    name: 'Ritaglia PDF',
    tagline: 'Taglia i margini delle pagine.',
    description: 'Ritaglia i margini di ogni pagina impostando il crop box.',
  },
  'auto-crop': {
    name: 'Ritaglio automatico',
    tagline: 'Rileva e taglia i margini bianchi.',
    description: 'Renderizza ogni pagina localmente, rileva i bordi del contenuto non bianco e imposta crop box per pagina con margine.',
  },
  metadata: {
    name: 'Modifica metadati',
    tagline: 'Cambia titolo, autore e altro.',
    description: 'Visualizza e sovrascrivi i metadati del documento: titolo, autore, oggetto e parole chiave.',
  },
  sign: {
    name: 'Firma',
    tagline: 'Disegna una firma e posizionala.',
    description: 'Disegna una firma e inseriscila sulla pagina. È una firma visiva, non una firma crittografica.',
  },
  optimize: {
    name: 'Ottimizza',
    tagline: 'Riduci il file senza perdita di qualità.',
    description: 'Riduci un PDF senza perdita rimuovendo oggetti inutilizzati e comprimendo i flussi; il testo resta selezionabile.',
  },
  compress: {
    name: 'Comprimi PDF',
    tagline: 'Riduci PDF ricchi di immagini con Ghostscript.',
    description: 'Usa Ghostscript pdfwrite localmente per comprimere il PDF. Se parte il fallback raster, il testo diventa non selezionabile.',
  },
  'compress-image': {
    name: 'Comprimi immagine',
    tagline: 'Riduci PNG, JPG, WebP, AVIF, JPEG XL e formati tipo TIFF.',
    description: 'Ricodifica e, se vuoi, ridimensiona immagini e formati come TIFF, PSD, BMP, GIF e ICO localmente.',
  },
  'fill-form': {
    name: 'Compila modulo',
    tagline: 'Compila i campi modulo rilevati.',
    description: 'Rileva i campi AcroForm e compilali nel browser.',
  },
  flatten: {
    name: 'Appiattisci',
    tagline: 'Incorpora i campi modulo nella pagina.',
    description: 'Trasforma campi modulo e annotazioni in contenuto statico della pagina.',
  },
  'remove-metadata': {
    name: 'Rimuovi metadati',
    tagline: 'Elimina informazioni nascoste prima di condividere.',
    description: 'Rimuovi metadati del documento come titolo, autore, programma di creazione e altri campi.',
  },
  protect: {
    name: 'Proteggi PDF',
    tagline: 'Aggiungi una password di apertura al PDF.',
    description: 'Cripta un PDF localmente con password utente e, se vuoi, password proprietario e permessi separati.',
  },
  unlock: {
    name: 'Sblocca PDF',
    tagline: 'Rimuovi la crittografia se conosci la password.',
    description: 'Apri un PDF criptato con la sua password e salva localmente una copia sbloccata.',
  },
  sanitize: {
    name: 'Sanifica PDF',
    tagline: 'Rimuovi elementi attivi e nascosti dal PDF.',
    description: 'Elimina metadati, script del documento, file incorporati, azioni di pagina e, opzionalmente, annotazioni e moduli prima della condivisione.',
  },
  redact: {
    name: 'Oscura PDF',
    tagline: 'Applica riquadri di oscuramento permanenti.',
    description: 'Copri le aree selezionate e ricostruisci le pagine come immagini, così il testo nascosto non resta sotto.',
  },
};

const TOOL_TEXT: Record<NonDefaultLocale, ToolTextMap> = {
  uk: UK_TOOLS,
  it: IT_TOOLS,
};

export function localizeTool(tool: Tool, locale: Locale): Tool {
  if (locale === 'en') return tool;
  const text = TOOL_TEXT[locale][tool.id];
  return text ? { ...tool, ...text } : tool;
}

export const CATEGORY_TEXT: Record<Locale, Record<Category, { label: string; descriptor: string }>> = {
  en: {
    organize: { label: 'Organize', descriptor: 'Reorder, merge, split, rotate' },
    convert: { label: 'Convert', descriptor: 'Move between PDF, image and text' },
    edit: { label: 'Edit', descriptor: 'Stamp, mark up and sign pages' },
    optimize: { label: 'Optimize', descriptor: 'Make files smaller' },
    forms: { label: 'Forms', descriptor: 'Detect and fill form fields' },
    security: { label: 'Security & Privacy', descriptor: 'Strip what you do not want to share' },
  },
  uk: {
    organize: { label: 'Упорядкування', descriptor: 'Порядок, обʼєднання, поділ, поворот' },
    convert: { label: 'Конвертація', descriptor: 'PDF, зображення й текст' },
    edit: { label: 'Редагування', descriptor: 'Позначки, водяні знаки й підпис' },
    optimize: { label: 'Оптимізація', descriptor: 'Зменшення файлів' },
    forms: { label: 'Форми', descriptor: 'Поля форм і заповнення' },
    security: { label: 'Безпека й приватність', descriptor: 'Прибрати зайве перед надсиланням' },
  },
  it: {
    organize: { label: 'Organizza', descriptor: 'Ordina, unisci, dividi, ruota' },
    convert: { label: 'Converti', descriptor: 'Tra PDF, immagini e testo' },
    edit: { label: 'Modifica', descriptor: 'Timbri, filigrane e firme' },
    optimize: { label: 'Ottimizza', descriptor: 'Riduci le dimensioni' },
    forms: { label: 'Moduli', descriptor: 'Rileva e compila campi' },
    security: { label: 'Sicurezza e privacy', descriptor: 'Rimuovi ciò che non vuoi condividere' },
  },
};

export interface LayoutText {
  skip: string;
  homeAria: string;
  navAria: string;
  allTools: string;
  howItWorks: string;
  privacy: string;
  footerBlurb: string;
  tools: string;
  project: string;
  trust: string;
  github: string;
  license: string;
  reportIssue: string;
  noUpload: string;
  noAccount: string;
  noTracking: string;
}

export const LAYOUT_TEXT: Record<Locale, LayoutText> = {
  en: {
    skip: 'Skip to content',
    homeAria: 'Unfleece home',
    navAria: 'Primary',
    allTools: 'All tools',
    howItWorks: 'How it works',
    privacy: 'Privacy',
    footerBlurb: 'Free, private PDF tools. Your files never leave your browser.',
    tools: 'Tools',
    project: 'Project',
    trust: 'Trust',
    github: 'GitHub ↗',
    license: 'AGPL-3.0',
    reportIssue: 'Report an issue ↗',
    noUpload: 'No upload',
    noAccount: 'No account',
    noTracking: 'No tracking',
  },
  uk: {
    skip: 'Перейти до вмісту',
    homeAria: 'Домівка Unfleece',
    navAria: 'Основна навігація',
    allTools: 'Усі інструменти',
    howItWorks: 'Як це працює',
    privacy: 'Приватність',
    footerBlurb: 'Безкоштовні приватні PDF-інструменти. Ваші файли не залишають браузер.',
    tools: 'Інструменти',
    project: 'Проєкт',
    trust: 'Довіра',
    github: 'GitHub ↗',
    license: 'AGPL-3.0',
    reportIssue: 'Повідомити про проблему ↗',
    noUpload: 'Без завантаження',
    noAccount: 'Без акаунта',
    noTracking: 'Без трекінгу',
  },
  it: {
    skip: 'Vai al contenuto',
    homeAria: 'Home di Unfleece',
    navAria: 'Navigazione principale',
    allTools: 'Tutti gli strumenti',
    howItWorks: 'Come funziona',
    privacy: 'Privacy',
    footerBlurb: 'Strumenti PDF gratuiti e privati. I tuoi file restano nel browser.',
    tools: 'Strumenti',
    project: 'Progetto',
    trust: 'Fiducia',
    github: 'GitHub ↗',
    license: 'AGPL-3.0',
    reportIssue: 'Segnala un problema ↗',
    noUpload: 'Nessun upload',
    noAccount: 'Nessun account',
    noTracking: 'Nessun tracciamento',
  },
};

export interface ToolPageText {
  breadcrumbHome: string;
  breadcrumbTools: string;
  privateWorkspace: string;
  nothingUploaded: string;
  howEyebrow: string;
  howHeading: (toolName: string) => string;
  steps: { title: string; body: string }[];
  differenceEyebrow: string;
  differenceHeading: string;
  usBullets: string[];
  themHeading: string;
  themBullets: string[];
  questionsEyebrow: string;
  questionsHeading: string;
  relatedEyebrow: string;
  relatedHeading: string;
  faqs: (toolName: string) => { q: string; a: string }[];
}

export const TOOL_PAGE_TEXT: Record<Locale, ToolPageText> = {
  en: {
    breadcrumbHome: 'Home',
    breadcrumbTools: 'Tools',
    privateWorkspace: 'Private workspace',
    nothingUploaded: 'Nothing uploaded',
    howEyebrow: 'How it works',
    howHeading: (toolName) => `How to ${toolName.toLowerCase()}`,
    steps: [
      { title: 'Add your file', body: 'Drop it in or click to choose. It stays on your device.' },
      { title: 'Set options', body: 'Tweak the settings, or just use the sensible defaults.' },
      { title: 'Download the result', body: 'One click. It was built right here, in your browser.' },
    ],
    differenceEyebrow: 'The honest difference',
    differenceHeading: 'Why Unfleece vs fleeceware',
    usBullets: ['Runs in your browser — no upload', 'Free, no account, no card', 'Open source (AGPL-3.0)', 'No file-size paywall'],
    themHeading: 'Typical fleeceware site',
    themBullets: ['Uploads your file to a server', '$1 trial → ~$50 / week', 'Closed and opaque', 'Locks results behind pay'],
    questionsEyebrow: 'Questions',
    questionsHeading: 'Frequently asked',
    relatedEyebrow: 'Keep going',
    relatedHeading: 'Related tools',
    faqs: (toolName) => [
      {
        q: `Is ${toolName} really free?`,
        a: 'Yes — free, forever, no account and no trial that bites. There is no server bill to recover because nothing is uploaded.',
      },
      {
        q: 'Are my files uploaded anywhere?',
        a: `No. Your browser reads the file into memory and runs ${toolName} on your device. Open the Network tab and watch — you will see zero upload requests.`,
      },
      {
        q: 'Is there a file-size or page limit?',
        a: 'No hard limit. The only ceiling is your browser memory, so very large files may be slow. We warn you before that becomes a problem.',
      },
      {
        q: 'Does it work offline?',
        a: 'Once the page has loaded, yes. You can turn off Wi-Fi and it still works — the strongest proof that nothing leaves your device.',
      },
    ],
  },
  uk: {
    breadcrumbHome: 'Головна',
    breadcrumbTools: 'Інструменти',
    privateWorkspace: 'Приватна робоча зона',
    nothingUploaded: 'Нічого не завантажується',
    howEyebrow: 'Як це працює',
    howHeading: (toolName) => `Як використати ${toolName}`,
    steps: [
      { title: 'Додайте файл', body: 'Перетягніть файл або виберіть його вручну. Він лишається на вашому пристрої.' },
      { title: 'Налаштуйте параметри', body: 'Змініть опції або залиште безпечні значення за замовчуванням.' },
      { title: 'Завантажте результат', body: 'Один клік. Результат створено прямо у вашому браузері.' },
    ],
    differenceEyebrow: 'Чесна різниця',
    differenceHeading: 'Чому Unfleece замість fleeceware',
    usBullets: ['Працює у браузері — без upload', 'Безкоштовно, без акаунта й картки', 'Відкритий код (AGPL-3.0)', 'Без платного ліміту розміру файлу'],
    themHeading: 'Типовий fleeceware-сайт',
    themBullets: ['Завантажує файл на сервер', 'Пробний $1 → приблизно $50 на тиждень', 'Закритий і непрозорий', 'Блокує результат оплатою'],
    questionsEyebrow: 'Питання',
    questionsHeading: 'Часті запитання',
    relatedEyebrow: 'Продовжити',
    relatedHeading: 'Схожі інструменти',
    faqs: (toolName) => [
      {
        q: `Чи ${toolName} справді безкоштовний?`,
        a: 'Так — безкоштовний назавжди, без акаунта і без пробного періоду з прихованою оплатою. Нам не потрібно покривати серверні витрати, бо файли не завантажуються.',
      },
      {
        q: 'Чи мої файли кудись завантажуються?',
        a: `Ні. Браузер читає файл у памʼять і виконує ${toolName} на вашому пристрої. Відкрийте вкладку Network — upload-запитів не буде.`,
      },
      {
        q: 'Чи є ліміт розміру або кількості сторінок?',
        a: 'Жорсткого ліміту немає. Межа — памʼять вашого браузера, тому дуже великі файли можуть працювати повільніше.',
      },
      {
        q: 'Чи працює без інтернету?',
        a: 'Так, після завантаження сторінки. Можна вимкнути Wi‑Fi — інструмент усе одно працюватиме на пристрої.',
      },
    ],
  },
  it: {
    breadcrumbHome: 'Home',
    breadcrumbTools: 'Strumenti',
    privateWorkspace: 'Area privata',
    nothingUploaded: 'Nessun upload',
    howEyebrow: 'Come funziona',
    howHeading: (toolName) => `Come usare ${toolName}`,
    steps: [
      { title: 'Aggiungi il file', body: 'Trascinalo qui o scegli un file. Resta sul tuo dispositivo.' },
      { title: 'Imposta le opzioni', body: 'Modifica le impostazioni oppure usa i valori predefiniti.' },
      { title: 'Scarica il risultato', body: 'Un clic. Il file viene creato qui, nel tuo browser.' },
    ],
    differenceEyebrow: 'La differenza onesta',
    differenceHeading: 'Perché Unfleece invece dei siti fleeceware',
    usBullets: ['Funziona nel browser — nessun upload', 'Gratis, senza account, senza carta', 'Open source (AGPL-3.0)', 'Nessun paywall sulla dimensione dei file'],
    themHeading: 'Tipico sito fleeceware',
    themBullets: ['Carica il tuo file su un server', 'Prova da $1 → circa $50 a settimana', 'Chiuso e opaco', 'Blocca il risultato dietro pagamento'],
    questionsEyebrow: 'Domande',
    questionsHeading: 'Domande frequenti',
    relatedEyebrow: 'Continua',
    relatedHeading: 'Strumenti correlati',
    faqs: (toolName) => [
      {
        q: `${toolName} e davvero gratis?`,
        a: "Sì — gratis per sempre, senza account e senza prove che diventano addebiti. Non c'è un server da pagare perché nulla viene caricato.",
      },
      {
        q: 'I miei file vengono caricati da qualche parte?',
        a: `No. Il browser legge il file in memoria ed esegue ${toolName} sul tuo dispositivo. Apri la scheda Network: non vedrai richieste di upload.`,
      },
      {
        q: "C'e un limite di dimensione o pagine?",
        a: "Non c'è un limite rigido. Il limite reale è la memoria del browser, quindi file molto grandi possono essere più lenti.",
      },
      {
        q: 'Funziona offline?',
        a: 'Dopo il caricamento della pagina, si. Puoi disattivare il Wi‑Fi e lo strumento continua a funzionare sul dispositivo.',
      },
    ],
  },
};
