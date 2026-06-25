// Per-tool English on-page copy (unique intro + tool-specific FAQ) used to give
// each tool page genuinely distinct, helpful content rather than a shared template.
// English is the source of truth here; locales may override via their i18n bundle
// (ToolText.intro / ToolText.faqs). Drafted from each tool's real behavior and
// verified against the implementation. Keep claims honest and accurate.

export interface ToolContent {
  /** Unique 2-4 sentence overview, distinct from the one-line description. */
  intro: string;
  /** Tool-specific FAQ; the last item is the shared privacy question. */
  faqs: { q: string; a: string }[];
}

export const TOOL_CONTENT: Record<string, ToolContent> = {
  "merge": {
    intro: "Merge PDF stitches several separate PDFs into one file, so a contract, its appendices, and a signed cover page can travel as a single document. You decide the sequence: add your files, arrange them in the order you want, and the pages are joined in exactly that order. The whole thing runs in a Web Worker right inside this tab, so your documents are read into memory and combined on your own machine rather than sent off to a server. Reach for it when you have a handful of PDFs that belong together.",
    faqs: [
      { q: "Can I control the order the PDFs are combined in?", a: "Yes. The files are merged in the exact order you add and arrange them, so put them in sequence before you run it and the pages follow that order from first file to last." },
      { q: "Can I merge more than two PDFs at once?", a: "Yes. You can add several PDF files in one go, and they are all combined into a single output document in the order you set." },
      { q: "Does it run in the background or freeze the page?", a: "It runs in a Web Worker, a separate background thread in your browser, so the merge happens off the main page. That keeps the tab responsive while your files are being joined." },
      { q: "Are my files uploaded anywhere?", a: "No. Your browser reads the PDFs into memory and merges them on your device in a Web Worker, so nothing is sent to a server. Open the Network tab while it runs and you will see no file-upload request." },
    ],
  },
  "split": {
    intro: "Reach for this when one PDF needs to become several — say a scanned packet that should be one file per page, or a long report you want broken into chapters. Pick \"every page into its own file\" to burst the whole document apart, or \"by page ranges\" and type something like 1-3, 4-6, 7 to carve out exactly the sections you want, one file per range. Because it produces more than one PDF, the results come back together as a single ZIP. The whole split happens in your browser, so the original never goes anywhere.",
    faqs: [
      { q: "What's the difference between the two split modes?", a: "Every page mode turns each page into its own single-page PDF, while ranges mode lets you group pages into a custom file per range. Use ranges and type something like 1-3, 4-6, 7 to get one PDF for each group." },
      { q: "Why is the result a ZIP file?", a: "Splitting produces more than one PDF, so the tool bundles them into a single ZIP for one tidy download. Unzip it to get your separate PDFs." },
      { q: "How do I write the page ranges?", a: "List each range as a group of pages, separated by commas, such as 1-3, 4-6, 7. Each range becomes its own PDF, and pages are numbered from 1." },
      { q: "Are my files uploaded anywhere?", a: "No. Your browser reads the PDF into memory and splits it on your device, then hands back the ZIP. Open the Network tab and watch while it runs — you will see no file-upload request." },
    ],
  },
  "extract-pages": {
    intro: "Reach for this when you only need a few pages out of a longer PDF, like pulling a single signed form, one chapter, or a handful of exhibits out of a big report. You type the pages and ranges you want using 1-based numbers, separated by commas, for example 1-3, 5, and you get back a new PDF holding only those pages in that order. The original file is untouched. Everything happens in your browser through a worker, so the PDF you drop in never leaves your device and nothing is uploaded.",
    faqs: [
      { q: "How do I type the pages I want?", a: "Enter the pages and ranges using 1-based numbers separated by commas, like 1-3, 5. A range such as 1-3 means pages 1 through 3, and a single number like 5 picks just that one page." },
      { q: "Does it change my original file?", a: "No, your original PDF is left alone. The tool builds a new PDF containing only the pages you chose and hands that back to you, so you keep the source file exactly as it was." },
      { q: "What order will the extracted pages come out in?", a: "The pages come out in the order you list them. If you enter 5, 1-3, the result starts with page 5 followed by pages 1 through 3, so you can rearrange while you extract." },
      { q: "Are my files uploaded anywhere?", a: "No. The extraction runs entirely in your browser, so the PDF you select never leaves your device. You can confirm this by watching the Network tab while you use the tool." },
    ],
  },
  "remove-pages": {
    intro: "Reach for this when a PDF has a few pages you want gone — a cover sheet, a blank scan, an internal note before you share the file. You type the pages and ranges to delete, like 2, 5-7, and everything else stays exactly as it was, in the same order. The result is a single PDF without the pages you named. It all happens in your browser, so the document you are trimming never leaves your device. If you would rather pick the pages to keep instead of the ones to drop, the Extract pages tool does the inverse.",
    faqs: [
      { q: "How do I tell it which pages to remove?", a: "Type the page numbers and ranges you want deleted, separated by commas, for example 2, 5-7. Page numbers are 1-based, so the first page is 1. Everything you do not list is kept in its original order." },
      { q: "What does the output look like?", a: "The output is a single PDF containing every page except the ones you removed. The remaining pages keep their original order and content; nothing is re-rendered or re-compressed." },
      { q: "Can I delete every page, or do I have to keep some?", a: "You need to keep at least one page, since a PDF cannot be empty. List the pages you want gone and leave the rest unnamed to keep them." },
      { q: "Are my files uploaded anywhere?", a: "No. The whole thing runs in your browser, so your PDF never leaves your device and nothing is sent to a server. You can confirm it yourself by opening your browser's Network tab and watching for no upload." },
    ],
  },
  "reorder": {
    intro: "Reach for this when a PDF's pages are in the wrong sequence and you want them rearranged without cutting anything out. You type the new order as a list of page numbers, like 3,1,2, and the tool rebuilds the document in that exact sequence. Every page must appear once and only once, so this is a pure rearrangement, not a way to drop or duplicate pages. The work happens right here in your browser, so your file is never uploaded.",
    faqs: [
      { q: "How do I tell it the new order?", a: "You type the page numbers in the sequence you want, separated by commas, such as 3,1,2 to move page 3 to the front. The tool reads that list and rebuilds the PDF in that order." },
      { q: "Can I drop or duplicate pages here?", a: "No. Each page number has to appear exactly once, so this tool only rearranges the pages you already have. Use the delete-pages or extract-pages tool if you want to remove or pull out specific pages instead." },
      { q: "Does reordering change the page content or quality?", a: "No. Reordering only changes the sequence of the pages; the content of each page is moved as-is, with nothing re-rendered or recompressed." },
      { q: "Are my files uploaded anywhere?", a: "No. The reordering runs entirely in your browser, and your file never leaves your device. You can confirm this by opening your browser's Network tab and watching that nothing is sent while the tool works." },
    ],
  },
  "rotate": {
    intro: "Reach for this when a scan came in sideways or a few pages were photographed upside down. You can turn every page at once, or list just the ones that need fixing, like \"1, 3-4\", and spin them 90, 180 or 270 degrees. It works by setting each page's rotation flag rather than redrawing anything, so the change is lossless: text stays selectable, images keep their original quality, and nothing is re-rendered. The whole thing runs in your browser, so your PDF never has to leave your device to get straightened out.",
    faqs: [
      { q: "Does rotating lower the quality of my PDF?", a: "No. Rotation is lossless because it sets each page's rotation flag instead of re-rendering the content, so text stays selectable and images keep their original quality." },
      { q: "Can I rotate only certain pages instead of the whole document?", a: "Yes. Leave the pages field blank to turn every page, or list specific pages like \"1, 3-4\" to rotate only those; the rest are left exactly as they are." },
      { q: "Is the rotation absolute or relative to the page's current orientation?", a: "The rotation is relative to each page's current orientation. The angle you pick is added to whatever rotation a page already has, so a page that's already sideways turns further from where it sits now." },
      { q: "Are my files uploaded anywhere?", a: "No. The rotation happens entirely in your browser and your file never leaves your device. You can confirm it yourself by watching the Network tab while you run the tool." },
    ],
  },
  "n-up": {
    intro: "N-up per sheet shrinks several source pages down and arranges them onto a single A4 sheet, so a long document prints on far fewer pieces of paper. Reach for it when you want handouts, a quick reference copy, or just to save paper and toner on a draft. You choose 2, 4, 6, or 9 pages per sheet, and the tool lays your existing pages out in a grid and gives you back a new PDF. The smaller you pack the pages, the smaller the text gets, so pick a count that stays readable for what you're printing.",
    faqs: [
      { q: "How many pages can I fit on one sheet?", a: "You can place 2, 4, 6, or 9 source pages onto each A4 sheet. Fewer pages per sheet keeps the text larger and more readable; more pages per sheet saves more paper but shrinks everything down." },
      { q: "What size are the output sheets?", a: "Every output sheet is A4, with your source pages arranged in a grid on it. The result is a new PDF you can print or save." },
      { q: "Does the text stay readable when pages are combined?", a: "It depends on how many pages you pack per sheet, since each one is scaled down to fit. Two per sheet stays close to full size, while nine per sheet makes the text quite small, so choose a count that suits your document." },
      { q: "Are my files uploaded anywhere?", a: "No. The layout happens right here in your browser, and your file never leaves your device. You can confirm it yourself by opening your browser's Network tab and watching for zero uploads while it runs." },
    ],
  },
  "booklet": {
    intro: "Reach for this when you want to print a multi-page PDF as a saddle-stitched booklet: a stack of sheets you fold down the middle and staple. It works out the imposition for you, placing two pages side by side on each landscape sheet in the shuffled order that puts everything in sequence once folded. Pick A4 or Letter, set the binding edge to left or right, and choose a margin. Because a booklet folds in groups of four, the page count is padded up to a multiple of four with blank slots so the fold lands cleanly.",
    faqs: [
      { q: "Why does my booklet have extra blank pages?", a: "Blank pages are added because a folded booklet works in groups of four sheets. The page count is padded up to the next multiple of four with blank slots so the spreads stay in order and the fold lands cleanly; with 6 source pages, for example, you get 8 placed pages." },
      { q: "Do I print this double-sided, and how do I fold it?", a: "Print the result double-sided (flip on the short edge), then stack the sheets, fold the whole stack in half, and staple along the fold. The pages are already imposed two-up in booklet order, so they read in sequence once folded." },
      { q: "What do the binding edge and margin options do?", a: "Binding edge sets whether the booklet opens on the left or the right, which matters for right-to-left layouts. Margin adds inner spacing in points around each placed page, defaulting to 18pt, so content does not run into the fold or sheet edges." },
      { q: "Are my files uploaded anywhere?", a: "No. The imposition runs entirely in your browser, so your PDF never leaves your device. You can confirm this by opening your browser's Network tab and watching that no file is sent while it works." },
    ],
  },
  "compare-pdf": {
    intro: "Reach for this when you have two versions of a document and want to know which pages actually changed. It opens both PDFs with pdf.js, renders each page to an image in your browser, and walks them pixel by pixel, then writes a plain-text report listing every page and how many pixels differ. This catches visual changes a text search would miss, like a moved logo, a tweaked figure, or a reflowed paragraph. It compares appearance, not the underlying bytes, so it is not a cryptographic file hash. Pages that differ in size are reported as fully changed.",
    faqs: [
      { q: "What does the report actually tell me?", a: "It gives you a page-by-page text summary listing, for each shared page, whether there were no pixel differences or how many sampled pixels differ and what percentage that is. It also notes any page-count mismatch and ends with a one-line summary of how many pages changed. The output is text, not an annotated or highlighted PDF." },
      { q: "What do the render scale and pixel threshold options do?", a: "Render scale sets how large each page is drawn before comparison, so a higher scale catches finer differences but uses more memory and time. Pixel threshold sets how much a pixel's combined color and transparency must change before it counts as different, letting you ignore tiny rendering noise by raising it." },
      { q: "What happens if the two PDFs have a different number of pages?", a: "It compares only the pages the two files share and reports the page-count difference separately at the top. Pages that exist in just one file are not pixel-compared, and any shared page whose rendered dimensions differ is reported as fully changed." },
      { q: "Are my files uploaded anywhere?", a: "No. Both PDFs are read into memory and rendered and compared entirely in your browser on your device. Open the Network tab and run it — you will see no file-upload request." },
    ],
  },
  "images-to-pdf": {
    intro: "Reach for this when you have a stack of photos or scans and need them in a single PDF to email, print, or file. Add your images in order and each one becomes its own page. JPGs and PNGs go straight in; other formats like WebP, HEIC, TIFF, or BMP are converted to PNG on your device first, so they fit too. Choose \"Fit page to image\" to match each page to its picture, or pick A4 or Letter with a margin for a uniform document. The pages are images, so the result has no selectable or searchable text.",
    faqs: [
      { q: "Can I control the page size and order?", a: "Yes. Pages follow the order you add the images, and you can pick \"Fit page to image\" to size each page to its picture, or choose A4 or Letter with a margin in points for the fixed sizes." },
      { q: "Will the text in my images be selectable in the PDF?", a: "No. Each page is the image itself, so any text in a photo or scan stays part of the picture and cannot be selected, copied, or searched. If you need selectable text, run the result through an OCR tool afterward." },
      { q: "What image formats can I use besides JPG and PNG?", a: "You can also add WebP, AVIF, TIFF, BMP, GIF, HEIC, PSD, and several other formats. JPG and PNG are embedded directly, while the others are converted to PNG in your browser before going into the PDF." },
      { q: "Are my files uploaded anywhere?", a: "No. The whole conversion runs in your browser, so your images never leave your device. You can confirm it yourself by watching the Network tab while the PDF is built." },
    ],
  },
  "pdf-to-jpg": {
    intro: "Reach for this when you need each PDF page as a flat image you can drop into a slide, a chat, or a web page without a PDF reader. It draws every page with pdf.js, the same renderer your browser uses to display PDFs, then encodes each one as a JPG. You set the resolution scale, so you can trade file size for sharpness, and the JPG quality, which controls compression. Since JPG is a pixel format, any text in the result is part of the image and is no longer selectable or searchable. Everything renders right here, so the PDF never leaves your device.",
    faqs: [
      { q: "Why do I get a ZIP instead of separate images?", a: "A multi-page PDF becomes one JPG per page, so the pages are bundled into a single ZIP to keep the download tidy. Unzip it and you'll find the images in page order. A one-page PDF still comes back this way." },
      { q: "What do the resolution scale and JPG quality options do?", a: "Resolution scale sets how large each page is rendered before it becomes a JPG, so a higher scale gives a sharper, bigger image. JPG quality controls compression: higher quality means a clearer image and a larger file. Pick a low scale and lower quality for small previews, or raise both for print-grade pages." },
      { q: "Will the text in the JPGs still be selectable?", a: "No. JPG is a pixel image, so any text on the page is baked into the picture and can't be selected, copied, or searched. If you need selectable text, keep the original PDF or use a PDF-to-text tool instead." },
      { q: "Are my files uploaded anywhere?", a: "No. The conversion runs entirely in your browser, and your PDF never leaves your device. Open the Network tab before you start and you'll see no file-upload request." },
    ],
  },
  "pdf-to-png": {
    intro: "Reach for this when you need each PDF page as a crisp, lossless image: a figure to drop into a slide, a page to post somewhere that won't take a PDF, or a screenshot-quality reference. Every page is rendered with pdf.js and saved as a PNG, then bundled into a single ZIP you download. The resolution scale controls how sharp the output is, so raise it for print or detail work and lower it to keep file sizes down. PNG keeps every pixel exactly, so there's no JPG-style compression blur. The whole render happens in your browser, on your machine.",
    faqs: [
      { q: "Why do I get a ZIP instead of one image?", a: "Because a PDF has many pages and each one becomes its own PNG, the tool packs them all into a single ZIP so you download once instead of clicking through every page. Unzip it and you'll have one numbered PNG per page." },
      { q: "What does the resolution scale do?", a: "The resolution scale sets how many pixels each page is rendered at, so a higher scale gives sharper, larger images and a lower scale gives smaller files. Turn it up for print or fine detail and down when you just need a quick preview." },
      { q: "Why is PNG better here than JPG?", a: "PNG is lossless, so each pixel is stored exactly with no compression blur around text or sharp edges. That makes it the better pick for screenshots, diagrams, and line art, though the files are larger than the equivalent JPG." },
      { q: "Are my files uploaded anywhere?", a: "No. The PDF is rendered to PNG entirely in your browser, on your own device, and nothing is sent to a server. You can confirm it yourself by opening your browser's Network tab and watching that no upload happens." },
    ],
  },
  "pdf-to-text": {
    intro: "Reach for this when you need the raw words out of a PDF and nothing else. It reads the text layer that is already embedded in the file and hands you back plain text you can paste into a note, an editor, or a search box. Because it only pulls what is genuinely there, a scanned or image-only PDF gives you nothing back, since those pages are pictures with no text underneath. If that is your situation, run OCR on the file first to build a text layer, then come back here. The whole thing happens in your browser, so the document stays on your machine.",
    faqs: [
      { q: "What does the output look like?", a: "The output is plain text, not a formatted document. It pulls the words from the PDF's text layer, so you get the readable content without the original page layout, fonts, or styling." },
      { q: "Why did I get nothing back from my scanned PDF?", a: "A scanned or image-only PDF has no text layer to extract, so there is nothing for this tool to pull out. Those pages are images of text, not actual text. Run an OCR tool on the file first to add a text layer, then extract from that." },
      { q: "Does it keep formatting, tables, or columns?", a: "No, this tool returns plain text only and does not preserve formatting, tables, or column structure. The text comes out in the order the PDF stores it, which may not always match the visual reading order." },
      { q: "Are my files uploaded anywhere?", a: "No. The extraction runs entirely in your browser and your PDF never leaves your device. You can confirm this by opening your browser's Network tab and watching that no upload request is made." },
    ],
  },
  "pdf-to-epub": {
    intro: "Reach for this when you want to read a PDF on an e-reader or phone, where reflowing text to fit the screen beats pinching and panning a fixed page. Reflowable mode pulls the selectable text out of the PDF and rebuilds it into a chapter that flows, and it can drop repeated headers and footers, join hard line breaks back into paragraphs, and stitch words split by line-end hyphens. If the PDF is a scan or has a layout too intricate to reconstruct, switch to fixed-layout mode, which renders each page as an image instead. You set the title, author, and language, and the file is converted in your browser.",
    faqs: [
      { q: "What's the difference between reflowable and fixed-layout mode?", a: "Reflowable mode rebuilds the PDF's selectable text into chapters that reflow to fit any screen, with optional cleanup of repeated headers and footers, wrapped lines, and line-end hyphenation. Fixed-layout mode renders each page as an image and keeps the original look, which suits scans and complex layouts but produces no selectable text." },
      { q: "Will the text be selectable and searchable in the EPUB?", a: "In reflowable mode, yes, because it converts the actual text from the PDF. In fixed-layout mode the pages are images, so the text is not selectable or searchable. Note that reflowable mode needs a PDF that already has selectable text; a scanned PDF has none, so use fixed-layout mode for scans." },
      { q: "Can I set the book's title, author, and language?", a: "Yes, you can set the title, author, and language before converting. The title defaults to the file name if you leave it blank, the author is optional, and the language sets the EPUB's metadata so readers handle it correctly." },
      { q: "Are my files uploaded anywhere?", a: "No. The conversion runs entirely in your browser using pdf.js to read the PDF and a local engine to build the EPUB, so your file never leaves your device. You can confirm this by opening your browser's Network tab and watching that no upload occurs." },
    ],
  },
  "pdf-to-docx": {
    intro: "Reach for this when you need to pull words out of a PDF and keep editing them in Word. It reads the selectable text already in your PDF and drops it into a simple .docx you can open, edit, and reflow. One honest caveat: it preserves the text, not the design, so the original columns, tables, spacing, and graphics will not carry over the way they look in the PDF. If your PDF is a scan or photo with no real text underneath, run OCR on it first, since there is nothing here to extract until then.",
    faqs: [
      { q: "Will the original layout, columns, and images come across?", a: "No. This preserves the text, not the original layout, so columns, tables, spacing, and graphics will not be recreated. You get the words in a simple Word document you can reformat yourself." },
      { q: "Why did I get an empty or near-empty document?", a: "That usually means your PDF is a scan or image with no selectable text underneath. This tool only extracts text that already exists, so run OCR on the file first, then convert it." },
      { q: "What file format does it produce?", a: "It produces a standard .docx Word document. The output is plain, editable text rather than a pixel-perfect copy of the page." },
      { q: "Are my files uploaded anywhere?", a: "No. The conversion runs entirely in your browser and your file never leaves your device. You can confirm this by opening your browser's Network tab and watching that nothing is sent while it works." },
    ],
  },
  "pdf-to-excel": {
    intro: "Reach for this when a PDF already holds tabular data as selectable text — an exported report, an invoice, a statement — and you want it back in a workbook you can sort and total. Your browser reads the text and its position on each page, then groups it into rows and columns to build an .xlsx file, all on your device. It is best-effort, not a true table-recognition engine, so dense or merged-cell layouts may land in the wrong columns and want a quick cleanup. If your PDF is a scan with no selectable text, run OCR on it first.",
    faqs: [
      { q: "Will it perfectly rebuild my original table?", a: "Not always. This is best-effort extraction, not a true table-recognition engine, so it groups text by its position into rows and columns but can misplace cells in dense, multi-column, or merged-cell layouts. Treat the result as a starting point and expect to tidy a few columns." },
      { q: "My PDF is a scan and the sheet comes out empty — why?", a: "A scanned PDF is just page images with no selectable text underneath, so there is nothing for this tool to pull into cells. Run our OCR tool on the file first to add a selectable text layer, then extract to Excel." },
      { q: "What does the output file look like?", a: "You get a single .xlsx workbook containing the text extracted from your PDF, arranged into rows and columns. It captures the text content, not the original styling, colors, or formulas, since the source PDF stores none of those." },
      { q: "Are my files uploaded anywhere?", a: "No. Your browser reads the PDF, extracts the text, and builds the Excel workbook entirely on your device — nothing is sent to a server. Open the Network tab and run it: you will see no file-upload request." },
    ],
  },
  "pdf-to-pptx": {
    intro: "Reach for this when you want a head start on a slide deck instead of retyping a PDF by hand. It pulls the selectable text out of your PDF and drops it into a plain .pptx, one slide per page, so you can rework the wording in PowerPoint or your slide editor of choice. It does not rebuild the original layout, fonts, columns, or images, so think of it as raw text to arrange rather than a finished presentation. If your PDF is scanned with no selectable text, there is nothing to pull and the slides will come out empty. The whole conversion happens in your browser.",
    faqs: [
      { q: "Does it recreate my PDF's original slide layout and design?", a: "No. It extracts the selectable text into a simple deck, one slide per page, and does not reproduce the original layout, fonts, columns, or graphics. You get the words to rearrange, not a styled copy of the original." },
      { q: "How are pages turned into slides?", a: "Each page of your PDF becomes one slide, in order. The text from that page lands on its slide as plain text for you to format afterward." },
      { q: "What happens if my PDF is scanned with no selectable text?", a: "You will get a deck with no usable text, because a scanned PDF is just page images with nothing to extract. Run it through an OCR step first if you need the text recognized." },
      { q: "Are my files uploaded anywhere?", a: "No. The conversion runs entirely in your browser and your file never leaves your device. Open the Network tab and watch as you convert — you will see no file-upload request." },
    ],
  },
  "ocr-pdf": {
    intro: "Reach for this when you have a scanned PDF you can't search or copy from, like a photographed contract or a faxed form. It renders each page, runs Tesseract OCR in your browser, and lays an invisible text layer over the picture so you can select, search, and copy. The original scan stays exactly as it looks; nothing is redrawn or cleaned up. You can raise the render scale for sharper recognition or set a minimum confidence to drop shaky guesses. The bundled model reads English only, so other languages won't come through.",
    faqs: [
      { q: "Does this change how my scanned pages look?", a: "No. The original scanned image stays exactly as-is; the recognized text is added as an invisible layer on top, so the page looks identical but you can now select, search, and copy from it." },
      { q: "What languages can it read?", a: "English only. The bundled Tesseract model is the English one, so pages in other languages or scripts won't be recognized accurately." },
      { q: "What do the render scale and minimum confidence options do?", a: "Render scale sets how large each page is rasterized before OCR, so a higher scale can sharpen recognition on small or faint text at the cost of more work. Minimum confidence drops recognized words that score below your threshold, which trims unreliable guesses from the text layer." },
      { q: "Are my files uploaded anywhere?", a: "No. The OCR runs entirely in your browser with Tesseract, and your file never leaves your device. You can confirm this in your browser's Network tab while it works." },
    ],
  },
  "pdfa": {
    intro: "PDF/A is the format archives, courts, and records systems ask for when a document needs to look the same years from now. This tool renders each page locally and rebuilds it as a PDF/A-2b file with the Rust krilla engine, so the visual layout is preserved as a self-contained archival copy. Because pages are rebuilt from a rendered image, the text becomes non-selectable in the result, so reach for this when long-term fidelity matters more than copying text. You can adjust the render scale to trade file size against sharpness, and the whole conversion happens in your browser.",
    faqs: [
      { q: "Will the text still be selectable in the PDF/A file?", a: "No. Because each page is rebuilt from a rendered image rather than from the original text layer, the result has no selectable or searchable text. Keep the original PDF if you need to copy or search the text later." },
      { q: "What does the render scale option do?", a: "Render scale controls the resolution at which pages are rendered before being rebuilt. A higher scale produces sharper pages and a larger file, while a lower scale keeps the file smaller at the cost of detail." },
      { q: "Which PDF/A version does this produce?", a: "This produces a PDF/A-2b file, the archival conformance level that focuses on visual reproduction. It is built with the Rust krilla engine from your rendered pages." },
      { q: "Are my files uploaded anywhere?", a: "No. The conversion runs entirely in your browser, so your file never leaves your device. You can confirm this by opening your browser's Network tab and watching that nothing is sent while it works." },
    ],
  },
  "image-convert": {
    intro: "Reach for this when you have an image in a format something won't accept, like a camera HEIC, a Photoshop PSD, or an old TIFF, BMP, GIF, or ICO, and you need a plain PNG, JPG, WebP, AVIF, or JPEG XL instead. It runs entirely in your browser, so the picture is decoded and re-encoded on your own machine and never gets sent anywhere. For the lossy formats you can set a quality level to trade file size against detail. You convert one image at a time.",
    faqs: [
      { q: "Which formats can it read and write?", a: "It reads common images plus TIFF, PSD, BMP, GIF, ICO, and HEIC, and writes PNG, JPG, WebP, AVIF, or JPEG XL. Pick the output format that the program or site you're feeding the image into actually accepts." },
      { q: "Can I control the quality or file size?", a: "Yes, for the lossy output formats you can set a quality level to balance file size against visible detail. Lower quality means a smaller file; higher quality keeps more detail at a larger size." },
      { q: "Can I convert several images at once?", a: "No, this tool converts one image at a time. To handle a batch, convert each image individually and download them one by one." },
      { q: "Are my files uploaded anywhere?", a: "No. The conversion happens entirely in your browser, so your image is decoded and re-encoded on your device and never leaves it. You can confirm this by opening your browser's Network tab and watching that no upload occurs." },
    ],
  },
  "html-to-pdf": {
    intro: "Reach for this when you have an .html, .md, or .txt file and just want a clean, readable PDF of its words. It pulls the text out of your file, then lays it onto pages with real, selectable type. Because it is text-focused and not a full browser layout engine, styling, images, links, and complex CSS do not carry over, so intricate web pages will not look the same. You set the document title, page size (A4 or Letter), and font size; the conversion happens in your browser, so the file never leaves your device.",
    faqs: [
      { q: "Will my CSS, images, and page layout be preserved?", a: "No. This is text-focused, not a browser layout engine, so it reads the text out of your HTML or Markdown and lays it onto clean pages; styling, images, links, and complex layouts are not carried over. If you need a pixel-faithful copy of a styled web page, this is not the right tool." },
      { q: "Is the text in the PDF selectable?", a: "Yes. The output is real PDF text laid out with standard font metrics, so you can select, copy, and search it. It is not a flat image of the page." },
      { q: "What options can I set?", a: "You can set the document title, the page size (A4 or Letter), and the font size. The title is normalized to plain ASCII, so any accented or non-Latin characters in it may be replaced with a question mark." },
      { q: "Are my files uploaded anywhere?", a: "No. The conversion runs entirely in your browser, and your file never leaves your device. You can confirm this by opening your browser's Network tab and watching that nothing is uploaded while it works." },
    ],
  },
  "page-numbers": {
    intro: "Reach for this when a PDF needs visible page numbers before you print, share, or file it. You write a format with placeholders, where {n} is the page number and {total} is the page count, then pick one of six positions and set the font size, margin, and which number to start counting from. A live preview shows exactly where each number lands so you can nudge things before committing. It stamps fresh text onto every page and saves a new PDF, all in your browser. It draws numbers over your pages rather than renumbering anything already printed inside the file.",
    faqs: [
      { q: "How do I show numbers like \"1 of 12\" instead of just a number?", a: "Type a format using the placeholders, such as \"{n} of {total}\", where {n} fills in each page's number and {total} fills in the page count. Anything else you type, like the word \"Page\" or dashes, is drawn literally, so \"Page {n}\" becomes \"Page 1\", \"Page 2\", and so on." },
      { q: "Can I start the numbering at a value other than 1?", a: "Yes, the \"Start at\" option sets the number for the first page, so the count begins there and increases by one per page. Set it to 0 or any other whole number when your document continues from somewhere else." },
      { q: "Does this change page numbers that are already printed in the document?", a: "No. It draws new number text on top of every page at the position you choose and does not detect, replace, or renumber any numbers already baked into the file, so existing printed numbers stay where they are." },
      { q: "Are my files uploaded anywhere?", a: "No. The numbering runs entirely in your browser and your file never leaves your device. You can open the Network tab before you start and confirm nothing is sent." },
    ],
  },
  "bates": {
    intro: "Bates numbering stamps a unique, sequential identifier on every page so a document set can be referenced exactly during discovery, depositions, or production. You set the prefix, the starting number, how many digits to pad to, and where the stamp sits, and each page gets the next number in order. Reach for it before you hand a PDF to opposing counsel or the court, when \"page 14 of the Smith deposition\" needs to become a citation anyone can find. The stamp is drawn onto each page as text, and the whole thing runs in your browser.",
    faqs: [
      { q: "Can I set the prefix, starting number, and padding?", a: "Yes. You choose the prefix text, the start number, how many digits to zero-pad to, and the corner or center position, so a start of 1 padded to six digits with prefix BATES- produces BATES-000001 and counts up from there." },
      { q: "Where does the number get placed on the page?", a: "The stamp goes in the position you pick: bottom-right, bottom-center, bottom-left, top-right, top-center, or top-left. You can also adjust the font size and the margin from the edge so it does not overlap your content." },
      { q: "Does it renumber the whole set continuously across the document?", a: "Yes, within the single PDF you load. It numbers every page sequentially from your start value to the last page; because it works on one file at a time, running a multi-file set means continuing the start number yourself on the next file." },
      { q: "Are my files uploaded anywhere?", a: "No. The numbering happens in your browser and your file never leaves your device, with nothing sent to a server. You can confirm this by opening your browser's Network tab and watching for no uploads while it runs." },
    ],
  },
  "watermark": {
    intro: "Reach for this when you want to mark a PDF as a draft, a confidential copy, or your own work before you share it. You type the text once and it gets stamped diagonally across the center of every page, so it carries through the whole document rather than sitting on page one alone. Set the font size, opacity, and angle, and watch the live preview update before you save. It draws the text as a layer over your existing pages, so what is already there stays intact, and the result downloads as a PDF.",
    faqs: [
      { q: "What can I change about the watermark?", a: "You can set the watermark text, its font size, its opacity, and its angle. A live preview shows how those choices look on the page before you save, so you can adjust the lettering until it reads the way you want." },
      { q: "Does it watermark every page or just the first one?", a: "It stamps the diagonal text across every page in the document. There is no per-page selection, so the same watermark appears on the first page and the last page alike." },
      { q: "Will the watermark cover up or remove my existing content?", a: "No. The text is drawn as a layer on top of your existing pages, so nothing underneath is replaced or flattened. Lowering the opacity lets the original content show through clearly beneath the lettering." },
      { q: "Are my files uploaded anywhere?", a: "No. The watermark is applied right here in your browser, and your file never leaves your device. You can confirm there is no upload by opening your browser's Network tab while the tool runs." },
    ],
  },
  "crop": {
    intro: "Reach for this when scanned or printed pages carry wide white borders, or when you want to focus a reading area before sharing a PDF. You set how much to trim from the top, right, bottom, and left in points, and the same crop box is applied to every page at once, with a visual preview to guide you. Because cropping only sets the crop box, the trimmed content is hidden rather than deleted, so another viewer could restore it. Everything happens in your browser, and you get a PDF back.",
    faqs: [
      { q: "Does cropping permanently remove the trimmed content?", a: "No. Cropping only sets the page crop box, so the hidden margins are concealed rather than deleted and could be restored by another tool or viewer. If you need the content gone for good, this is not the right tool." },
      { q: "Are the crop margins applied to every page or just one?", a: "The margins you set are applied to every page in the PDF at once. You enter the trim for the top, right, bottom, and left edges in points, and the same crop box is used throughout the document." },
      { q: "What units does the crop use?", a: "The crop uses points, the standard PDF unit, where 72 points equal one inch. A visual interface lets you see the result, and you enter the amount to trim from each of the four edges." },
      { q: "Are my files uploaded anywhere?", a: "No. The crop runs entirely in your browser and your file never leaves your device. You can confirm this by opening your browser's Network tab and watching that nothing is sent while you work." },
    ],
  },
  "auto-crop": {
    intro: "Reach for this when a PDF has wide white borders from scanning or printing and you want each page trimmed to its actual content. It renders every page locally, finds the bounds of the non-white pixels, and sets a per-page crop box with a little padding around the content, so columns of different widths each get their own tight crop. You control the scan scale, how much off-white still counts as background (white tolerance), and how much padding to keep. It only adjusts the visible crop region, so your real text and images stay intact underneath.",
    faqs: [
      { q: "Does cropping delete the trimmed margins or just hide them?", a: "It only sets each page's crop box, which changes the visible area without removing anything from the underlying page. The full original content stays in the file, so your text and images are untouched and a later tool can restore the wider view." },
      { q: "Why are some pages left at their original size?", a: "A page is only cropped when detectable content sits inside white margins and the new box is actually smaller than the page. Pages that are already tight, or that have content reaching every edge, are kept as they are." },
      { q: "What do the scan scale, white tolerance, and padding options do?", a: "Scan scale sets how finely each page is rendered before margins are measured, white tolerance decides how light a pixel can be and still count as background, and padding is the breathing room kept around the detected content. Raise tolerance if faint paper texture is being treated as content, and raise padding for a looser crop." },
      { q: "Are my files uploaded anywhere?", a: "No. Each page is rendered and measured in your browser, and only the crop boxes are written back on your device. Open the Network tab and watch while it runs; you will see no file-upload request." },
    ],
  },
  "metadata": {
    intro: "Open a PDF here to see what it already carries: its title, author, subject, keywords, and the creator app that made it. You'd reach for this when a document shows a stale filename or someone else's name in the title bar, or when you want clean, accurate descriptive fields before you share it. Type new values, leave the fields you don't care about alone, and download a PDF with the metadata overwritten. It edits the Info dictionary fields directly; it does not touch page content. To wipe everything for privacy instead of editing it, use the Remove metadata tool.",
    faqs: [
      { q: "Which metadata fields can I change?", a: "You can view and overwrite the title, author, subject, keywords, and creator fields in the PDF's Info dictionary. The page content itself is never altered, only these descriptive properties." },
      { q: "What happens to fields I leave blank?", a: "Fields you don't touch are left exactly as they were. Clearing a field's value instead removes that entry from the document, so blanking the title means the PDF ends up with no title rather than an empty one." },
      { q: "Does this delete all hidden metadata from my file?", a: "No, this tool edits the named fields you give it rather than scrubbing everything. If your goal is to strip out identifying information before sharing, use the separate Remove metadata tool, which clears the Info dictionary and the XMP stream." },
      { q: "Are my files uploaded anywhere?", a: "No. The PDF is read into memory and the metadata is rewritten on your device, right in the browser, so your file never leaves it. You can confirm this by opening your browser's Network tab and watching for no upload request." },
    ],
  },
  "sign": {
    intro: "Reach for Sign when you need to put your signature on a PDF without printing, signing, and scanning it back. Draw your signature once, then position and scale it onto the page where it belongs. It is an electronic mark — a drawn image placed on the document — not a cryptographic signature backed by a certificate, so it adds no tamper-evidence or identity proof. The whole process happens in your browser, and any signatures you save stay in this browser's local storage and are never sent anywhere. The result is a standard PDF.",
    faqs: [
      { q: "Is this a legally binding or cryptographic signature?", a: "No. This places an electronic mark — a drawn image — onto the page, which is not a cryptographic or digital signature with a certificate. It carries no tamper-evidence or identity proof, so whether it satisfies a given legal requirement is up to the recipient and your jurisdiction." },
      { q: "Can I move and resize the signature after I draw it?", a: "Yes. You draw the signature, then position it and scale its width on the page before placing it. Height follows the signature's aspect ratio automatically." },
      { q: "Where are my saved signatures stored?", a: "Saved signatures live only in this browser's local storage (IndexedDB) on your device and are never uploaded. They stay available for reuse on this browser but are not synced anywhere, so clearing your browser data removes them." },
      { q: "Are my files uploaded anywhere?", a: "No. The PDF is read into memory and signed entirely in your browser, on your device, with no upload. Open your browser's Network tab while you work and you will see no file leaving your device." },
    ],
  },
  "optimize": {
    intro: "Reach for Optimize when a PDF feels heavier than it should but you don't want to touch a single pixel. It rewrites the file losslessly, dropping unused objects left behind by editing and re-saving and compressing the data streams, while leaving every image exactly as it was. Because nothing is re-encoded, text stays selectable and the visual result is identical to the original. The savings depend on the source: bloated, repeatedly edited PDFs can drop noticeably, while files that are already tight will barely change. It all runs in your browser, so the document never leaves your device.",
    faqs: [
      { q: "Will this reduce the quality of my images or text?", a: "No. The shrink is fully lossless: images are left untouched and nothing is re-encoded, so the visual result is identical to the original and text stays selectable. It only removes unused objects and compresses the file's data streams." },
      { q: "Why did my file barely get smaller?", a: "Lossless optimization mainly helps bloated PDFs carrying unused objects and uncompressed streams, which often come from repeated editing and re-saving. A file that is already tight has little left to remove, so the reduction will be modest." },
      { q: "How is this different from the Compress tool?", a: "Optimize is lossless and never alters your images, so quality is identical but the savings are limited to structural cleanup. Compress uses Ghostscript to re-encode images for bigger reductions, which is lossy and can change image quality." },
      { q: "Are my files uploaded anywhere?", a: "No. Optimize runs entirely in your browser and your file never leaves your device. You can confirm it by opening your browser's Network tab and watching for any upload request while the tool works." },
    ],
  },
  "compress": {
    intro: "Reach for Compress when a PDF is bloated by photos, scans, or screenshots and you need a smaller file to email or upload elsewhere. It runs Ghostscript's pdfwrite locally in your browser, downsampling images with a lossy preset you pick: screen for the smallest result, then ebook, printer, and prepress for progressively higher quality. When Ghostscript can rewrite the file cleanly, your real text stays selectable. If a PDF resists that, it falls back to re-rendering each page as an image, which shrinks it but turns the text into flat pixels you can no longer select or search.",
    faqs: [
      { q: "Will the text stay selectable after compressing?", a: "Usually yes, because Ghostscript rewrites the file while keeping your real text intact. But if a PDF can't be rewritten cleanly and the raster fallback kicks in, each page is re-rendered as an image, so the text becomes flat pixels you can no longer select or search." },
      { q: "What do the screen, ebook, printer, and prepress presets do?", a: "They set how aggressively images are downsampled, trading file size against quality. Screen is the smallest and lowest quality; ebook and printer sit in the middle; prepress keeps the most detail and produces the largest file." },
      { q: "Is the compression lossy, and what files is it best for?", a: "Yes, this is lossy compression that discards image detail to shrink the file, so it works best on image-heavy or scanned PDFs. Text-only documents have little to squeeze and may not get much smaller." },
      { q: "Are my files uploaded anywhere?", a: "No. Ghostscript runs as WebAssembly right in your browser, so your file never leaves your device. Open the Network tab while you compress and you will see no file-upload request." },
    ],
  },
  "compress-image": {
    intro: "Reach for this when a photo or screenshot is too heavy for email, a web page, or a chat thread, and you want it smaller without handing it to a server. Pick WebP, AVIF, JPEG XL, or JPG to trade a little quality for a much smaller file, or stay on PNG when you need it lossless. Set a quality level and an optional max width or height, then re-encode right in the tab. Beyond ordinary images, it also reads TIFF, PSD, BMP, GIF, and ICO. Lossy formats discard detail to save space, so very low quality can soften edges.",
    faqs: [
      { q: "Which output format should I pick for the smallest file?", a: "WebP, AVIF, and JPEG XL are the lossy options and usually produce the smallest files at a given quality, while JPG is the widely compatible lossy choice. Pick PNG only when you need a lossless result, since it will not shrink the way the lossy formats do." },
      { q: "Will compressing reduce the image quality?", a: "With WebP, AVIF, JPEG XL, or JPG, yes, because they are lossy and drop some detail to save space. You control how much with the quality setting, and PNG stays lossless if you would rather not lose any detail." },
      { q: "Can it resize the image too, or only re-encode it?", a: "It can do both. Set a max width or height in pixels to scale the image down, or leave that at 0 to keep the original dimensions and only re-encode." },
      { q: "Are my files uploaded anywhere?", a: "No. The image is decoded and re-encoded by your browser on your own device, so it never leaves your machine. Open the Network tab while you run it and you will see no file-upload request." },
    ],
  },
  "fill-form": {
    intro: "Reach for this when a PDF already has interactive form fields and you want to type into them without printing, signing by hand, or scanning. It reads the document's AcroForm fields, so it handles text boxes, checkboxes, radio buttons, and dropdowns, and gives you a filled PDF back. One honest limit: it only works on PDFs that genuinely contain form fields. A flat scan of a paper form has no fields to detect, so there is nothing here to fill. Everything happens in your browser, on the file you opened.",
    faqs: [
      { q: "What kinds of form fields can it fill?", a: "It fills the standard AcroForm field types: text boxes, checkboxes, radio buttons, and dropdowns. You type or toggle each detected field, then download the filled PDF." },
      { q: "Why doesn't it work on my scanned form?", a: "A scanned or flattened PDF has no interactive form fields to detect, so there is nothing for this tool to fill. It only works on PDFs that actually contain AcroForm fields, not a flat image of a paper form." },
      { q: "Can the filled fields still be edited afterward?", a: "Yes, the output keeps the fields interactive, so the values you entered can be changed later in any PDF viewer. If you want them locked into the page so they can't be edited, use the Flatten tool afterward." },
      { q: "Are my files uploaded anywhere?", a: "No. The detection and filling run entirely in your browser, so your PDF never leaves your device. You can confirm it by opening your browser's Network tab and watching for no upload." },
    ],
  },
  "flatten": {
    intro: "Flatten bakes filled form fields and annotations into the page itself, so the values become fixed part of the content instead of editable widgets. Reach for it once a form is filled and you want to send a final copy that nobody can change and that renders the same in every viewer, including ones that handle interactive fields poorly. It works on the file you add and gives you back a PDF. After flattening, the fields are no longer editable, so keep your original if you might need to revise the answers later.",
    faqs: [
      { q: "Can the form still be edited after flattening?", a: "No. Flattening converts the fields and annotations into static page content, so the values can no longer be typed into or changed. Keep a copy of the original if you may need to edit it again." },
      { q: "Why would I flatten a form before sharing it?", a: "Flattening locks in the answers and makes the page render identically across viewers. Some readers display interactive fields inconsistently or let recipients alter them, and baking the values in avoids both problems." },
      { q: "What does it do to annotations and comments?", a: "Annotations are merged into the page along with the form fields, becoming part of the static content. They will show up the same everywhere but can no longer be moved, edited, or removed as separate objects." },
      { q: "Are my files uploaded anywhere?", a: "No. Flatten runs entirely in your browser, so your file never leaves your device. Open the Network tab while you run it and you will see no file-upload request." },
    ],
  },
  "remove-metadata": {
    intro: "PDFs carry a hidden layer most people never see: the title, author, producer, and the name of the app that created the file. Reach for this when you are sending a document to someone outside your circle and would rather not hand over who or what made it. It clears those metadata fields and gives you back a cleaned PDF, all in your browser, so the original never leaves your device. The pages themselves are untouched, so anything actually printed on them, including a name in the body text, stays exactly as it was.",
    faqs: [
      { q: "What exactly gets removed?", a: "It strips the document metadata fields such as title, author, subject, producer, and the creating application or tool. These are the hidden entries that quietly travel with the file and reveal who or what made it." },
      { q: "Does it change how the document looks?", a: "No. It only clears the metadata, so the visible page content stays exactly as it was. Text printed on the page, including a name written into the body, is not touched." },
      { q: "Does it remove text or images from inside the pages?", a: "No. It targets the document's metadata, not the page content, so visible names, words, and images remain. If sensitive information is printed on a page, you will need a tool that edits or redacts that content instead." },
      { q: "Are my files uploaded anywhere?", a: "No. The tool runs entirely in your browser and your file never leaves your device. Open the Network tab and watch as it works; you will see no file-upload request." },
    ],
  },
  "protect": {
    intro: "Reach for this when you want a PDF that asks for a password before it will open. You set an open password, and anyone without it sees only an encryption prompt. You can also add a separate owner password and pick which permissions to grant: printing, copying text and images, or editing and form-filling. Leave the owner password blank and it reuses your open password. The encryption runs in your browser, so the file and your passwords never leave your device. Keep the password somewhere safe; without it, the encrypted PDF cannot be reopened.",
    faqs: [
      { q: "What is the difference between the open password and the owner password?", a: "The open password is required to open and view the PDF at all, while the owner password controls what a reader can do once it is open, such as printing, copying, or editing. If you leave the owner password blank, it reuses your open password, so the two are the same." },
      { q: "Can someone really not print or copy if I deny those permissions?", a: "Denying printing, copying, or editing sets the permission flags inside the encrypted PDF, and conforming readers will honor them. These are PDF permission settings, not a hard guarantee, since some software can ignore the flags once a file is open, so treat them as a reasonable restriction rather than absolute protection." },
      { q: "What happens if I lose the password?", a: "There is no recovery if you lose the password, because the file is genuinely encrypted on your device and no copy or key is stored anywhere. Save the password somewhere safe before you rely on the protected PDF, since without it the file cannot be reopened." },
      { q: "Are my files uploaded anywhere?", a: "No. The encryption runs entirely in your browser, so the PDF and the passwords you type never leave your device. Open the Network tab and run the tool, and you will see no file-upload request." },
    ],
  },
  "unlock": {
    intro: "Reach for this when you have a PDF that asks for a password every time you open it and you want a plain copy you can read, search, and edit without retyping it. Enter the password the file already uses, and the encryption is stripped, leaving an unlocked version saved to your device. This is not a password cracker or recovery tool, so you do need to know the correct password first. The work happens in your browser, so the protected file and its password never travel anywhere.",
    faqs: [
      { q: "Can this open a PDF if I don't know the password?", a: "No. This removes encryption from a PDF you can already open, so you must enter the correct password yourself. It is not a password cracker or recovery tool and cannot guess or bypass a password you don't have." },
      { q: "What does the output look like?", a: "You get an unlocked copy of the same PDF with its password requirement removed. The content is unchanged; it simply no longer prompts for a password when opened." },
      { q: "Does it work with both user and owner passwords?", a: "Yes, you can unlock with either the password that's required to open the file or the one that restricts editing and printing. Whichever password you know and enter, the encryption is removed in the saved copy." },
      { q: "Are my files uploaded anywhere?", a: "No. The PDF and the password you type are processed entirely in your browser, so neither leaves your device. You can confirm this by watching the Network tab while you unlock a file." },
    ],
  },
  "sanitize": {
    intro: "Reach for this before you send a PDF outside your team, when you want the visible pages without the baggage that rides along with them. It strips author and tool metadata, document-level JavaScript, embedded files, and page actions, and you can also clear annotations and form fields with the two toggles. The work happens in your browser through the Rust core, so the file is never uploaded. One thing to know first: a password-protected PDF has to be unlocked before you sanitize it, since editing it while encrypted would corrupt the pages.",
    faqs: [
      { q: "What exactly does it remove from my PDF?", a: "It strips metadata, document-level JavaScript, embedded files, and page actions, and optionally annotations and form fields. The two toggles for annotations and form fields are on by default, so turn them off if you need to keep markups or fillable fields." },
      { q: "Why won't it sanitize my password-protected PDF?", a: "An encrypted PDF is rejected up front because editing it while still encrypted would corrupt the page content. Unlock the file first with its password, then sanitize the unlocked copy." },
      { q: "Will the page content and text still look the same?", a: "Yes. It removes hidden and active extras like scripts, embedded files, and metadata, but leaves the visible pages and their selectable text intact." },
      { q: "Are my files uploaded anywhere?", a: "No. The sanitizing runs entirely in your browser through the Rust core, so your file never leaves your device. You can confirm it yourself by watching the Network tab while the tool works." },
    ],
  },
  "redact": {
    intro: "Reach for this when you need to hand off a PDF with names, account numbers, or other sensitive details actually removed, not just hidden. You draw boxes over the parts you want gone, and instead of laying a black rectangle on top of live text, it rebuilds every affected page as an image-only page so the data underneath is genuinely deleted. Because those pages become images, their text is no longer selectable or searchable, and you cannot copy it back out. The whole pass runs in your browser, so the document you are redacting never leaves your device.",
    faqs: [
      { q: "Can someone recover the text I covered?", a: "No. The page under each box is rebuilt as an image-only page, so the original text and data are gone rather than just hidden beneath a rectangle. There is no live text layer left to select, copy, or pull back out." },
      { q: "Will the text still be selectable after redacting?", a: "No, not on the pages you redact. Those pages are flattened into images so the hidden content is truly removed, which means their text is no longer selectable or searchable. Pages you do not touch keep their original text." },
      { q: "Does it redact the whole document or just the pages I mark?", a: "Only the pages where you draw boxes are rebuilt as images. Pages you leave alone are kept as-is, so the rest of the document stays selectable and unchanged. The output is a single PDF." },
      { q: "Are my files uploaded anywhere?", a: "No. The redaction runs entirely in your browser and your file never leaves your device. Open the Network tab while you work and you will see no file-upload request." },
    ],
  },
};
