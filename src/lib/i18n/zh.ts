// Chinese (Simplified) — full translation bundle. Falls back to English for
// anything missing.
import type { LocaleBundle } from './types.js';

export const zh: LocaleBundle = {
  meta: { code: 'zh', htmlLang: 'zh-Hans', autonym: '中文', englishName: 'Chinese', dir: 'ltr' },
  categories: {
    organize: { label: '整理', descriptor: '重新排序、合并、拆分、旋转' },
    convert: { label: '转换', descriptor: '在 PDF、图片和文本之间转换' },
    edit: { label: '编辑', descriptor: '盖章、标注与签名' },
    optimize: { label: '优化', descriptor: '让文件更小' },
    forms: { label: '表单', descriptor: '检测并填写表单字段' },
    security: { label: '安全与隐私', descriptor: '清除你不想分享的内容' },
  },
  layout: {
    skip: '跳到主要内容',
    homeAria: 'Unfleece 首页',
    navAria: '主导航',
    allTools: '全部工具',
    howItWorks: '工作原理',
    privacy: '隐私',
    footerBlurb: '免费、私密的 PDF 工具。你的文件永远不会离开浏览器。',
    tools: '工具',
    project: '项目',
    trust: '信任',
    github: 'GitHub ↗',
    license: 'AGPL-3.0',
    reportIssue: '报告问题 ↗',
    noUpload: '不上传文件',
    noAccount: '无需账户',
    noTracking: '不追踪',
    language: '语言',
  },
  toolPage: {
    breadcrumbHome: '首页',
    breadcrumbTools: '工具',
    privateWorkspace: '私密工作区',
    nothingUploaded: '未上传文件',
    howEyebrow: '工作原理',
    howHeading: (toolName) => `如何使用「${toolName}」`,
    steps: [
      { title: '添加文件', body: '拖入或点击选择。文件始终留在你的设备上。' },
      { title: '设置选项', body: '调整设置，或直接使用合理的默认值。' },
      { title: '下载结果', body: '一键完成。它就在你的浏览器里生成。' },
    ],
    differenceEyebrow: '诚实的差别',
    differenceHeading: '为什么选 Unfleece，而非宰客软件（fleeceware）',
    usBullets: ['在浏览器中运行 — 不上传文件', '免费，无需账户，无需信用卡', '开源（AGPL-3.0）', '没有文件大小付费墙'],
    themHeading: '典型的宰客软件网站',
    themBullets: ['把你的文件上传到服务器', '$1 试用 → 每周约 $50', '封闭且不透明', '把结果锁在付费墙后'],
    questionsEyebrow: '问题',
    questionsHeading: '常见问题',
    relatedEyebrow: '继续探索',
    relatedHeading: '相关工具',
    faqs: (toolName) => [
      {
        q: `${toolName}真的免费吗？`,
        a: '是的 — 永久免费，无需账户，也没有暗藏陷阱的试用。没有文档处理服务器账单需要分摊。',
      },
      {
        q: '我的文件会被上传到什么地方吗？',
        a: `不会。你的浏览器把文件读入内存，并在你的设备上运行${toolName}。打开 Network（网络）面板看看吧 — 你不会看到文件上传请求。如果工具出错且已启用报告，最多只会出现不含内容的 /api/err beacon。`,
      },
      {
        q: '有文件大小或页数限制吗？',
        a: '没有硬性限制。唯一的上限是你浏览器的内存，所以超大文件可能会慢一些。在出问题之前我们会提醒你。',
      },
      {
        q: '可以离线使用吗？',
        a: '页面加载完成后就可以。你可以关掉 Wi-Fi，它依然能用 — 这是没有任何东西离开你设备的最有力证明。',
      },
    ],
  },
  home: {
    heroLine1: '免费、私密的 PDF 工具。',
    heroLine2: '你的文件永远不会离开浏览器。',
    subhead:
      '合并、拆分、转换、签名与压缩 — 每个工具都完全在你的设备上运行。永久免费，无需账户，不上传文件。开源，并且坦诚相待。',
    browseCta: '浏览工具',
    howCta: '工作原理',
    proof: [
      { title: '隐私为本', body: '文件在本地处理，从不上传。', linkLabel: '在 Network（网络）面板中验证。' },
      { title: '永久免费', body: '没有试用，没有订阅，没有文件大小付费墙。没有什么可向你兜售的。' },
      { title: '快速且开放', body: '通过 WebAssembly 在本地运行。完全开源（AGPL-3.0）。' },
    ],
    toolsEyebrow: '全部工具',
    pickHeading: '挑一个工具 — 打开它，运行它，下载结果。',
    pickBody: '无需注册。无需排队。一切都在这个标签页里完成。',
    proveEyebrow: '别信我们 — 自己验证',
    proveHeading: '10 秒就能用上，而且你能亲自证明它是私密的。',
    proveBody:
      '没有任何文件会碰到服务器，所以没什么可向你收费的，也没什么可泄露的。免费，只是理所当然的诚实之选。',
    proveSteps: [
      { strong: '打开任意工具', rest: '，按 F12 → Network（网络）面板。' },
      { strong: '对一个文件运行它。', rest: ' 盯着请求列表看。' },
      { strong: '看不到文件上传。', rest: ' 关掉 Wi-Fi — 它照样能用。' },
    ],
  },
  tools: {
    merge: {
      name: '合并 PDF',
      tagline: '把多个 PDF 合并成一个文档。',
      description: '按你添加的顺序，把多个 PDF 文件合并成单一文档。',
    },
    split: {
      name: '拆分 PDF',
      tagline: '把一个 PDF 拆分成多个文件。',
      description: '把一个 PDF 拆分成多个独立文件 — 每页一个，或按自定义页码范围拆分。',
    },
    'extract-pages': {
      name: '提取页面',
      tagline: '只取出你需要的那几页。',
      description: '生成一个只包含你所选页面的新 PDF。',
    },
    'remove-pages': {
      name: '删除页面',
      tagline: '删掉你不想要的页面。',
      description: '移除你选中的页面，保留其余部分。',
    },
    reorder: {
      name: '重排页面',
      tagline: '把页面调整成新的顺序。',
      description: '把 PDF 的页面重新排列成新的次序。',
    },
    rotate: {
      name: '旋转 PDF',
      tagline: '把页面转正。',
      description: '将全部或选定页面旋转 90°、180° 或 270°。无损。',
    },
    'n-up': {
      name: '每页多版（N-up）',
      tagline: '每张纸打印多页。',
      description: '把多个原始页面拼到每张 A4 纸上 — 很适合做讲义。',
    },
    booklet: {
      name: '小册子',
      tagline: '为折叠小册子打印进行拼版。',
      description: '重新排序并把页面以双联方式放到横向纸张上，对折后即成一本小册子。',
    },
    'compare-pdf': {
      name: '对比 PDF',
      tagline: '找出两个 PDF 之间的视觉差异。',
      description: '在本地渲染两个 PDF，并生成逐页的视觉差异报告。',
    },
    'images-to-pdf': {
      name: '图片 → PDF',
      tagline: '把 JPG 和 PNG 变成 PDF。',
      description: '把一张或多张 JPG/PNG 图片转换成 PDF，每张图片一页。',
    },
    'pdf-to-jpg': {
      name: 'PDF → JPG',
      tagline: '把每一页保存为 JPG。',
      description: '把每个 PDF 页面渲染成 JPG 图片，并打包成 ZIP 下载。',
    },
    'pdf-to-png': {
      name: 'PDF → PNG',
      tagline: '把每一页保存为 PNG。',
      description: '把每个 PDF 页面渲染成无损 PNG 图片，并打包成 ZIP 下载。',
    },
    'pdf-to-text': {
      name: 'PDF → 文本',
      tagline: '从 PDF 中提取可选中的文字。',
      description: '从 PDF 中提取可选中的文字。（扫描件/图片型 PDF 没有可提取的文字。）',
    },
    'pdf-to-epub': {
      name: 'PDF → EPUB',
      tagline: '用可选中的 PDF 文字生成 EPUB。',
      description: '用可选中的 PDF 文字构建可重排（reflowable）的 EPUB；对于扫描件和复杂版面，则用渲染后的页面生成固定版式的 EPUB。',
    },
    'pdf-to-docx': {
      name: '提取为 Word',
      tagline: '把可选中的 PDF 文字保存为 DOCX。',
      description: '把可选中的文字提取到一个简单的 Word 文档中。它保留文字，而非原始版面。',
    },
    'pdf-to-excel': {
      name: '提取为 Excel',
      tagline: '把可选中的行变成电子表格。',
      description: '尽力把可选中的文字行提取到 Excel 工作簿中。扫描件需要先做 OCR。',
    },
    'pdf-to-pptx': {
      name: '提取为 PowerPoint',
      tagline: '把 PDF 文字变成简单的幻灯片。',
      description: '把可选中的 PDF 文字提取成一个简单的 PowerPoint 演示文稿，每页一张幻灯片。它不会还原原始版面。',
    },
    'ocr-pdf': {
      name: 'OCR 可搜索 PDF',
      tagline: '为扫描版 PDF 添加可选中的文字。',
      description: '在本地运行 Tesseract OCR，在每一页上叠加一层不可见但可搜索的文字层。已内置英文模型。',
    },
    pdfa: {
      name: 'PDF/A 导出',
      tagline: '生成可长期归档的 PDF/A-2b 副本。',
      description: '在本地渲染页面，并用 Rust krilla 引擎将其重建为可视化的 PDF/A-2b 文件。文字将变得不可选中。',
    },
    'image-convert': {
      name: '转换图片',
      tagline: '在 PNG、JPG、WebP、AVIF、JPEG XL 以及 TIFF 类输入之间切换。',
      description: '把浏览器支持的图片，以及 TIFF、PSD、BMP、GIF、ICO 等格式，完全在你的浏览器里转换成 PNG、JPG、WebP、AVIF 或 JPEG XL。',
    },
    'html-to-pdf': {
      name: 'HTML / Markdown → PDF',
      tagline: '把文本文档变成干净的 PDF。',
      description: '把 HTML、Markdown 或纯文本文件转换成易读的 PDF。它专注于文字，并不是一个浏览器排版引擎。',
    },
    'page-numbers': {
      name: '页码',
      tagline: '在 PDF 上加盖页码。',
      description: '添加页码，格式和位置都可自定义。',
    },
    bates: {
      name: 'Bates 编号',
      tagline: '添加法律文书式的 Bates 编号。',
      description: '在每一页上加盖连续的 Bates 编号，可设置前缀、补零位数和位置。',
    },
    watermark: {
      name: '水印',
      tagline: '为每一页添加文字水印。',
      description: '在每一页上加盖斜向的文字水印。',
    },
    crop: {
      name: '裁剪',
      tagline: '裁掉页面的页边。',
      description: '通过设置裁剪框，裁去每一页的页边。',
    },
    'auto-crop': {
      name: '自动裁剪页边',
      tagline: '检测并裁去白色页边。',
      description: '在本地渲染每一页，检测非白色内容的边界，并为每页设置带留白的裁剪框。',
    },
    metadata: {
      name: '编辑元数据',
      tagline: '修改标题、作者等信息。',
      description: '查看并覆盖文档元数据（标题、作者、主题、关键词）。',
    },
    sign: {
      name: '签名',
      tagline: '手写签名并放到页面上。',
      description: '手写一个签名并放置到页面上。这是电子手写标记，并非加密数字签名。',
    },
    optimize: {
      name: '优化',
      tagline: '在不损失质量的前提下缩小文件。',
      description: '通过移除无用对象并压缩数据流来无损缩小 PDF — 文字依旧可选中。',
    },
    compress: {
      name: '压缩',
      tagline: '用 Ghostscript 压缩图片密集的 PDF。',
      description: '在本地使用 Ghostscript pdfwrite 进行有损 PDF 压缩。当 Ghostscript 能够重写文件时，文字仍可选中；而光栅化回退方案会让文字变得不可选中。',
    },
    'compress-image': {
      name: '压缩图片',
      tagline: '缩小 PNG、JPG、WebP、AVIF、JPEG XL 以及 TIFF 类输入。',
      description: '在本地重新编码并（可选）缩放浏览器图片，以及 TIFF、PSD、BMP、GIF、ICO。选择 WebP、AVIF、JPEG XL 或 JPG 可获得更小的有损输出。',
    },
    'fill-form': {
      name: '填写表单',
      tagline: '填写检测到的表单字段。',
      description: '检测 AcroForm 表单字段，并在你的浏览器中填写。',
    },
    flatten: {
      name: '扁平化',
      tagline: '把表单字段固化进页面。',
      description: '把表单字段和批注扁平化为静态的页面内容。',
    },
    'remove-metadata': {
      name: '移除元数据',
      tagline: '分享前清除隐藏信息。',
      description: '移除文档元数据（标题、作者、生成程序……），以免不小心被一并分享出去。',
    },
    protect: {
      name: '加密 PDF',
      tagline: '为打开 PDF 设置密码。',
      description: '在本地用用户密码加密 PDF，并可选设置所有者密码/权限。',
    },
    unlock: {
      name: '解锁 PDF',
      tagline: '在你知道密码时移除密码加密。',
      description: '用密码打开加密的 PDF，并在本地保存一份已解锁的副本。',
    },
    sanitize: {
      name: '净化 PDF',
      tagline: '清除 PDF 中的活动与隐藏附加内容。',
      description: '在分享前清除元数据、文档脚本、嵌入文件、页面动作，以及可选的批注/表单。',
    },
    redact: {
      name: '涂黑 PDF',
      tagline: '将涂黑遮盖永久烧录进文件。',
      description: '遮盖选定区域，并把页面重建为纯图片型 PDF 页面，确保下方不会残留隐藏文字。',
    },
  },
};

export default zh;
