// Italian — full translation bundle. Reuses the previously vetted Italian copy
// for categories, layout, tool pages and tool text; the home namespace and the
// language-switcher label are translated here.
import type { LocaleBundle } from './types.js';

export const it: LocaleBundle = {
  meta: { code: 'it', htmlLang: 'it', autonym: 'Italiano', englishName: 'Italian', dir: 'ltr' },
  categories: {
    organize: { label: 'Organizza', descriptor: 'Ordina, unisci, dividi, ruota' },
    convert: { label: 'Converti', descriptor: 'Tra PDF, immagini e testo' },
    edit: { label: 'Modifica', descriptor: 'Timbri, filigrane e firme' },
    optimize: { label: 'Ottimizza', descriptor: 'Riduci le dimensioni' },
    forms: { label: 'Moduli', descriptor: 'Rileva e compila campi' },
    security: { label: 'Sicurezza e privacy', descriptor: 'Rimuovi ciò che non vuoi condividere' },
  },
  layout: {
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
    language: 'Lingua',
  },
  toolPage: {
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
  home: {
    heroLine1: 'Strumenti PDF gratuiti e privati.',
    heroLine2: 'I tuoi file non lasciano mai il browser.',
    subhead:
      'Unisci, dividi, converti, firma e comprimi: ogni strumento funziona interamente sul tuo dispositivo. Gratis per sempre, senza account e senza upload. Open source, e onesti al riguardo.',
    browseCta: 'Esplora gli strumenti',
    howCta: 'Come funziona',
    proof: [
      { title: 'Privato per principio', body: 'I file vengono elaborati localmente e non vengono mai caricati.', linkLabel: 'Verifica nella scheda Network.' },
      { title: 'Gratis per sempre', body: "Niente prove, niente abbonamenti, niente paywall sulla dimensione dei file. Non c'è nulla da venderti in più." },
      { title: 'Veloce e aperto', body: 'Funziona localmente tramite WebAssembly. Completamente open source (AGPL-3.0).' },
    ],
    toolsEyebrow: 'Tutti gli strumenti',
    pickHeading: 'Scegli uno strumento: si apre, lo usi, scarichi il risultato.',
    pickBody: 'Nessuna registrazione. Nessuna coda. Tutto accade in questa scheda.',
    proveEyebrow: 'Non fidarti di noi: verifica',
    proveHeading: 'Funziona in 10 secondi, e puoi dimostrare che è privato.',
    proveBody:
      "Nessun file tocca mai un server, quindi non c'è nulla da fatturarti e nulla che possa trapelare. Gratis è semplicemente l'impostazione onesta.",
    proveSteps: [
      { strong: 'Apri uno strumento qualsiasi', rest: ' e premi F12 → scheda Network.' },
      { strong: 'Usalo su un file.', rest: " Guarda l'elenco delle richieste." },
      { strong: 'Vedrai che non si carica nulla.', rest: ' Spegni il Wi‑Fi: funziona lo stesso.' },
    ],
  },
  tools: {
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
  },
};

export default it;
