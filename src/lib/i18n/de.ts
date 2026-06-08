// German — full translation bundle. Mirrors the structure of en.ts exactly and
// falls back to English only for anything genuinely missing.
import type { LocaleBundle } from './types.js';

export const de: LocaleBundle = {
  meta: { code: 'de', htmlLang: 'de', autonym: 'Deutsch', englishName: 'German', dir: 'ltr' },
  categories: {
    organize: { label: 'Organisieren', descriptor: 'Neu anordnen, zusammenführen, teilen, drehen' },
    convert: { label: 'Konvertieren', descriptor: 'Zwischen PDF, Bild und Text wechseln' },
    edit: { label: 'Bearbeiten', descriptor: 'Seiten stempeln, markieren und unterschreiben' },
    optimize: { label: 'Optimieren', descriptor: 'Dateien kleiner machen' },
    forms: { label: 'Formulare', descriptor: 'Formularfelder erkennen und ausfüllen' },
    security: { label: 'Sicherheit & Datenschutz', descriptor: 'Entferne, was du nicht teilen willst' },
  },
  layout: {
    skip: 'Zum Inhalt springen',
    homeAria: 'Unfleece Startseite',
    navAria: 'Hauptnavigation',
    allTools: 'Alle Tools',
    howItWorks: 'So funktioniert’s',
    privacy: 'Datenschutz',
    footerBlurb: 'Kostenlose, private PDF-Tools. Deine Dateien verlassen niemals deinen Browser.',
    tools: 'Tools',
    project: 'Projekt',
    trust: 'Vertrauen',
    github: 'GitHub ↗',
    license: 'AGPL-3.0',
    reportIssue: 'Problem melden ↗',
    noUpload: 'Kein Datei-Upload',
    noAccount: 'Kein Konto',
    noTracking: 'Kein Tracking',
    language: 'Sprache',
  },
  toolPage: {
    breadcrumbHome: 'Start',
    breadcrumbTools: 'Tools',
    privateWorkspace: 'Privater Arbeitsbereich',
    nothingUploaded: 'Keine Datei hochgeladen',
    howEyebrow: 'So funktioniert’s',
    howHeading: (toolName) => `So nutzt du ${toolName}`,
    steps: [
      { title: 'Datei hinzufügen', body: 'Zieh sie hinein oder klick zum Auswählen. Sie bleibt auf deinem Gerät.' },
      { title: 'Optionen festlegen', body: 'Pass die Einstellungen an – oder nimm einfach die sinnvollen Voreinstellungen.' },
      { title: 'Ergebnis herunterladen', body: 'Ein Klick. Erstellt genau hier, in deinem Browser.' },
    ],
    differenceEyebrow: 'Der ehrliche Unterschied',
    differenceHeading: 'Warum Unfleece statt Fleeceware',
    usBullets: ['Läuft in deinem Browser – kein Datei-Upload', 'Kostenlos, kein Konto, keine Kreditkarte', 'Open Source (AGPL-3.0)', 'Keine Bezahlschranke je nach Dateigröße'],
    themHeading: 'Typische Fleeceware-Seite',
    themBullets: ['Lädt deine Datei auf einen Server hoch', '$1 Testphase → ~$50 / Woche', 'Geschlossen und undurchsichtig', 'Sperrt Ergebnisse hinter einer Bezahlschranke'],
    questionsEyebrow: 'Fragen',
    questionsHeading: 'Häufig gestellt',
    relatedEyebrow: 'Weitermachen',
    relatedHeading: 'Verwandte Tools',
    faqs: (toolName) => [
      {
        q: `Ist ${toolName} wirklich kostenlos?`,
        a: 'Ja – kostenlos, für immer, ohne Konto und ohne fiese Testphase. Es gibt keine Serverrechnung zu decken, weil nichts hochgeladen wird.',
      },
      {
        q: 'Werden meine Dateien irgendwohin hochgeladen?',
        a: `Nein. Dein Browser liest die Datei in den Arbeitsspeicher und führt ${toolName} auf deinem Gerät aus. Öffne den Netzwerk-Tab und schau zu – du wirst null Upload-Anfragen sehen.`,
      },
      {
        q: 'Gibt es ein Limit für Dateigröße oder Seitenzahl?',
        a: 'Kein hartes Limit. Die einzige Grenze ist der Arbeitsspeicher deines Browsers, daher können sehr große Dateien langsam sein. Wir warnen dich, bevor das zum Problem wird.',
      },
      {
        q: 'Funktioniert es offline?',
        a: 'Sobald die Seite geladen ist, ja. Du kannst das Wi-Fi ausschalten und es funktioniert trotzdem – der beste Beweis, dass nichts dein Gerät verlässt.',
      },
    ],
  },
  home: {
    heroLine1: 'Kostenlose, private PDF-Tools.',
    heroLine2: 'Deine Dateien verlassen niemals deinen Browser.',
    subhead:
      'Zusammenführen, teilen, konvertieren, unterschreiben und komprimieren – jedes Tool läuft komplett auf deinem Gerät. Für immer kostenlos, kein Konto, kein Upload. Open Source und ehrlich dabei.',
    browseCta: 'Tools durchstöbern',
    howCta: 'So funktioniert’s',
    proof: [
      { title: 'Privat by Design', body: 'Dateien werden lokal verarbeitet und niemals hochgeladen.', linkLabel: 'Überprüf es im Netzwerk-Tab.' },
      { title: 'Für immer kostenlos', body: 'Keine Testphasen, keine Abos, keine Bezahlschranken nach Dateigröße. Es gibt nichts hochzuverkaufen.' },
      { title: 'Schnell & offen', body: 'Läuft lokal über WebAssembly. Vollständig Open Source (AGPL-3.0).' },
    ],
    toolsEyebrow: 'Alle Tools',
    pickHeading: 'Wähl ein Tool – es öffnet sich, du nutzt es, du lädst herunter.',
    pickBody: 'Keine Anmeldung. Keine Warteschlange. Alles passiert in diesem Tab.',
    proveEyebrow: 'Vertrau uns nicht – überprüf es',
    proveHeading: 'Es läuft in 10 Sekunden, und du kannst beweisen, dass es privat ist.',
    proveBody:
      'Keine Datei berührt jemals einen Server – also gibt es nichts, wofür wir dir Geld abnehmen könnten, und nichts, das durchsickern kann. Kostenlos ist einfach die ehrliche Voreinstellung.',
    proveSteps: [
      { strong: 'Öffne ein beliebiges Tool', rest: ' und drück F12 → Netzwerk-Tab.' },
      { strong: 'Nutz es mit einer Datei.', rest: ' Beobachte die Anfrageliste.' },
      { strong: 'Sieh, dass nichts hochgeladen wird.', rest: ' Schalt das Wi-Fi aus – es funktioniert trotzdem.' },
    ],
  },
  tools: {
    merge: {
      name: 'PDF zusammenführen',
      tagline: 'PDFs zu einem Dokument kombinieren.',
      description: 'Mehrere PDF-Dateien zu einem einzigen Dokument kombinieren – in der Reihenfolge, in der du sie hinzufügst.',
    },
    split: {
      name: 'PDF teilen',
      tagline: 'Ein PDF in mehrere Dateien aufteilen.',
      description: 'Ein PDF in separate Dateien aufteilen – eine pro Seite oder nach eigenen Seitenbereichen.',
    },
    'extract-pages': {
      name: 'Seiten extrahieren',
      tagline: 'Nur die Seiten herausholen, die du brauchst.',
      description: 'Ein neues PDF erstellen, das nur die von dir gewählten Seiten enthält.',
    },
    'remove-pages': {
      name: 'Seiten entfernen',
      tagline: 'Lösch die Seiten, die du nicht willst.',
      description: 'Entferne die ausgewählten Seiten und behalte den Rest.',
    },
    reorder: {
      name: 'Seiten neu anordnen',
      tagline: 'Bring die Seiten in eine neue Reihenfolge.',
      description: 'Ordne die Seiten eines PDFs in einer neuen Reihenfolge an.',
    },
    rotate: {
      name: 'PDF drehen',
      tagline: 'Dreh die Seiten richtig herum.',
      description: 'Drehe alle oder ausgewählte Seiten um 90°, 180° oder 270°. Verlustfrei.',
    },
    'n-up': {
      name: 'N-up pro Blatt',
      tagline: 'Mehrere Seiten pro Blatt drucken.',
      description: 'Platziere mehrere Quellseiten auf jedem A4-Blatt – ideal für Handouts.',
    },
    booklet: {
      name: 'Broschüre',
      tagline: 'Seiten für den Druck als gefaltete Broschüre ausschießen.',
      description: 'Ordne Seiten neu an und platziere sie zweifach auf Querformat-Blättern, sodass sie sich zu einer Broschüre falten lassen.',
    },
    'compare-pdf': {
      name: 'PDFs vergleichen',
      tagline: 'Finde visuelle Unterschiede zwischen zwei PDFs.',
      description: 'Rendere zwei PDFs lokal und erstelle einen visuellen Unterschiedsbericht Seite für Seite.',
    },
    'images-to-pdf': {
      name: 'Bilder → PDF',
      tagline: 'Mach aus JPGs und PNGs ein PDF.',
      description: 'Verwandle ein oder mehrere JPG-/PNG-Bilder in ein PDF, ein Bild pro Seite.',
    },
    'pdf-to-jpg': {
      name: 'PDF → JPG',
      tagline: 'Speichere jede Seite als JPG.',
      description: 'Rendere jede PDF-Seite als JPG-Bild und lade sie als ZIP herunter.',
    },
    'pdf-to-png': {
      name: 'PDF → PNG',
      tagline: 'Speichere jede Seite als PNG.',
      description: 'Rendere jede PDF-Seite als verlustfreies PNG-Bild und lade sie als ZIP herunter.',
    },
    'pdf-to-text': {
      name: 'PDF → Text',
      tagline: 'Hol markierbaren Text aus einem PDF.',
      description: 'Extrahiere den markierbaren Text aus einem PDF. (Gescannte PDFs bzw. Bild-PDFs enthalten keinen extrahierbaren Text.)',
    },
    'pdf-to-epub': {
      name: 'PDF → EPUB',
      tagline: 'Erstelle ein EPUB aus markierbarem PDF-Text.',
      description: 'Baue ein umfließbares EPUB aus markierbarem PDF-Text – oder ein EPUB mit festem Layout aus gerenderten Seiten für Scans und komplexe Layouts.',
    },
    'pdf-to-docx': {
      name: 'Nach Word extrahieren',
      tagline: 'Speichere markierbaren PDF-Text als DOCX.',
      description: 'Extrahiere markierbaren Text in ein einfaches Word-Dokument. Es bewahrt den Text, nicht das ursprüngliche Layout.',
    },
    'pdf-to-excel': {
      name: 'Nach Excel extrahieren',
      tagline: 'Verwandle markierbare Zeilen in eine Tabelle.',
      description: 'Bestmögliche Extraktion markierbarer Textzeilen in eine Excel-Arbeitsmappe. Scans brauchen erst OCR.',
    },
    'pdf-to-pptx': {
      name: 'Nach PowerPoint extrahieren',
      tagline: 'Verwandle PDF-Text in einfache Folien.',
      description: 'Extrahiere markierbaren PDF-Text in eine einfache PowerPoint-Präsentation, eine Folie pro Seite. Das ursprüngliche Layout wird nicht nachgebildet.',
    },
    'ocr-pdf': {
      name: 'Durchsuchbares PDF per OCR',
      tagline: 'Füge gescannten PDFs markierbaren Text hinzu.',
      description: 'Führe Tesseract-OCR lokal aus und leg eine unsichtbare, durchsuchbare Textebene über jede Seite. Englisches Modell inklusive.',
    },
    pdfa: {
      name: 'PDF/A-Export',
      tagline: 'Erstelle eine archivtaugliche PDF/A-2b-Kopie.',
      description: 'Rendere Seiten lokal und baue sie mit der Rust-Engine krilla als visuelle PDF/A-2b-Datei neu auf. Text wird dabei nicht mehr markierbar.',
    },
    'image-convert': {
      name: 'Bild konvertieren',
      tagline: 'Wechsle zwischen PNG, JPG, WebP, AVIF, JPEG XL und TIFF-artigen Eingaben.',
      description: 'Konvertiere Browser-Bilder und Formate wie TIFF, PSD, BMP, GIF und ICO in PNG, JPG, WebP, AVIF oder JPEG XL – komplett in deinem Browser.',
    },
    'html-to-pdf': {
      name: 'HTML / Markdown → PDF',
      tagline: 'Mach aus einem Textdokument ein sauberes PDF.',
      description: 'Konvertiere eine HTML-, Markdown- oder Klartext-Datei in ein gut lesbares PDF. Das ist textorientiert, keine Browser-Layout-Engine.',
    },
    'page-numbers': {
      name: 'Seitenzahlen',
      tagline: 'Stemple Seitenzahlen auf ein PDF.',
      description: 'Füge Seitenzahlen mit anpassbarem Format und anpassbarer Position hinzu.',
    },
    bates: {
      name: 'Bates-Nummerierung',
      tagline: 'Füge Bates-Nummern im juristischen Stil hinzu.',
      description: 'Stemple fortlaufende Bates-Nummern auf jede Seite – mit Präfix, aufgefüllter Nummer und Position.',
    },
    watermark: {
      name: 'Wasserzeichen',
      tagline: 'Füge jeder Seite ein Text-Wasserzeichen hinzu.',
      description: 'Stemple ein diagonales Text-Wasserzeichen über jede Seite.',
    },
    crop: {
      name: 'Zuschneiden',
      tagline: 'Schneide die Ränder deiner Seiten weg.',
      description: 'Schneide Ränder von jeder Seite weg, indem du die Beschnittbox festlegst.',
    },
    'auto-crop': {
      name: 'Ränder automatisch zuschneiden',
      tagline: 'Erkenne und entferne weiße Seitenränder.',
      description: 'Rendere jede Seite lokal, erkenne die nicht-weißen Inhaltsgrenzen und setze pro Seite Beschnittboxen mit Abstand.',
    },
    metadata: {
      name: 'Metadaten bearbeiten',
      tagline: 'Ändere Titel, Autor und mehr.',
      description: 'Sieh dir die Dokument-Metadaten an und überschreibe sie (Titel, Autor, Betreff, Schlagwörter).',
    },
    sign: {
      name: 'Unterschreiben',
      tagline: 'Zeichne eine Unterschrift und platziere sie.',
      description: 'Zeichne eine Unterschrift und platziere sie auf der Seite. Elektronisches Zeichen, keine kryptografische Signatur.',
    },
    optimize: {
      name: 'Optimieren',
      tagline: 'Verkleinere die Datei ohne Qualitätsverlust.',
      description: 'Verkleinere ein PDF verlustfrei, indem ungenutzte Objekte entfernt und Streams komprimiert werden – Text bleibt markierbar.',
    },
    compress: {
      name: 'Komprimieren',
      tagline: 'Verkleinere bildlastige PDFs mit Ghostscript.',
      description: 'Nutze Ghostscript pdfwrite lokal für verlustbehaftete PDF-Komprimierung. Text bleibt markierbar, wenn Ghostscript die Datei neu schreiben kann; der Raster-Fallback macht Text nicht mehr markierbar.',
    },
    'compress-image': {
      name: 'Bild komprimieren',
      tagline: 'Verkleinere PNG, JPG, WebP, AVIF, JPEG XL und TIFF-artige Eingaben.',
      description: 'Kodiere Browser-Bilder sowie TIFF, PSD, BMP, GIF und ICO lokal neu und ändere optional ihre Größe. Wähle WebP, AVIF, JPEG XL oder JPG für eine kleinere, verlustbehaftete Ausgabe.',
    },
    'fill-form': {
      name: 'Formular ausfüllen',
      tagline: 'Fülle erkannte Formularfelder aus.',
      description: 'Erkenne AcroForm-Felder und fülle sie in deinem Browser aus.',
    },
    flatten: {
      name: 'Reduzieren',
      tagline: 'Backe Formularfelder fest in die Seite ein.',
      description: 'Reduziere Formularfelder und Anmerkungen zu statischem Seiteninhalt.',
    },
    'remove-metadata': {
      name: 'Metadaten entfernen',
      tagline: 'Entferne versteckte Infos vor dem Teilen.',
      description: 'Entferne Dokument-Metadaten (Titel, Autor, Erzeuger …), damit sie nicht versehentlich geteilt werden.',
    },
    protect: {
      name: 'PDF schützen',
      tagline: 'Füge ein Passwort zum Öffnen eines PDFs hinzu.',
      description: 'Verschlüssele ein PDF lokal mit einem Benutzerpasswort sowie optionalem Besitzerpasswort und Berechtigungen.',
    },
    unlock: {
      name: 'PDF entsperren',
      tagline: 'Entferne die Passwortverschlüsselung, wenn du das Passwort kennst.',
      description: 'Öffne ein verschlüsseltes PDF mit seinem Passwort und speichere lokal eine entsperrte Kopie.',
    },
    sanitize: {
      name: 'PDF bereinigen',
      tagline: 'Entferne aktive und versteckte PDF-Extras.',
      description: 'Entferne Metadaten, Dokumentskripte, eingebettete Dateien, Seitenaktionen und optional Anmerkungen/Formulare vor dem Teilen.',
    },
    redact: {
      name: 'PDF schwärzen',
      tagline: 'Brenne Schwärzungsboxen fest ein.',
      description: 'Decke ausgewählte Bereiche ab und baue die Seiten als reine Bild-PDF-Seiten neu auf, sodass kein versteckter Text darunter zurückbleibt.',
    },
  },
};

export default de;
