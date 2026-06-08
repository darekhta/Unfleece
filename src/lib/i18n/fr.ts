// French — full translation bundle. Mirrors the structure of en.ts exactly and
// falls back to English for anything missing.
import type { LocaleBundle } from './types.js';

export const fr: LocaleBundle = {
  meta: { code: 'fr', htmlLang: 'fr', autonym: 'Français', englishName: 'French', dir: 'ltr' },
  categories: {
    organize: { label: 'Organiser', descriptor: 'Réorganiser, fusionner, diviser, pivoter' },
    convert: { label: 'Convertir', descriptor: 'Passer du PDF à l’image et au texte' },
    edit: { label: 'Modifier', descriptor: 'Tamponner, annoter et signer les pages' },
    optimize: { label: 'Optimiser', descriptor: 'Réduire la taille des fichiers' },
    forms: { label: 'Formulaires', descriptor: 'Détecter et remplir les champs de formulaire' },
    security: { label: 'Sécurité et confidentialité', descriptor: 'Supprimez ce que vous ne voulez pas partager' },
  },
  layout: {
    skip: 'Aller au contenu',
    homeAria: 'Accueil Unfleece',
    navAria: 'Principal',
    allTools: 'Tous les outils',
    howItWorks: 'Comment ça marche',
    privacy: 'Confidentialité',
    footerBlurb: 'Des outils PDF gratuits et privés. Vos fichiers ne quittent jamais votre navigateur.',
    tools: 'Outils',
    project: 'Projet',
    trust: 'Confiance',
    github: 'GitHub ↗',
    license: 'AGPL-3.0',
    reportIssue: 'Signaler un problème ↗',
    noUpload: 'Aucun envoi de fichier',
    noAccount: 'Aucun compte',
    noTracking: 'Aucun suivi',
    language: 'Langue',
  },
  toolPage: {
    breadcrumbHome: 'Accueil',
    breadcrumbTools: 'Outils',
    privateWorkspace: 'Espace de travail privé',
    nothingUploaded: 'Aucun fichier envoyé',
    howEyebrow: 'Comment ça marche',
    howHeading: (toolName) => `${toolName} : mode d’emploi`,
    steps: [
      { title: 'Ajoutez votre fichier', body: 'Glissez-le ici ou cliquez pour le choisir. Il reste sur votre appareil.' },
      { title: 'Réglez les options', body: 'Ajustez les paramètres, ou gardez simplement les valeurs par défaut bien pensées.' },
      { title: 'Téléchargez le résultat', body: 'Un seul clic. Tout a été créé ici même, dans votre navigateur.' },
    ],
    differenceEyebrow: 'La différence, en toute honnêteté',
    differenceHeading: 'Unfleece face aux arnaques fleeceware',
    usBullets: ['Fonctionne dans votre navigateur — aucun envoi de fichier', 'Gratuit, sans compte, sans carte bancaire', 'Open source (AGPL-3.0)', 'Aucune limite de taille payante'],
    themHeading: 'Site fleeceware typique',
    themBullets: ['Envoie votre fichier sur un serveur', 'Essai à 1 $ → ~50 $ / semaine', 'Fermé et opaque', 'Verrouille les résultats derrière un paiement'],
    questionsEyebrow: 'Questions',
    questionsHeading: 'Questions fréquentes',
    relatedEyebrow: 'Continuez',
    relatedHeading: 'Outils similaires',
    faqs: (toolName) => [
      {
        q: `${toolName}, c’est vraiment gratuit ?`,
        a: 'Oui — gratuit, pour toujours, sans compte et sans essai qui se transforme en piège. Il n’y a aucune facture de serveur à rentabiliser, puisque rien n’est envoyé.',
      },
      {
        q: 'Mes fichiers sont-ils envoyés quelque part ?',
        a: `Non. Votre navigateur charge le fichier en mémoire et exécute ${toolName} sur votre appareil. Ouvrez l’onglet Réseau et observez — vous ne verrez aucune requête d’envoi.`,
      },
      {
        q: 'Y a-t-il une limite de taille ou de nombre de pages ?',
        a: 'Aucune limite stricte. Le seul plafond est la mémoire de votre navigateur ; les fichiers très volumineux peuvent donc être lents. Nous vous prévenons avant que cela ne pose problème.',
      },
      {
        q: 'Est-ce que ça fonctionne hors ligne ?',
        a: 'Une fois la page chargée, oui. Vous pouvez couper le Wi-Fi et tout continue de fonctionner — la meilleure preuve que rien ne quitte votre appareil.',
      },
    ],
  },
  home: {
    heroLine1: 'Des outils PDF gratuits et privés.',
    heroLine2: 'Vos fichiers ne quittent jamais votre navigateur.',
    subhead:
      'Fusionnez, divisez, convertissez, signez et compressez — chaque outil s’exécute entièrement sur votre appareil. Gratuit pour toujours, sans compte, sans envoi. Open source, et honnête à ce sujet.',
    browseCta: 'Parcourir les outils',
    howCta: 'Comment ça marche',
    proof: [
      { title: 'Privé par conception', body: 'Les fichiers sont traités localement et ne sont jamais envoyés.', linkLabel: 'Vérifiez dans l’onglet Réseau.' },
      { title: 'Gratuit pour toujours', body: 'Aucun essai, aucun abonnement, aucune limite de taille payante. Il n’y a rien à vous vendre en plus.' },
      { title: 'Rapide et ouvert', body: 'Fonctionne localement via WebAssembly. Entièrement open source (AGPL-3.0).' },
    ],
    toolsEyebrow: 'Tous les outils',
    pickHeading: 'Choisissez un outil — il s’ouvre, vous l’utilisez, vous téléchargez.',
    pickBody: 'Aucune inscription. Aucune file d’attente. Tout se passe dans cet onglet.',
    proveEyebrow: 'Ne nous croyez pas sur parole — vérifiez',
    proveHeading: 'Ça marche en 10 secondes, et vous pouvez prouver que c’est privé.',
    proveBody:
      'Aucun fichier ne touche jamais un serveur : il n’y a donc rien à vous facturer et rien à fuiter. La gratuité, c’est simplement la valeur par défaut honnête.',
    proveSteps: [
      { strong: 'Ouvrez un outil', rest: ' et appuyez sur F12 → onglet Réseau.' },
      { strong: 'Lancez-le sur un fichier.', rest: ' Observez la liste des requêtes.' },
      { strong: 'Constatez : aucun envoi.', rest: ' Coupez le Wi-Fi — ça marche toujours.' },
    ],
  },
  tools: {
    merge: {
      name: 'Fusionner PDF',
      tagline: 'Combinez des PDF en un seul document.',
      description: 'Combinez plusieurs fichiers PDF en un seul document, dans l’ordre où vous les ajoutez.',
    },
    split: {
      name: 'Diviser PDF',
      tagline: 'Divisez un PDF en plusieurs fichiers.',
      description: 'Divisez un PDF en fichiers séparés — une page par fichier, ou par plages de pages personnalisées.',
    },
    'extract-pages': {
      name: 'Extraire des pages',
      tagline: 'Extrayez uniquement les pages dont vous avez besoin.',
      description: 'Créez un nouveau PDF contenant uniquement les pages que vous choisissez.',
    },
    'remove-pages': {
      name: 'Supprimer des pages',
      tagline: 'Supprimez les pages dont vous ne voulez pas.',
      description: 'Supprimez les pages que vous sélectionnez et conservez le reste.',
    },
    reorder: {
      name: 'Réorganiser les pages',
      tagline: 'Remettez les pages dans un nouvel ordre.',
      description: 'Réorganisez les pages d’un PDF selon un nouvel ordre.',
    },
    rotate: {
      name: 'Pivoter le PDF',
      tagline: 'Remettez les pages dans le bon sens.',
      description: 'Faites pivoter toutes les pages ou une sélection de 90°, 180° ou 270°. Sans perte.',
    },
    'n-up': {
      name: 'N pages par feuille',
      tagline: 'Imprimez plusieurs pages par feuille.',
      description: 'Placez plusieurs pages source sur chaque feuille A4 — idéal pour les supports à distribuer.',
    },
    booklet: {
      name: 'Livret',
      tagline: 'Imposez les pages pour imprimer un livret plié.',
      description: 'Réorganisez et placez les pages deux par deux sur des feuilles en paysage pour les plier en livret.',
    },
    'compare-pdf': {
      name: 'Comparer des PDF',
      tagline: 'Repérez les différences visuelles entre deux PDF.',
      description: 'Affichez deux PDF localement et générez un rapport des différences visuelles page par page.',
    },
    'images-to-pdf': {
      name: 'Images → PDF',
      tagline: 'Transformez des JPG et des PNG en PDF.',
      description: 'Transformez une ou plusieurs images JPG/PNG en PDF, une image par page.',
    },
    'pdf-to-jpg': {
      name: 'PDF → JPG',
      tagline: 'Enregistrez chaque page en JPG.',
      description: 'Convertissez chaque page du PDF en image JPG et téléchargez-les dans un ZIP.',
    },
    'pdf-to-png': {
      name: 'PDF → PNG',
      tagline: 'Enregistrez chaque page en PNG.',
      description: 'Convertissez chaque page du PDF en image PNG sans perte et téléchargez-les dans un ZIP.',
    },
    'pdf-to-text': {
      name: 'PDF → Texte',
      tagline: 'Extrayez le texte sélectionnable d’un PDF.',
      description: 'Extrayez le texte sélectionnable d’un PDF. (Les PDF scannés ou composés d’images n’ont aucun texte à extraire.)',
    },
    'pdf-to-epub': {
      name: 'PDF → EPUB',
      tagline: 'Créez un EPUB à partir du texte sélectionnable d’un PDF.',
      description: 'Créez un EPUB redimensionnable à partir du texte sélectionnable d’un PDF, ou un EPUB à mise en page fixe à partir des pages rendues pour les scans et les mises en page complexes.',
    },
    'pdf-to-docx': {
      name: 'Extraire vers Word',
      tagline: 'Enregistrez le texte sélectionnable d’un PDF en DOCX.',
      description: 'Extrayez le texte sélectionnable dans un document Word simple. Le texte est conservé, mais pas la mise en page d’origine.',
    },
    'pdf-to-excel': {
      name: 'Extraire vers Excel',
      tagline: 'Transformez les lignes sélectionnables en feuille de calcul.',
      description: 'Extraction au mieux des lignes de texte sélectionnable vers un classeur Excel. Les scans nécessitent d’abord une OCR.',
    },
    'pdf-to-pptx': {
      name: 'Extraire vers PowerPoint',
      tagline: 'Transformez le texte d’un PDF en diapositives simples.',
      description: 'Extrayez le texte sélectionnable d’un PDF dans une présentation PowerPoint simple, une diapositive par page. La mise en page d’origine n’est pas reproduite.',
    },
    'ocr-pdf': {
      name: 'PDF interrogeable (OCR)',
      tagline: 'Ajoutez du texte sélectionnable aux PDF scannés.',
      description: 'Exécutez l’OCR Tesseract localement et ajoutez une couche de texte invisible et interrogeable sur chaque page. Modèle anglais inclus.',
    },
    pdfa: {
      name: 'Export PDF/A',
      tagline: 'Créez une copie d’archivage au format PDF/A-2b.',
      description: 'Rendez les pages localement et reconstruisez-les en un fichier PDF/A-2b visuel grâce au moteur Rust krilla. Le texte devient non sélectionnable.',
    },
    'image-convert': {
      name: 'Convertir une image',
      tagline: 'Passez d’un format à l’autre : PNG, JPG, WebP, AVIF, JPEG XL et formats de type TIFF.',
      description: 'Convertissez les images de navigateur ainsi que des formats comme TIFF, PSD, BMP, GIF et ICO en PNG, JPG, WebP, AVIF ou JPEG XL, entièrement dans votre navigateur.',
    },
    'html-to-pdf': {
      name: 'HTML / Markdown → PDF',
      tagline: 'Transformez un document texte en PDF soigné.',
      description: 'Convertissez un fichier HTML, Markdown ou texte brut en un PDF lisible. Cet outil se concentre sur le texte, ce n’est pas un moteur de mise en page de navigateur.',
    },
    'page-numbers': {
      name: 'Numéros de page',
      tagline: 'Apposez des numéros de page sur un PDF.',
      description: 'Ajoutez des numéros de page avec un format et une position personnalisables.',
    },
    bates: {
      name: 'Numérotation Bates',
      tagline: 'Ajoutez des numéros Bates de style juridique.',
      description: 'Apposez des numéros Bates séquentiels sur chaque page, avec préfixe, numéro complété par des zéros et position.',
    },
    watermark: {
      name: 'Filigrane',
      tagline: 'Ajoutez un filigrane texte sur chaque page.',
      description: 'Apposez un filigrane texte en diagonale sur chaque page.',
    },
    crop: {
      name: 'Recadrer',
      tagline: 'Rognez les marges de vos pages.',
      description: 'Rognez les marges de chaque page en définissant la zone de recadrage.',
    },
    'auto-crop': {
      name: 'Recadrage automatique des marges',
      tagline: 'Détectez et rognez les marges blanches des pages.',
      description: 'Rendez chaque page localement, détectez les limites du contenu non blanc et définissez une zone de recadrage par page avec une marge de sécurité.',
    },
    metadata: {
      name: 'Modifier les métadonnées',
      tagline: 'Modifiez le titre, l’auteur et plus encore.',
      description: 'Consultez et remplacez les métadonnées du document (titre, auteur, sujet, mots-clés).',
    },
    sign: {
      name: 'Signer',
      tagline: 'Dessinez une signature et placez-la.',
      description: 'Dessinez une signature et placez-la sur la page. Marque électronique, et non une signature cryptographique.',
    },
    optimize: {
      name: 'Optimiser',
      tagline: 'Réduisez la taille du fichier sans perte de qualité.',
      description: 'Réduisez la taille d’un PDF sans perte en supprimant les objets inutilisés et en compressant les flux — le texte reste sélectionnable.',
    },
    compress: {
      name: 'Compresser',
      tagline: 'Réduisez les PDF riches en images avec Ghostscript.',
      description: 'Utilisez Ghostscript pdfwrite localement pour une compression PDF avec perte. Le texte reste sélectionnable lorsque Ghostscript peut réécrire le fichier ; la solution de repli matricielle rend le texte non sélectionnable.',
    },
    'compress-image': {
      name: 'Compresser une image',
      tagline: 'Réduisez les fichiers PNG, JPG, WebP, AVIF, JPEG XL et de type TIFF.',
      description: 'Réencodez et, si vous le souhaitez, redimensionnez localement les images de navigateur ainsi que les TIFF, PSD, BMP, GIF et ICO. Choisissez WebP, AVIF, JPEG XL ou JPG pour un résultat avec perte plus léger.',
    },
    'fill-form': {
      name: 'Remplir un formulaire',
      tagline: 'Remplissez les champs de formulaire détectés.',
      description: 'Détectez les champs AcroForm et remplissez-les dans votre navigateur.',
    },
    flatten: {
      name: 'Aplatir',
      tagline: 'Intégrez les champs de formulaire dans la page.',
      description: 'Aplatissez les champs de formulaire et les annotations en contenu de page statique.',
    },
    'remove-metadata': {
      name: 'Supprimer les métadonnées',
      tagline: 'Effacez les infos cachées avant de partager.',
      description: 'Supprimez les métadonnées du document (titre, auteur, producteur, …) pour ne pas les partager par accident.',
    },
    protect: {
      name: 'Protéger le PDF',
      tagline: 'Ajoutez un mot de passe pour ouvrir un PDF.',
      description: 'Chiffrez un PDF localement avec un mot de passe utilisateur et, en option, un mot de passe propriétaire et des autorisations.',
    },
    unlock: {
      name: 'Déverrouiller le PDF',
      tagline: 'Retirez le chiffrement par mot de passe quand vous le connaissez.',
      description: 'Ouvrez un PDF chiffré avec son mot de passe et enregistrez localement une copie déverrouillée.',
    },
    sanitize: {
      name: 'Assainir le PDF',
      tagline: 'Supprimez les éléments actifs et cachés d’un PDF.',
      description: 'Supprimez les métadonnées, les scripts du document, les fichiers intégrés, les actions de page et, en option, les annotations et formulaires avant de partager.',
    },
    redact: {
      name: 'Caviarder le PDF',
      tagline: 'Intégrez définitivement des zones de caviardage.',
      description: 'Masquez les zones sélectionnées et reconstruisez les pages en PDF composé uniquement d’images, afin qu’aucun texte caché ne subsiste en dessous.',
    },
  },
};

export default fr;
